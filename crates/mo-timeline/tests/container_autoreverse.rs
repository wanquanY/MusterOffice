use mo_common::*;
use mo_timeline::*;
use serde_json::json;

fn t(n: i64, d: u32) -> RationalTime {
    RationalTime::new(n, d).unwrap()
}
fn exact(n: i64, d: i64) -> ExactValue {
    ExactValue {
        numerator: n.to_string(),
        denominator: d.to_string(),
    }
}
fn id(name: &str) -> TimingNodeId {
    TimingNodeId::new(name).unwrap()
}
fn binding() -> PlaybackBinding {
    PlaybackBinding {
        session: PlaybackSessionId::new("container-autoreverse").unwrap(),
        revision: Digest::from_sha256([27; 32]),
        generation: PlaybackGeneration::new(1),
    }
}
fn model() -> Timeline {
    serde_json::from_value(json!({
        "format":"musteroffice.timeline/0.2-draft",
        "nodes":[
            {"id":"a","start":{"kind":"at","offset":{"ticks":"0","timescale":1}},"duration":{"ticks":"1","timescale":1},"repeatMilli":2000,"fill":"hold","effect":{"kind":"rotation","target":"a","from":0,"to":100}},
            {"id":"b","start":{"kind":"at","offset":{"ticks":"0","timescale":1}},"duration":{"ticks":"2","timescale":1},"repeatMilli":1000,"fill":"hold","effect":{"kind":"rotation","target":"b","from":0,"to":100}}
        ],
        "tree":{"roots":["group"],"containers":[{"id":"group","kind":"parallel","start":{"kind":"at","offset":{"ticks":"0","timescale":1}},"duration":{"kind":"automatic"},"fill":"hold","children":["a","b"],"timeTransform":{"speedMilliPercent":100000,"autoReverse":true,"accelerationMilliPercent":0,"decelerationMilliPercent":0}}]}
    })).unwrap()
}
fn sample(m: &Timeline, at: RationalTime, events: Option<&EventHistory>) -> FrameState {
    TimelinePlan::compile(m, TimelineLimits::default(), &|| false)
        .unwrap()
        .evaluate(&binding(), at, events, &|| false)
        .unwrap()
        .state
}
fn add_container(m: &mut Timeline, name: &str, auto: bool, speed: i32) {
    let tree = m.tree.as_mut().unwrap();
    let mut c = tree.containers[0].clone();
    c.id = id(name);
    c.children = tree.roots.clone();
    c.time_transform = Some(TimeTransform {
        speed_milli_percent: speed,
        auto_reverse: auto,
        ..Default::default()
    });
    tree.roots = vec![c.id.clone()];
    tree.containers.push(c);
}
#[test]
fn two_legs_have_distinct_child_activations_and_real_notifications() {
    let mut m = model();
    let mut listener = m.nodes[1].clone();
    listener.id = id("listener");
    listener.restart = RestartMode::Always;
    listener.start = TimeCondition::After {
        node: id("a"),
        event: NodeEvent::OnEnd,
        delay: t(1, 4),
    }
    .into();
    m.tree.as_mut().unwrap().roots.push(listener.id.clone());
    m.nodes.push(listener);
    for (at, start, a, b) in [
        (t(0, 1), 0, exact(0, 1), exact(0, 1)),
        (t(1, 1), 0, exact(0, 1), exact(1, 2)),
        (t(2, 1), 2, exact(1, 1), exact(1, 1)),
        (t(5, 2), 2, exact(1, 2), exact(3, 4)),
        (t(3, 1), 2, exact(0, 1), exact(1, 2)),
        (t(4, 1), 2, exact(0, 1), exact(0, 1)),
    ] {
        let f = sample(&m, at, None);
        assert_eq!(f.nodes[0].start, Some(exact(start, 1)));
        assert_eq!(f.nodes[0].end, Some(exact(start + 2, 1)));
        assert_eq!(f.nodes[0].progress, Some(a));
        assert_eq!(f.nodes[1].progress, Some(b));
        assert_eq!(f.containers[0].start, Some(exact(0, 1)));
        assert_eq!(f.containers[0].end, Some(exact(4, 1)));
    }
    assert_eq!(sample(&m, t(3, 1), None).nodes[2].start, Some(exact(9, 4)));
    assert_eq!(sample(&m, t(5, 1), None).nodes[2].start, Some(exact(17, 4)));
}
#[test]
fn nested_turns_and_negative_ancestors_preserve_each_physical_leg() {
    for outer_auto in [false, true] {
        for speed in [-100000, 100000] {
            let mut m = model();
            add_container(&mut m, "outer", outer_auto, speed);
            let end = if outer_auto { 8 } else { 4 };
            for at in 0..end {
                let f = sample(&m, t(at, 1), None);
                let start = at / 2 * 2;
                assert_eq!(
                    f.nodes[0].start,
                    Some(exact(start, 1)),
                    "{outer_auto} {speed} {at}"
                );
                assert_eq!(
                    f.nodes[1].progress,
                    Some(match at % 4 {
                        0 => exact(0, 1),
                        2 => exact(1, 1),
                        _ => exact(1, 2),
                    })
                );
                assert_eq!(f.containers[1].end, Some(exact(end, 1)));
            }
            assert_eq!(
                sample(&m, t(end, 1), None).nodes[1].progress,
                Some(exact(0, 1))
            );
        }
    }
}
#[test]
fn ancestor_cutoff_at_turn_keeps_forward_fill_and_cancels_reverse_activation() {
    for stop in [t(2, 1), t(3, 1)] {
        let mut m = model();
        add_container(&mut m, "clip", false, 100000);
        m.tree.as_mut().unwrap().containers[1].duration =
            ContainerDuration::Fixed { duration: stop };
        let f = sample(&m, t(8, 1), None);
        assert_eq!(f.nodes[0].phase, NodePhase::Frozen);
        if stop == t(2, 1) {
            assert_eq!(f.nodes[0].start, Some(exact(0, 1)));
            assert_eq!(f.nodes[0].progress, Some(exact(1, 1)));
            assert_eq!(f.nodes[1].progress, Some(exact(1, 1)));
        } else {
            assert_eq!(f.nodes[0].start, Some(exact(2, 1)));
            assert_eq!(f.nodes[0].progress, Some(exact(0, 1)));
            assert_eq!(f.nodes[1].progress, Some(exact(1, 2)));
        }
    }
    let mut m = model();
    m.tree.as_mut().unwrap().containers[0].end_conditions = vec![TimeCondition::After {
        node: id("a"),
        event: NodeEvent::OnEnd,
        delay: t(0, 1),
    }];
    let f = sample(&m, t(9, 1), None);
    assert_eq!(f.containers[0].end, Some(exact(2, 1)));
    assert_eq!(f.nodes[0].start, Some(exact(0, 1)));
    assert_eq!(f.nodes[0].progress, Some(exact(1, 1)));
}
#[test]
fn own_speed_easing_and_negative_speed_apply_to_both_legs() {
    for speed in [-200000, 200000] {
        let mut m = model();
        let c = &mut m.tree.as_mut().unwrap().containers[0];
        c.time_transform = Some(TimeTransform {
            speed_milli_percent: speed,
            auto_reverse: true,
            acceleration_milli_percent: 100000,
            ..Default::default()
        });
        for at in [t(1, 2), t(3, 2)] {
            let f = sample(&m, at, None);
            assert_eq!(f.nodes[1].progress, Some(exact(1, 4)));
            assert_eq!(f.containers[0].end, Some(exact(2, 1)));
        }
        assert_eq!(
            sample(&m, t(1, 1), None).nodes[1].progress,
            Some(exact(1, 1))
        );
    }
}
#[test]
fn restart_invalidates_old_turns_and_retained_sampling_preserves_earlier_legs() {
    let mut m = model();
    let c = &mut m.tree.as_mut().unwrap().containers[0];
    c.restart = RestartMode::Always;
    c.start = TimeCondition::Click {
        target: None,
        delay: t(0, 1),
    }
    .into();
    let history = EventHistory {
        binding: binding(),
        through: t(10, 1),
        events: [1, 2]
            .into_iter()
            .enumerate()
            .map(|(i, at)| PlaybackEvent {
                generation: binding().generation,
                sequence: i as u32 + 1,
                at: t(at, 1),
                event: InputEvent::Click { target: None },
            })
            .collect(),
    };
    let plan = TimelinePlan::compile(&m, TimelineLimits::default(), &|| false).unwrap();
    let mut retained = TimelineSampler::new(plan.clone());
    for (at, start, p) in [
        (t(5, 1), 4, exact(1, 2)),
        (t(3, 1), 2, exact(1, 2)),
        (t(3, 2), 1, exact(1, 4)),
        (t(4, 1), 4, exact(1, 1)),
    ] {
        let f = retained
            .evaluate(&binding(), at, Some(&history), &|| false)
            .unwrap()
            .state;
        assert_eq!(f.nodes[1].start, Some(exact(start, 1)));
        assert_eq!(f.nodes[1].progress, Some(p));
        assert_eq!(
            f,
            plan.evaluate(&binding(), at, Some(&history), &|| false)
                .unwrap()
                .state
        );
    }
}
#[test]
fn next_seeks_both_legs_and_previous_replays_from_a_clean_scope() {
    let mut m = model();
    add_container(&mut m, "sequence", false, 100000);
    let c = &mut m.tree.as_mut().unwrap().containers[1];
    c.kind = ContainerKind::Sequence;
    c.duration = ContainerDuration::Indefinite;
    c.navigation = Some(SequenceNavigation {
        concurrent: false,
        next_action: NextAction::Seek,
        previous_action: PreviousAction::None,
        next_conditions: vec![TimeCondition::Navigation {
            direction: NavigationDirection::Next,
            target: None,
            delay: t(0, 1),
        }],
        previous_conditions: vec![TimeCondition::Navigation {
            direction: NavigationDirection::Previous,
            target: None,
            delay: t(0, 1),
        }],
    });
    let history = EventHistory {
        binding: binding(),
        through: t(10, 1),
        events: [
            (t(1, 2), NavigationDirection::Next),
            (t(3, 4), NavigationDirection::Previous),
        ]
        .into_iter()
        .enumerate()
        .map(|(i, (at, direction))| PlaybackEvent {
            generation: binding().generation,
            sequence: i as u32 + 1,
            at,
            event: InputEvent::Navigation {
                direction,
                target: None,
            },
        })
        .collect(),
    };
    let f = sample(&m, t(1, 2), Some(&history));
    assert_eq!(f.nodes[1].end, Some(exact(1, 2)));
    assert_eq!(f.nodes[1].progress, Some(exact(0, 1)));
    assert_eq!(f.containers[0].end, Some(exact(1, 2)));
    let f = sample(&m, t(3, 4), Some(&history));
    assert_eq!(f.nodes[1].start, Some(exact(3, 4)));
    assert_eq!(f.nodes[1].progress, Some(exact(0, 1)));
    assert_eq!(
        sample(&m, t(11, 4), Some(&history)).nodes[1].progress,
        Some(exact(1, 1))
    );
}
#[test]
fn descendant_activations_remain_bounded_and_cancelled_sampling_does_not_commit() {
    let mut m = model();
    m.tree.as_mut().unwrap().containers[0].restart = RestartMode::Always;
    let limits = TimelineLimits {
        max_intervals: 4,
        ..Default::default()
    };
    let plan = TimelinePlan::compile(&m, limits, &|| false).unwrap();
    let mut retained = TimelineSampler::new(plan);
    let before = retained
        .evaluate(&binding(), t(1, 1), None, &|| false)
        .unwrap();
    assert!(matches!(
        retained.evaluate(&binding(), t(2, 1), None, &|| false),
        Err(TimelineError::Limit(_))
    ));
    assert!(matches!(
        retained.evaluate(&binding(), t(1, 1), None, &|| true),
        Err(TimelineError::Cancelled)
    ));
    assert_eq!(
        retained
            .evaluate(&binding(), t(1, 1), None, &|| false)
            .unwrap(),
        before
    );
}
#[test]
fn turning_under_nonlinear_ancestor_is_not_scheduled_by_linear_approximation() {
    let mut m = model();
    add_container(&mut m, "easing", false, 100000);
    m.tree.as_mut().unwrap().containers[1]
        .time_transform
        .as_mut()
        .unwrap()
        .acceleration_milli_percent = 100000;
    assert!(TimelinePlan::compile(&m, TimelineLimits::default(), &|| false).is_err());
    let mut m = model();
    m.nodes[0].start = TimeCondition::At { offset: t(1, 1) }.into();
    assert!(TimelinePlan::compile(&m, TimelineLimits::default(), &|| false).is_err());
}
