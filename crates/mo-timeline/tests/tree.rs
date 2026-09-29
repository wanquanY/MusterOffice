use mo_common::*;
use mo_timeline::*;
use std::cell::Cell;
fn id(s: &str) -> TimingNodeId {
    TimingNodeId::new(s).unwrap()
}
fn time(n: i64, d: u32) -> RationalTime {
    RationalTime::new(n, d).unwrap()
}
fn at(n: i64, d: u32) -> StartCondition {
    StartCondition::Single(TimeCondition::At { offset: time(n, d) })
}
fn exact(n: i64, d: i64) -> ExactValue {
    ExactValue {
        numerator: n.to_string(),
        denominator: d.to_string(),
    }
}
fn leaf(name: &str, duration: i64, fill: FillMode) -> TimingNode {
    TimingNode {
        restart: mo_timeline::RestartMode::Never,
        id: id(name),
        start: at(0, 1),
        duration: time(duration, 1),
        end_conditions: vec![],
        repeat_milli: 1000.into(),
        repeat_duration: None,
        time_transform: None,
        fill,
        effect: Effect::Rotation {
            composition: Default::default(),
            target: ObjectId::new(name).unwrap(),
            from: 0,
            to: 120,
        },
    }
}
fn container(
    name: &str,
    kind: ContainerKind,
    duration: ContainerDuration,
    fill: FillMode,
    children: &[&str],
) -> TimingContainer {
    TimingContainer {
        time_transform: None,
        presentation: None,
        navigation: None,
        restart: mo_timeline::RestartMode::Never,
        id: id(name),
        kind,
        start: at(0, 1),
        end_conditions: vec![],
        duration,
        fill,
        children: children.iter().map(|s| id(s)).collect(),
    }
}
fn tree(nodes: Vec<TimingNode>, containers: Vec<TimingContainer>, roots: &[&str]) -> Timeline {
    Timeline {
        format: TimelineVersion::V02,
        nodes,
        tree: Some(TimingTree {
            roots: roots.iter().map(|s| id(s)).collect(),
            containers,
        }),
    }
}
fn binding() -> PlaybackBinding {
    PlaybackBinding {
        session: PlaybackSessionId::new("tree").unwrap(),
        revision: Digest::from_sha256([7; 32]),
        generation: PlaybackGeneration::new(3),
    }
}
fn sample(t: &Timeline, n: i64, d: u32, history: Option<&EventHistory>) -> EvaluatedFrame {
    TimelinePlan::compile(t, TimelineLimits::default(), &|| false)
        .unwrap()
        .evaluate(&binding(), time(n, d), history, &|| false)
        .unwrap()
}
fn rotation(f: &EvaluatedFrame, target: &str) -> Option<ExactValue> {
    f.state
        .rotations
        .get(&ObjectId::new(target).unwrap())
        .map(|r| {
            assert_eq!(r.basis, RotationBasis::Absolute);
            r.value()
        })
}
#[test]
fn nested_sequence_offsets_and_freeze_hold_have_distinct_lifetimes() {
    let mut a = leaf("a", 2, FillMode::Freeze);
    a.start = at(1, 1);
    let mut b = leaf("b", 2, FillMode::Remove);
    b.start = at(1, 2);
    let mut c = container(
        "seq",
        ContainerKind::Sequence,
        ContainerDuration::Automatic,
        FillMode::Freeze,
        &["a", "b"],
    );
    c.start = at(1, 1);
    let mut t = tree(vec![a, b], vec![c], &["seq"]);
    let f = sample(&t, 3, 1, None);
    assert_eq!(rotation(&f, "a"), Some(exact(60, 1)));
    assert_eq!(f.state.nodes[1].start, Some(exact(9, 2)));
    assert_eq!(f.state.containers[0].end, Some(exact(13, 2)));
    assert_eq!(rotation(&sample(&t, 17, 4, None), "a"), Some(exact(120, 1)));
    assert_eq!(rotation(&sample(&t, 9, 2, None), "a"), None);
    t.nodes[0].fill = FillMode::Hold;
    assert_eq!(rotation(&sample(&t, 20, 1, None), "a"), Some(exact(120, 1)));
    assert_eq!(rotation(&sample(&t, 20, 1, None), "b"), None);
    t.tree.as_mut().unwrap().containers[0].fill = FillMode::Remove;
    assert!(sample(&t, 20, 1, None).state.rotations.is_empty());
}
#[test]
fn ancestor_cutoff_freezes_active_descendants_but_respects_coincident_child_end() {
    let mut t = tree(
        vec![leaf("a", 10, FillMode::Remove)],
        vec![
            container(
                "outer",
                ContainerKind::Parallel,
                ContainerDuration::Fixed {
                    duration: time(3, 1),
                },
                FillMode::Freeze,
                &["inner"],
            ),
            container(
                "inner",
                ContainerKind::Parallel,
                ContainerDuration::Automatic,
                FillMode::Remove,
                &["a"],
            ),
        ],
        &["outer"],
    );
    let f = sample(&t, 100, 1, None);
    assert_eq!(rotation(&f, "a"), Some(exact(36, 1)));
    assert_eq!(f.state.nodes[0].phase, NodePhase::Frozen);
    assert_eq!(f.state.containers[1].phase, NodePhase::Frozen);
    t.nodes[0].duration = time(3, 1);
    assert_eq!(rotation(&sample(&t, 4, 1, None), "a"), None);
    t.nodes[0].duration = time(1, 1);
    assert_eq!(rotation(&sample(&t, 4, 1, None), "a"), None);
    t.nodes[0].start = at(4, 1);
    let f = sample(&t, 5, 1, None);
    assert_eq!(f.state.nodes[0].start, None);
    assert_eq!(f.state.nodes[0].phase, NodePhase::Suppressed);
}
#[test]
fn fractional_repeats_cutoff_uses_exact_last_boundary_and_backward_sampling() {
    let mut n = leaf("a", 2, FillMode::Remove);
    n.repeat_milli = 2500.into();
    let mut t = tree(
        vec![n],
        vec![container(
            "p",
            ContainerKind::Parallel,
            ContainerDuration::Fixed {
                duration: time(3, 1),
            },
            FillMode::Hold,
            &["a"],
        )],
        &["p"],
    );
    assert_eq!(rotation(&sample(&t, 9, 1, None), "a"), Some(exact(60, 1)));
    t.tree.as_mut().unwrap().containers[0].duration = ContainerDuration::Fixed {
        duration: time(2, 1),
    };
    assert_eq!(rotation(&sample(&t, 9, 1, None), "a"), Some(exact(120, 1)));
    assert_eq!(rotation(&sample(&t, 1, 3, None), "a"), Some(exact(20, 1)));
}
#[test]
fn clicks_wait_for_scope_and_do_not_reuse_the_activation_event() {
    let click = StartCondition::Single(TimeCondition::Click {
        target: None,
        delay: time(0, 1),
    });
    let mut a = leaf("a", 1, FillMode::Freeze);
    a.start = click.clone();
    let mut b = leaf("b", 1, FillMode::Freeze);
    b.start = click.clone();
    let mut c = container(
        "seq",
        ContainerKind::Sequence,
        ContainerDuration::Automatic,
        FillMode::Hold,
        &["a", "b"],
    );
    c.start = click;
    let t = tree(vec![a, b], vec![c], &["seq"]);
    let history = EventHistory {
        binding: binding(),
        through: time(10, 1),
        events: (1..=4)
            .map(|i| PlaybackEvent {
                generation: binding().generation,
                sequence: i,
                at: time(i64::from(i), 1),
                event: InputEvent::Click { target: None },
            })
            .collect(),
    };
    let f = sample(&t, 7, 2, Some(&history));
    assert_eq!(f.state.containers[0].start, Some(exact(1, 1)));
    assert_eq!(f.state.nodes[0].start, Some(exact(2, 1)));
    assert_eq!(f.state.nodes[1].start, Some(exact(3, 1)));
    assert_eq!(rotation(&f, "a"), None);
    assert_eq!(rotation(&f, "b"), Some(exact(60, 1)));
    let early = sample(&t, 3, 2, Some(&history));
    assert_eq!(early.state.nodes[0].phase, NodePhase::Waiting);
    assert_eq!(early.state.event_cursor, 1);
    let p = TimelinePlan::compile(&t, TimelineLimits::default(), &|| false).unwrap();
    assert!(matches!(
        p.evaluate(&binding(), time(3, 1), None, &|| false),
        Err(TimelineError::MissingEventHistory)
    ));
}
#[test]
fn tree_ownership_event_cycles_depth_and_cancellation_are_checked() {
    let base = tree(
        vec![leaf("a", 1, FillMode::Freeze)],
        vec![container(
            "p",
            ContainerKind::Parallel,
            ContainerDuration::Automatic,
            FillMode::Freeze,
            &["a"],
        )],
        &["p"],
    );
    let mut malformed = vec![];
    let mut t = base.clone();
    t.tree.as_mut().unwrap().roots.push(id("a"));
    malformed.push(t);
    let mut t = base.clone();
    t.tree.as_mut().unwrap().containers[0].children[0] = id("missing");
    malformed.push(t);
    let mut t = base.clone();
    t.nodes[0].start = StartCondition::Single(TimeCondition::After {
        node: id("p"),
        event: NodeEvent::End,
        delay: time(0, 1),
    });
    malformed.push(t);
    let mut t = base.clone();
    t.format = TimelineVersion::V01;
    malformed.push(t);
    for t in malformed {
        assert!(TimelinePlan::compile(&t, TimelineLimits::default(), &|| false).is_err());
    }
    assert!(matches!(
        TimelinePlan::compile(
            &base,
            TimelineLimits {
                max_depth: 1,
                ..Default::default()
            },
            &|| false
        ),
        Err(TimelineError::Limit(_))
    ));
    let calls = Cell::new(0);
    let p = TimelinePlan::compile(&base, TimelineLimits::default(), &|| {
        calls.set(calls.get() + 1);
        false
    })
    .unwrap();
    for stop in 0..calls.get() {
        let i = Cell::new(0);
        assert!(matches!(
            TimelinePlan::compile(&base, TimelineLimits::default(), &|| {
                let yes = i.get() == stop;
                i.set(i.get() + 1);
                yes
            }),
            Err(TimelineError::Cancelled)
        ));
    }
    assert!(matches!(
        p.evaluate(&binding(), time(1, 1), None, &|| true),
        Err(TimelineError::Cancelled)
    ));
}

