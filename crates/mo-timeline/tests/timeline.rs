use mo_common::*;
use mo_timeline::*;
use std::cell::Cell;
fn t(n: i64, d: u32) -> RationalTime {
    RationalTime::new(n, d).unwrap()
}
fn id(s: &str) -> TimingNodeId {
    TimingNodeId::new(s).unwrap()
}
fn object() -> ObjectId {
    ObjectId::new("shape").unwrap()
}
fn binding() -> PlaybackBinding {
    PlaybackBinding {
        session: PlaybackSessionId::new("session").unwrap(),
        revision: Digest::from_sha256([3; 32]),
        generation: PlaybackGeneration::new(8),
    }
}
fn node(s: &str) -> TimingNode {
    TimingNode {
        restart: mo_timeline::RestartMode::Never,
        id: id(s),
        start: StartCondition::Single(TimeCondition::At { offset: t(0, 1) }),
        duration: t(1, 1),
        end_conditions: vec![],
        repeat_milli: 1000.into(),
        repeat_duration: None,
        time_transform: None,
        fill: FillMode::Freeze,
        effect: Effect::Rotation {
            composition: Default::default(),
            target: object(),
            from: -21600000,
            to: 43200000,
        },
    }
}
fn timeline(nodes: Vec<TimingNode>) -> Timeline {
    Timeline {
        format: TimelineVersion::V01,
        tree: None,
        nodes,
    }
}
fn plan(nodes: Vec<TimingNode>) -> TimelinePlan {
    TimelinePlan::compile(&timeline(nodes), TimelineLimits::default(), &|| false).unwrap()
}
fn ratio(n: &str, d: &str) -> ExactValue {
    ExactValue {
        numerator: n.into(),
        denominator: d.into(),
    }
}
fn sample(p: &TimelinePlan, at: RationalTime) -> EvaluatedFrame {
    p.evaluate(&binding(), at, None, &|| false).unwrap()
}
#[test]
fn rational_sample_is_exact_and_multiturn_is_not_wrapped() {
    let p = plan(vec![node("a")]);
    let f = sample(&p, t(1001, 30000));
    assert_eq!(f.state.nodes[0].progress, Some(ratio("1001", "30000")));
    assert_eq!(
        f.state.rotations[&object()].value(),
        ratio("-19437840", "1")
    );
    assert_eq!(
        sample(&p, t(1, 1)).state.rotations[&object()].value(),
        ratio("43200000", "1")
    );
    assert_eq!(
        sample(&p, t(100, 1)).state.rotations[&object()].value(),
        ratio("43200000", "1")
    );
    assert_eq!(sample(&p, t(1001, 30000)), f); // seek backwards through immutable plan
    assert_eq!(sample(&p, t(2002, 60000)), f);
}
#[test]
fn repeat_boundaries_distinguish_active_cycle_and_frozen_endpoint() {
    let mut a = node("a");
    a.repeat_milli = 2500.into();
    let p = plan(vec![a.clone()]);
    let f = sample(&p, t(2, 1));
    assert_eq!(f.state.nodes[0].iteration.as_deref(), Some("2"));
    assert_eq!(f.state.nodes[0].progress, Some(ratio("0", "1")));
    let f = sample(&p, t(5, 2));
    assert_eq!(f.state.nodes[0].phase, NodePhase::Frozen);
    assert_eq!(f.state.nodes[0].progress, Some(ratio("1", "2")));
    a.repeat_milli = 2000.into();
    let f = sample(&plan(vec![a.clone()]), t(2, 1));
    assert_eq!(f.state.nodes[0].iteration.as_deref(), Some("1"));
    assert_eq!(f.state.nodes[0].progress, Some(ratio("1", "1")));
    a.fill = FillMode::Remove;
    let f = sample(&plan(vec![a]), t(2, 1));
    assert!(f.state.rotations.is_empty());
    assert_eq!(f.state.nodes[0].phase, NodePhase::Finished);
}
#[test]
fn dependencies_are_exact_and_author_order_breaks_activation_ties() {
    let a = node("a");
    let mut b = node("b");
    b.start = StartCondition::Single(TimeCondition::After {
        node: id("a"),
        event: NodeEvent::Begin,
        delay: t(0, 1),
    });
    b.effect = Effect::Rotation {
        composition: Default::default(),
        target: object(),
        from: 0,
        to: 100,
    };
    // Topological computation visits a then b; output/order must remain b then a.
    let f = sample(&plan(vec![b.clone(), a.clone()]), t(1, 2));
    assert_eq!(f.state.nodes[0].node, id("b"));
    assert_eq!(f.state.rotations[&object()].value(), ratio("10800000", "1"));
    b.start = StartCondition::Single(TimeCondition::After {
        node: id("a"),
        event: NodeEvent::End,
        delay: t(1, 3),
    });
    let f = sample(&plan(vec![b, a]), t(3, 2));
    assert_eq!(f.state.nodes[0].start, Some(ratio("4", "3")));
    assert_eq!(f.state.rotations[&object()].value(), ratio("50", "3"));
}
fn click(at: RationalTime, sequence: u32) -> PlaybackEvent {
    PlaybackEvent {
        generation: binding().generation,
        sequence,
        at,
        event: InputEvent::Click { target: None },
    }
}
fn interactive() -> TimelinePlan {
    let mut a = node("a");
    a.start = StartCondition::Single(TimeCondition::Click {
        target: None,
        delay: t(1, 4),
    });
    let mut b = node("b");
    b.start = StartCondition::Single(TimeCondition::After {
        node: id("a"),
        event: NodeEvent::End,
        delay: t(0, 1),
    });
    plan(vec![a, b])
}
#[test]
fn seek_requires_explicit_event_coverage_and_never_reads_future_clicks() {
    let p = interactive();
    assert!(matches!(
        p.evaluate(&binding(), t(0, 1), None, &|| false),
        Err(TimelineError::MissingEventHistory)
    ));
    let mut h = EventHistory {
        binding: binding(),
        through: t(5, 1),
        events: vec![click(t(2, 1), 1)],
    };
    let before = p
        .evaluate(&binding(), t(1, 1), Some(&h), &|| false)
        .unwrap();
    assert!(
        before
            .state
            .nodes
            .iter()
            .all(|f| f.phase == NodePhase::Waiting)
    );
    assert_eq!(before.state.event_cursor, 0);
    let after = p
        .evaluate(&binding(), t(3, 1), Some(&h), &|| false)
        .unwrap();
    assert_eq!(after.state.nodes[0].start, Some(ratio("9", "4")));
    assert_eq!(after.state.nodes[0].progress, Some(ratio("3", "4")));
    assert_eq!(
        p.evaluate(&binding(), t(1, 1), Some(&h), &|| false)
            .unwrap(),
        before
    );
    h.events.push(h.events[0].clone());
    assert_eq!(
        p.evaluate(&binding(), t(3, 1), Some(&h), &|| false)
            .unwrap(),
        after
    );
    h.events.clear();
    assert!(
        p.evaluate(&binding(), t(3, 1), Some(&h), &|| false)
            .unwrap()
            .state
            .rotations
            .is_empty()
    );
    h.through = t(2, 1);
    assert!(matches!(
        p.evaluate(&binding(), t(3, 1), Some(&h), &|| false),
        Err(TimelineError::MissingEventHistory)
    ));
}
#[test]
fn conflicting_stale_or_incomplete_logs_are_rejected() {
    let p = interactive();
    let base = EventHistory {
        binding: binding(),
        through: t(10, 1),
        events: vec![click(t(1, 1), 1)],
    };
    let mut variants = vec![];
    let mut h = base.clone();
    h.binding.generation = PlaybackGeneration::new(9);
    variants.push(h);
    let mut h = base.clone();
    h.events[0].generation = PlaybackGeneration::new(9);
    variants.push(h);
    let mut h = base.clone();
    h.events[0].sequence = 2;
    variants.push(h);
    let mut h = base.clone();
    h.events.push(click(t(2, 1), 1));
    variants.push(h);
    let mut h = base.clone();
    h.events.push(click(t(0, 1), 2));
    variants.push(h);
    let mut h = base.clone();
    h.events.push(click(t(11, 1), 2));
    variants.push(h);
    let mut h = base.clone();
    h.events.push(click(t(-1, 1), 2));
    variants.push(h);
    let mut h = base.clone();
    h.events.push(click(t(2, 1), 3));
    variants.push(h);
    for h in variants {
        assert!(matches!(
            p.evaluate(&binding(), t(3, 1), Some(&h), &|| false),
            Err(TimelineError::EventHistory(_))
        ));
    }
}
#[test]
fn only_matching_target_triggers_a_shape_click() {
    let mut n = node("a");
    n.start = StartCondition::Single(TimeCondition::Click {
        target: Some(object()),
        delay: t(0, 1),
    });
    let p = plan(vec![n]);
    let mut h = EventHistory {
        binding: binding(),
        through: t(2, 1),
        events: vec![click(t(0, 1), 1)],
    };
    assert_eq!(
        p.evaluate(&binding(), t(1, 1), Some(&h), &|| false)
            .unwrap()
            .state
            .nodes[0]
            .phase,
        NodePhase::Waiting
    );
    h.events.push(PlaybackEvent {
        event: InputEvent::Click {
            target: Some(object()),
        },
        ..click(t(1, 1), 2)
    });
    assert_eq!(
        p.evaluate(&binding(), t(1, 1), Some(&h), &|| false)
            .unwrap()
            .state
            .nodes[0]
            .phase,
        NodePhase::Active
    );
}
#[test]
fn graph_and_resource_limits_fail_before_a_frame_is_published() {
    let mut a = node("a");
    a.start = StartCondition::Single(TimeCondition::After {
        node: id("b"),
        event: NodeEvent::End,
        delay: t(0, 1),
    });
    assert!(
        TimelinePlan::compile(
            &timeline(vec![a.clone()]),
            TimelineLimits::default(),
            &|| false
        )
        .is_err()
    );
    let mut b = node("b");
    b.start = StartCondition::Single(TimeCondition::After {
        node: id("a"),
        event: NodeEvent::Begin,
        delay: t(0, 1),
    });
    assert!(
        TimelinePlan::compile(&timeline(vec![a, b]), TimelineLimits::default(), &|| false).is_err()
    );
    assert!(
        TimelinePlan::compile(
            &timeline(vec![node("a"), node("a")]),
            TimelineLimits::default(),
            &|| false
        )
        .is_err()
    );
    assert!(matches!(
        TimelinePlan::compile(
            &timeline(vec![node("a")]),
            TimelineLimits {
                max_nodes: 0,
                ..Default::default()
            },
            &|| false
        ),
        Err(TimelineError::Limit(_))
    ));
    for (duration, repeat, offset) in [
        (t(0, 1), 1000, t(0, 1)),
        (t(-1, 1), 1000, t(0, 1)),
        (t(1, 1), 0, t(0, 1)),
        (t(1, 1), 1000, t(-1, 1)),
    ] {
        let mut n = node("a");
        n.duration = duration;
        n.repeat_milli = repeat.into();
        n.start = StartCondition::Single(TimeCondition::At { offset });
        assert!(
            TimelinePlan::compile(&timeline(vec![n]), TimelineLimits::default(), &|| false)
                .is_err()
        );
    }
    assert!(sample(&plan(vec![]), t(0, 1)).state.nodes.is_empty());
    assert!(
        plan(vec![node("a")])
            .evaluate(&binding(), t(-1, 1), None, &|| false)
            .is_err()
    );
}
#[test]
fn denominator_growth_has_a_hard_exact_arithmetic_budget() {
    let nodes = [
        999999937, 999999929, 999999893, 999999883, 999999797, 999999761,
    ]
    .into_iter()
    .enumerate()
    .map(|(i, d)| {
        let mut n = node(&format!("n{i}"));
        n.duration = t(1, d);
        if i > 0 {
            n.start = StartCondition::Single(TimeCondition::After {
                node: id(&format!("n{}", i - 1)),
                event: NodeEvent::End,
                delay: t(0, 1),
            })
        }
        n
    })
    .collect();
    let p = TimelinePlan::compile(
        &timeline(nodes),
        TimelineLimits {
            max_exact_bits: 128,
            ..Default::default()
        },
        &|| false,
    )
    .unwrap();
    assert!(matches!(
        p.evaluate(&binding(), t(1, 1), None, &|| false),
        Err(TimelineError::Limit(_))
    ));
}
#[test]
fn cancellation_at_every_checkpoint_is_atomic() {
    let timeline = timeline(vec![node("a"), node("b")]);
    let n = Cell::new(0);
    TimelinePlan::compile(&timeline, TimelineLimits::default(), &|| {
        n.set(n.get() + 1);
        false
    })
    .unwrap();
    for stop in 0..n.get() {
        let i = Cell::new(0);
        assert!(matches!(
            TimelinePlan::compile(&timeline, TimelineLimits::default(), &|| {
                let cancel = i.get() == stop;
                i.set(i.get() + 1);
                cancel
            }),
            Err(TimelineError::Cancelled)
        ));
    }
    let p = interactive();
    let h = EventHistory {
        binding: binding(),
        through: t(2, 1),
        events: vec![click(t(0, 1), 1)],
    };
    n.set(0);
    let expected = p
        .evaluate(&binding(), t(1, 1), Some(&h), &|| {
            n.set(n.get() + 1);
            false
        })
        .unwrap();
    for stop in 0..n.get() {
        let i = Cell::new(0);
        assert!(matches!(
            p.evaluate(&binding(), t(1, 1), Some(&h), &|| {
                let cancel = i.get() == stop;
                i.set(i.get() + 1);
                cancel
            }),
            Err(TimelineError::Cancelled)
        ));
    }
    assert_eq!(
        p.evaluate(&binding(), t(1, 1), Some(&h), &|| false)
            .unwrap(),
        expected
    );
}
#[test]
fn generations_never_wrap_and_reject_noncanonical_wire_values() {
    assert!(PlaybackGeneration::new(u64::MAX).next().is_none());
    for value in [
        "\"01\"",
        "\"+1\"",
        "\"-1\"",
        "\"18446744073709551616\"",
        "1",
    ] {
        assert!(serde_json::from_str::<PlaybackGeneration>(value).is_err());
    }
    assert_eq!(
        serde_json::to_string(&PlaybackGeneration::new(u64::MAX)).unwrap(),
        "\"18446744073709551615\""
    );
}
