use mo_common::*;
use mo_timeline::*;
fn t(ms: i64) -> RationalTime {
    RationalTime::new(ms, 1000).unwrap()
}
fn p(x: &str, y: &str) -> MotionPoint {
    MotionPoint {
        x: x.to_owned().try_into().unwrap(),
        y: y.to_owned().try_into().unwrap(),
    }
}
fn binding() -> PlaybackBinding {
    PlaybackBinding {
        session: PlaybackSessionId::new("motion").unwrap(),
        revision: Digest::from_sha256([3; 32]),
        generation: PlaybackGeneration::new(1),
    }
}
fn timeline() -> Timeline {
    Timeline {
        format: TimelineVersion::V01,
        tree: None,
        nodes: vec![TimingNode {
            id: TimingNodeId::new("line").unwrap(),
            restart: RestartMode::Never,
            start: TimeCondition::At { offset: t(100) }.into(),
            duration: t(900),
            repeat_milli: 1000.into(),
            repeat_duration: None,
            end_conditions: vec![],
            fill: FillMode::Hold,
            time_transform: None,
            effect: Effect::MotionLine {
                target: ObjectId::new("shape").unwrap(),
                from: p("0.1", "-0.000000000000000001"),
                to: p("0.3", "0.3"),
            },
        }],
    }
}
fn sample(plan: &TimelinePlan, ms: i64) -> FrameState {
    plan.evaluate(&binding(), t(ms), None, &|| false)
        .unwrap()
        .state
}
#[test]
fn exact_slide_offsets_seek_repeat_reverse_and_remove_without_accumulation() {
    let mut q = timeline();
    let plan = TimelinePlan::compile(&q, TimelineLimits::default(), &|| false).unwrap();
    assert!(sample(&plan, 99).motion.is_empty());
    let mid = sample(&plan, 400);
    assert_eq!(mid.profile, MOTION_FRAME_PROFILE);
    let v = mid.motion.values().next().unwrap();
    assert_eq!(
        v.x,
        ExactValue {
            numerator: "1".into(),
            denominator: "6".into()
        }
    );
    assert_eq!(
        v.y,
        ExactValue {
            numerator: "149999999999999999".into(),
            denominator: "1500000000000000000".into()
        }
    );
    assert_eq!(sample(&plan, 10000).motion, sample(&plan, 1000).motion);
    assert_eq!(sample(&plan, 400).motion, mid.motion);
    q.nodes[0].time_transform = Some(TimeTransform {
        auto_reverse: true,
        ..Default::default()
    });
    q.nodes[0].repeat_milli = 2000.into();
    q.nodes[0].fill = FillMode::Remove;
    let plan = TimelinePlan::compile(&q, TimelineLimits::default(), &|| false).unwrap();
    assert_eq!(sample(&plan, 400).motion, sample(&plan, 1600).motion);
    assert_eq!(sample(&plan, 400).motion, sample(&plan, 2200).motion);
    assert!(sample(&plan, 3700).motion.is_empty());
}
#[test]
fn motion_winners_are_independent_of_other_properties() {
    let mut q = timeline();
    let mut next = q.nodes[0].clone();
    next.id = TimingNodeId::new("second").unwrap();
    next.start = TimeCondition::At { offset: t(500) }.into();
    next.effect = Effect::MotionLine {
        target: ObjectId::new("shape").unwrap(),
        from: p("-0.5", "0"),
        to: p("-0.5", "0"),
    };
    q.nodes.push(next.clone());
    next.id = TimingNodeId::new("rotation").unwrap();
    next.effect = Effect::Rotation {
        composition: Default::default(),
        target: ObjectId::new("shape").unwrap(),
        from: 0,
        to: 900000,
    };
    q.nodes.push(next);
    let plan = TimelinePlan::compile(&q, TimelineLimits::default(), &|| false).unwrap();
    let f = sample(&plan, 800);
    assert_eq!(f.rotations.values().next().unwrap().numerator, "300000");
    assert_eq!(f.motion.values().next().unwrap().x.numerator, "-1");
    assert_eq!(f.motion.values().next().unwrap().x.denominator, "2");
    assert!(matches!(
        plan.evaluate(&binding(), t(0), None, &|| true),
        Err(TimelineError::Cancelled)
    ));
}
#[test]
fn decimal_wire_is_canonical_and_rejects_rounding_or_non_finite_values() {
    let value: MotionCoordinate = "-0.0000".to_owned().try_into().unwrap();
    assert_eq!(value.lexical(), "0");
    for bad in [
        ".5",
        "1.",
        "+1",
        "00",
        "NaN",
        "inf",
        "1e-2",
        "0.0000000000000000001",
        "1000000000",
        "1\n",
        "１",
        "-",
    ] {
        assert!(MotionCoordinate::try_from(bad.to_owned()).is_err(), "{bad}");
    }
    assert!(MotionCoordinate::try_from("1".repeat(10000)).is_err());
    let a = p("999999999.999999999999999999", "0");
    assert!(a.x.checked_add(&p("0.000000000000000001", "0").x).is_err());
}