#[test]
fn empty_timer_completion_carries_the_click_order_through_automatic_ancestors() {
    let click = StartCondition::Single(TimeCondition::Click {
        target: None,
        delay: time(0, 1),
    });
    let mut timer = container(
        "timer",
        ContainerKind::Parallel,
        ContainerDuration::Fixed {
            duration: time(0, 1),
        },
        FillMode::Remove,
        &[],
    );
    timer.start = click.clone();
    let mut leaf = leaf("later", 1, FillMode::Hold);
    leaf.start = click;
    let t = tree(
        vec![leaf],
        vec![
            container(
                "sequence",
                ContainerKind::Sequence,
                ContainerDuration::Automatic,
                FillMode::Hold,
                &["auto", "later"],
            ),
            container(
                "auto",
                ContainerKind::Parallel,
                ContainerDuration::Automatic,
                FillMode::Remove,
                &["timer"],
            ),
            timer,
        ],
        &["sequence"],
    );
    let mut history = EventHistory {
        binding: binding(),
        through: time(4, 1),
        events: vec![PlaybackEvent {
            generation: binding().generation,
            sequence: 1,
            at: time(1, 1),
            event: InputEvent::Click { target: None },
        }],
    };
    let f = sample(&t, 2, 1, Some(&history));
    assert_eq!(f.state.nodes[0].phase, NodePhase::Waiting);
    assert_eq!(f.state.containers[1].end, Some(exact(1, 1)));
    history.events.push(PlaybackEvent {
        generation: binding().generation,
        sequence: 2,
        at: time(1, 1),
        event: InputEvent::Click { target: None },
    });
    let f = sample(&t, 3, 2, Some(&history));
    assert_eq!(rotation(&f, "later"), Some(exact(60, 1)));
}
#[test]
fn dependency_events_before_scope_activation_are_not_replayed_at_the_same_timestamp() {
    let mut early = container(
        "early",
        ContainerKind::Parallel,
        ContainerDuration::Indefinite,
        FillMode::Hold,
        &[],
    );
    early.start = StartCondition::Single(TimeCondition::Click {
        target: Some(ObjectId::new("early").unwrap()),
        delay: time(0, 1),
    });
    let mut scope = container(
        "scope",
        ContainerKind::Parallel,
        ContainerDuration::Indefinite,
        FillMode::Hold,
        &["a"],
    );
    scope.start = StartCondition::Single(TimeCondition::Click {
        target: Some(ObjectId::new("late").unwrap()),
        delay: time(0, 1),
    });
    let mut a = leaf("a", 1, FillMode::Hold);
    a.start = StartCondition::Single(TimeCondition::After {
        node: id("early"),
        event: NodeEvent::Begin,
        delay: time(0, 1),
    });
    let t = tree(vec![a], vec![early, scope], &["early", "scope"]);
    let history = EventHistory {
        binding: binding(),
        through: time(4, 1),
        events: ["early", "late"]
            .iter()
            .enumerate()
            .map(|(i, target)| PlaybackEvent {
                generation: binding().generation,
                sequence: i as u32 + 1,
                at: time(1, 1),
                event: InputEvent::Click {
                    target: Some(ObjectId::new(*target).unwrap()),
                },
            })
            .collect(),
    };
    assert_eq!(
        sample(&t, 2, 1, Some(&history)).state.nodes[0].phase,
        NodePhase::Waiting
    );
}
#[test]
fn structural_order_controls_simultaneous_replacement_and_container_freeze_release() {
    let a = leaf("a", 1, FillMode::Hold);
    let mut b = leaf("b", 1, FillMode::Hold);
    b.effect = Effect::Rotation {
        composition: Default::default(),
        target: ObjectId::new("a").unwrap(),
        from: 120,
        to: 240,
    };
    let t = tree(vec![a.clone(), b], vec![], &["b", "a"]);
    // Explicit tree order, not the storage order of the leaf arena.
    assert_eq!(rotation(&sample(&t, 1, 2, None), "a"), Some(exact(60, 1)));
    let mut t = tree(
        vec![a],
        vec![
            container(
                "seq",
                ContainerKind::Sequence,
                ContainerDuration::Indefinite,
                FillMode::Hold,
                &["first", "timer"],
            ),
            container(
                "first",
                ContainerKind::Parallel,
                ContainerDuration::Automatic,
                FillMode::Freeze,
                &["a"],
            ),
            container(
                "timer",
                ContainerKind::Parallel,
                ContainerDuration::Indefinite,
                FillMode::Hold,
                &[],
            ),
        ],
        &["seq"],
    );
    assert_eq!(rotation(&sample(&t, 2, 1, None), "a"), None);
    t.tree.as_mut().unwrap().containers[1].fill = FillMode::Hold;
    assert_eq!(rotation(&sample(&t, 2, 1, None), "a"), Some(exact(120, 1)));
}
