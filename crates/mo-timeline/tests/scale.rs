use mo_common::*;
use mo_timeline::*;
fn t(n: i64, d: u32) -> RationalTime {
    RationalTime::new(n, d).unwrap()
}
fn exact(n: i64, d: i64) -> ExactValue {
    ExactValue {
        numerator: n.to_string(),
        denominator: d.to_string(),
    }
}
fn binding() -> PlaybackBinding {
    PlaybackBinding {
        session: PlaybackSessionId::new("scale").unwrap(),
        revision: Digest::from_sha256([3; 32]),
        generation: PlaybackGeneration::new(1),
    }
}
fn node(name: &str) -> TimingNode {
    TimingNode {
        restart: mo_timeline::RestartMode::Never,
        id: TimingNodeId::new(name).unwrap(),
        start: StartCondition::Single(TimeCondition::At { offset: t(0, 1) }),
        duration: t(1, 1),
        end_conditions: vec![],
        repeat_milli: 1000.into(),
        repeat_duration: None,
        fill: FillMode::Hold,
        time_transform: None,
        effect: Effect::Scale {
            target: ObjectId::new("shape").unwrap(),
            from: ScaleValue {
                x: 100000,
                y: 200000,
            },
            to: ScaleValue { x: 200000, y: 0 },
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
fn sample(timeline: &Timeline, at: RationalTime) -> EvaluatedFrame {
    TimelinePlan::compile(timeline, TimelineLimits::default(), &|| false)
        .unwrap()
        .evaluate(&binding(), at, None, &|| false)
        .unwrap()
}
#[test]
fn anisotropic_scale_is_exact_and_rotation_is_an_independent_channel() {
    let a = node("scale");
    let mut b = node("rotation");
    b.effect = Effect::Rotation {
        composition: Default::default(),
        target: a.target().clone(),
        from: 0,
        to: 60000,
    };
    let timeline = timeline(vec![a, b]);
    let f = sample(&timeline, t(1, 3));
    let target = ObjectId::new("shape").unwrap();
    assert_eq!(
        f.state.scales[&target],
        ExactScale {
            x: exact(400000, 3),
            y: exact(400000, 3)
        }
    );
    assert_eq!(f.state.rotations[&target].value(), exact(20000, 1));
    assert_eq!(
        sample(&timeline, t(2, 1)).state.scales[&target],
        ExactScale {
            x: exact(200000, 1),
            y: exact(0, 1)
        }
    );
    assert!(f.state.profile.contains("transform-frame"));
}
#[test]
fn replacement_precedence_and_remove_reveal_the_underlying_scale() {
    let a = node("first");
    let mut b = node("second");
    b.start = StartCondition::Single(TimeCondition::At { offset: t(1, 2) });
    b.duration = t(1, 2);
    b.fill = FillMode::Remove;
    b.effect = Effect::Scale {
        target: a.target().clone(),
        from: ScaleValue { x: 50000, y: 50000 },
        to: ScaleValue {
            x: 100000,
            y: 100000,
        },
    };
    let target = a.target().clone();
    let timeline = timeline(vec![a, b]);
    assert_eq!(
        sample(&timeline, t(3, 4)).state.scales[&target].x,
        exact(75000, 1)
    );
    assert_eq!(
        sample(&timeline, t(1, 1)).state.scales[&target].x,
        exact(200000, 1)
    );
}
#[test]
fn click_end_repeat_easing_and_reverse_apply_to_scale_without_clock_duplication() {
    let mut a = node("scale");
    a.start = StartCondition::Single(TimeCondition::Click {
        target: Some(a.target().clone()),
        delay: t(1, 4),
    });
    a.repeat_milli = 2500.into();
    a.end_conditions = vec![TimeCondition::After {
        node: a.id.clone(),
        event: NodeEvent::Begin,
        delay: t(1, 4),
    }];
    a.time_transform = Some(TimeTransform {
        speed_milli_percent: 100000,
        auto_reverse: true,
        acceleration_milli_percent: 25000,
        deceleration_milli_percent: 25000,
    });
    let target = a.target().clone();
    let plan =
        TimelinePlan::compile(&timeline(vec![a]), TimelineLimits::default(), &|| false).unwrap();
    let history = EventHistory {
        binding: binding(),
        through: t(10, 1),
        events: vec![PlaybackEvent {
            generation: binding().generation,
            sequence: 1,
            at: t(1, 1),
            event: InputEvent::Click {
                target: Some(target.clone()),
            },
        }],
    };
    let before = plan
        .evaluate(&binding(), t(1, 1), Some(&history), &|| false)
        .unwrap();
    assert!(before.state.scales.is_empty());
    let frozen = plan
        .evaluate(&binding(), t(9, 1), Some(&history), &|| false)
        .unwrap();
    assert_eq!(frozen.state.nodes[0].phase, NodePhase::Frozen);
    assert_eq!(frozen.state.scales[&target].x, exact(350000, 3));
    assert_eq!(frozen.state.scales[&target].y, exact(500000, 3));
}
#[test]
fn invalid_scale_is_rejected_during_admission_and_target_is_required() {
    for v in [MAX_SCALE_MILLI_PERCENT + 1, u32::MAX] {
        let mut n = node("bad");
        let Effect::Scale { ref mut from, .. } = n.effect else {
            unreachable!()
        };
        from.x = v;
        assert!(
            TimelinePlan::compile(&timeline(vec![n]), TimelineLimits::default(), &|| false)
                .is_err()
        );
    }
    let n = node("bounds");
    let target = n.target().clone();
    let plan =
        TimelinePlan::compile(&timeline(vec![n]), TimelineLimits::default(), &|| false).unwrap();
    assert!(plan.targets().contains(&target));
    assert!(matches!(
        plan.evaluate(&binding(), t(1, 2), None, &|| true),
        Err(TimelineError::Cancelled)
    ));
}
