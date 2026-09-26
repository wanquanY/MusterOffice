use mo_common::*;
use mo_timeline::*;
fn time(n: i64, d: u32) -> RationalTime {
    RationalTime::new(n, d).unwrap()
}
fn id(s: &str) -> TimingNodeId {
    TimingNodeId::new(s).unwrap()
}
fn exact(n: i64, d: i64) -> ExactValue {
    ExactValue {
        numerator: n.to_string(),
        denominator: d.to_string(),
    }
}
fn node() -> TimingNode {
    TimingNode {
        id: id("a"),
        start: StartCondition::At { offset: time(0, 1) },
        duration: time(1, 1),
        end_conditions: vec![],
        repeat_milli: 1000.into(),
        repeat_duration: None,
        fill: FillMode::Hold,
        time_transform: Some(TimeTransform::default()),
        effect: Effect::Rotation {
            target: ObjectId::new("shape").unwrap(),
            from: 0,
            to: 120,
        },
    }
}
fn timeline(n: TimingNode) -> Timeline {
    Timeline {
        format: TimelineVersion::V01,
        nodes: vec![n],
        tree: None,
    }
}
fn binding() -> PlaybackBinding {
    PlaybackBinding {
        session: PlaybackSessionId::new("clock").unwrap(),
        revision: Digest::from_sha256([1; 32]),
        generation: PlaybackGeneration::new(1),
    }
}
fn sample(t: &Timeline, n: i64, d: u32) -> EvaluatedFrame {
    TimelinePlan::compile(t, TimelineLimits::default(), &|| false)
        .unwrap()
        .evaluate(&binding(), time(n, d), None, &|| false)
        .unwrap()
}
#[test]
fn rate_changes_duration_and_end_dependencies_without_scaling_start_offsets() {
    let mut a = node();
    a.duration = time(2, 1);
    a.start = StartCondition::At { offset: time(1, 2) };
    a.time_transform.as_mut().unwrap().speed_milli_percent = 200_000;
    let mut b = node();
    b.id = id("b");
    b.start = StartCondition::After {
        node: id("a"),
        event: NodeEvent::End,
        delay: time(1, 4),
    };
    let mut t = timeline(a);
    t.nodes.push(b);
    let f = sample(&t, 1, 1);
    assert_eq!(f.state.nodes[0].start, Some(exact(1, 2)));
    assert_eq!(f.state.nodes[0].end, Some(exact(3, 2)));
    assert_eq!(f.state.nodes[0].progress, Some(exact(1, 2)));
    assert_eq!(f.state.nodes[1].start, Some(exact(7, 4)));
}
#[test]
fn reverse_rate_reverses_the_full_fractional_active_interval_and_handles_boundaries() {
    let mut n = node();
    n.duration = time(2, 1);
    n.repeat_milli = 2500.into();
    n.time_transform.as_mut().unwrap().speed_milli_percent = -200_000;
    let mut t = timeline(n);
    for (a, d, i, pn, pd) in [
        (0, 1, "2", 1, 2),
        (1, 2, "2", 0, 1),
        (1, 1, "1", 1, 2),
        (3, 2, "1", 0, 1),
        (2, 1, "0", 1, 2),
        (5, 2, "0", 0, 1),
        (7, 1, "0", 0, 1),
    ] {
        let f = sample(&t, a, d);
        assert_eq!(f.state.nodes[0].iteration.as_deref(), Some(i));
        assert_eq!(f.state.nodes[0].progress, Some(exact(pn, pd)));
        assert_eq!(f.state.nodes[0].end, Some(exact(5, 2)));
    }
    t.nodes[0].repeat_milli = 2000.into();
    assert_eq!(sample(&t, 0, 1).state.nodes[0].progress, Some(exact(1, 1)));
    t.nodes[0].duration = time(1, 1);
    t.nodes[0].repeat_milli = 1000.into();
    t.nodes[0]
        .time_transform
        .as_mut()
        .unwrap()
        .speed_milli_percent = i32::MIN;
    assert_eq!(
        sample(&t, 0, 1).state.nodes[0].end,
        Some(exact(3125, 67108864))
    );
}
#[test]
fn auto_reverse_doubles_each_cycle_before_fractional_repeating_and_rate() {
    let mut n = node();
    n.repeat_milli = 2500.into();
    n.time_transform.as_mut().unwrap().auto_reverse = true;
    let mut t = timeline(n);
    for (at, i, p) in [
        (0, "0", 0),
        (1, "0", 1),
        (2, "1", 0),
        (3, "1", 1),
        (4, "2", 0),
        (5, "2", 1),
        (20, "2", 1),
    ] {
        let f = sample(&t, at, 1);
        assert_eq!(f.state.nodes[0].iteration.as_deref(), Some(i));
        assert_eq!(f.state.nodes[0].progress, Some(exact(p, 1)));
        assert_eq!(f.state.nodes[0].end, Some(exact(5, 1)));
    }
    t.nodes[0]
        .time_transform
        .as_mut()
        .unwrap()
        .speed_milli_percent = -200_000;
    assert_eq!(sample(&t, 0, 1).state.nodes[0].progress, Some(exact(1, 1)));
    assert_eq!(sample(&t, 1, 2).state.nodes[0].progress, Some(exact(0, 1)));
    assert_eq!(sample(&t, 5, 2).state.nodes[0].progress, Some(exact(0, 1)));
}
#[test]
fn easing_is_the_exact_velocity_integral_at_joins_and_in_each_reverse_leg() {
    let mut n = node();
    let m = n.time_transform.as_mut().unwrap();
    m.auto_reverse = true;
    m.acceleration_milli_percent = 25000;
    m.deceleration_milli_percent = 25000;
    let mut t = timeline(n);
    for (num, den, pn, pd) in [
        (0, 1, 0, 1),
        (1, 8, 1, 24),
        (1, 4, 1, 6),
        (1, 2, 1, 2),
        (3, 4, 5, 6),
        (7, 8, 23, 24),
        (1, 1, 1, 1),
    ] {
        assert_eq!(
            sample(&t, num, den).state.nodes[0].progress,
            Some(exact(pn, pd))
        );
        assert_eq!(
            sample(&t, 2 * i64::from(den) - num, den).state.nodes[0].progress,
            Some(exact(pn, pd))
        );
    }
    let m = t.nodes[0].time_transform.as_mut().unwrap();
    m.acceleration_milli_percent = 100000;
    m.deceleration_milli_percent = 0;
    assert_eq!(sample(&t, 1, 2).state.nodes[0].progress, Some(exact(1, 4)));
    let m = t.nodes[0].time_transform.as_mut().unwrap();
    m.acceleration_milli_percent = 0;
    m.deceleration_milli_percent = 100000;
    assert_eq!(sample(&t, 1, 2).state.nodes[0].progress, Some(exact(3, 4)));
    let m = t.nodes[0].time_transform.as_mut().unwrap();
    m.acceleration_milli_percent = 50000;
    m.deceleration_milli_percent = 50000;
    assert_eq!(sample(&t, 1, 4).state.nodes[0].progress, Some(exact(1, 8)));
}
#[test]
fn fixed_ancestor_clips_the_transformed_clock_without_restarting_or_shortening_its_definition() {
    let mut n = node();
    n.duration = time(2, 1);
    n.repeat_milli = 2000.into();
    n.fill = FillMode::Remove;
    n.time_transform.as_mut().unwrap().speed_milli_percent = -100000;
    let mut t = timeline(n);
    t.format = TimelineVersion::V02;
    t.tree = Some(TimingTree {
        roots: vec![id("p")],
        containers: vec![TimingContainer {
            id: id("p"),
            kind: ContainerKind::Sequence,
            start: StartCondition::At { offset: time(0, 1) },
            end_conditions: vec![],
            duration: ContainerDuration::Fixed {
                duration: time(3, 2),
            },
            fill: FillMode::Hold,
            children: vec![id("a")],
        }],
    });
    // Full local active time is four seconds: cutoff samples 2.5 seconds,
    // the first quarter of the final cycle. It does not reverse a shortened clip.
    let f = sample(&t, 10, 1);
    assert_eq!(f.state.nodes[0].phase, NodePhase::Frozen);
    assert_eq!(f.state.nodes[0].progress, Some(exact(1, 4)));
    assert_eq!(f.state.nodes[0].end, Some(exact(3, 2)));
    t.nodes[0]
        .time_transform
        .as_mut()
        .unwrap()
        .speed_milli_percent = 200000;
    let f = sample(&t, 10, 1);
    assert_eq!(f.state.nodes[0].progress, Some(exact(1, 2)));
    assert_eq!(sample(&t, 1, 4).state.nodes[0].progress, Some(exact(1, 4)));
}
#[test]
fn invalid_transforms_fail_admission_and_defaults_preserve_legacy_evaluation() {
    let mut n = node();
    n.time_transform = None;
    let t = timeline(n.clone());
    let expected = sample(&t, 1, 3);
    n.time_transform = Some(TimeTransform::default());
    let f = sample(&timeline(n.clone()), 1, 3);
    assert_eq!(f.state.nodes, expected.state.nodes);
    assert_eq!(f.state.rotations, expected.state.rotations);
    assert_ne!(f.sha256, expected.sha256);
    for transform in [
        TimeTransform {
            speed_milli_percent: 0,
            ..Default::default()
        },
        TimeTransform {
            acceleration_milli_percent: 100001,
            ..Default::default()
        },
        TimeTransform {
            acceleration_milli_percent: u32::MAX,
            deceleration_milli_percent: u32::MAX,
            ..Default::default()
        },
        TimeTransform {
            acceleration_milli_percent: 50001,
            deceleration_milli_percent: 50000,
            ..Default::default()
        },
    ] {
        n.time_transform = Some(transform);
        assert!(
            TimelinePlan::compile(&timeline(n.clone()), TimelineLimits::default(), &|| false)
                .is_err()
        );
    }
}
