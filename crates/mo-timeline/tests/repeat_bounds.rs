use mo_common::*;
use mo_timeline::*;
fn time(n: i64, d: u32) -> RationalTime {
    RationalTime::new(n, d).unwrap()
}
fn id(s: &str) -> TimingNodeId {
    TimingNodeId::new(s).unwrap()
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
fn node(name: &str) -> TimingNode {
    TimingNode {
        restart: mo_timeline::RestartMode::Never,
        id: id(name),
        start: at(0, 1),
        duration: time(2, 1),
        end_conditions: vec![],
        repeat_milli: RepeatCount::Indefinite,
        repeat_duration: None,
        fill: FillMode::Hold,
        time_transform: None,
        effect: Effect::Rotation {
            composition: Default::default(),
            target: ObjectId::new(name).unwrap(),
            from: 0,
            to: 120,
        },
    }
}
fn timeline(nodes: Vec<TimingNode>, tree: Option<TimingTree>) -> Timeline {
    Timeline {
        format: if tree.is_some() {
            TimelineVersion::V02
        } else {
            TimelineVersion::V01
        },
        nodes,
        tree,
    }
}
fn container(name: &str, children: &[&str], duration: ContainerDuration) -> TimingContainer {
    TimingContainer {
        time_transform: None,
        presentation: None,
        navigation: None,
        restart: mo_timeline::RestartMode::Never,
        id: id(name),
        kind: ContainerKind::Sequence,
        start: at(0, 1),
        end_conditions: vec![],
        duration,
        fill: FillMode::Hold,
        children: children.iter().map(|n| id(n)).collect(),
    }
}
fn binding() -> PlaybackBinding {
    PlaybackBinding {
        session: PlaybackSessionId::new("repeat").unwrap(),
        revision: Digest::from_sha256([2; 32]),
        generation: PlaybackGeneration::new(1),
    }
}
fn plan(t: &Timeline) -> TimelinePlan {
    TimelinePlan::compile(t, TimelineLimits::default(), &|| false).unwrap()
}
fn sample(t: &Timeline, n: i64, d: u32) -> EvaluatedFrame {
    plan(t)
        .evaluate(&binding(), time(n, d), None, &|| false)
        .unwrap()
}
#[test]
fn repeat_wire_keeps_old_numbers_and_uses_explicit_indefinite_alternatives() {
    let mut n = node("a");
    n.repeat_milli = 2500.into();
    let j = serde_json::to_value(&n).unwrap();
    assert_eq!(j["repeatMilli"], 2500);
    assert!(j.get("repeatDuration").is_none());
    for count in [RepeatCount::Finite(2500), RepeatCount::Indefinite] {
        for duration in [
            None,
            Some(RepeatDuration::Indefinite),
            Some(RepeatDuration::Finite(time(7, 3))),
        ] {
            n.repeat_milli = count;
            n.repeat_duration = duration;
            let wire = serde_json::to_string(&n).unwrap();
            assert_eq!(serde_json::from_str::<TimingNode>(&wire).unwrap(), n);
        }
    }
    for invalid in [
        serde_json::json!(-1),
        serde_json::json!(1.5),
        serde_json::json!("infinity"),
        serde_json::json!({"finite":2500}),
        serde_json::Value::Null,
    ] {
        let mut j = j.clone();
        j["repeatMilli"] = invalid;
        assert!(serde_json::from_value::<TimingNode>(j).is_err());
    }
}
#[test]
fn count_and_duration_bounds_take_the_minimum_before_speed_scaling() {
    let mut n = node("a");
    n.repeat_milli = 2500.into();
    n.repeat_duration = Some(RepeatDuration::Finite(time(3, 1)));
    n.time_transform = Some(TimeTransform {
        speed_milli_percent: 200000,
        auto_reverse: true,
        ..Default::default()
    });
    let mut t = timeline(vec![n], None);
    // Auto-reverse makes the count bound 10 s; repeat duration remains 3 s.
    let f = sample(&t, 20, 1);
    assert_eq!(f.state.nodes[0].end, Some(exact(3, 2)));
    assert_eq!(f.state.nodes[0].progress, Some(exact(1, 2)));
    t.nodes[0].repeat_duration = Some(RepeatDuration::Finite(time(20, 1)));
    let f = sample(&t, 20, 1);
    assert_eq!(f.state.nodes[0].end, Some(exact(5, 1)));
    assert_eq!(f.state.nodes[0].progress, Some(exact(1, 1)));
    t.nodes[0].repeat_duration = Some(RepeatDuration::Indefinite);
    assert_eq!(sample(&t, 20, 1).state.nodes, f.state.nodes);
    t.nodes[0].repeat_duration = Some(RepeatDuration::Finite(time(3, 1)));
    t.nodes[0]
        .time_transform
        .as_mut()
        .unwrap()
        .speed_milli_percent = -200000;
    assert_eq!(sample(&t, 0, 1).state.nodes[0].progress, Some(exact(1, 2)));
    assert_eq!(sample(&t, 20, 1).state.nodes[0].progress, Some(exact(0, 1)));
}
#[test]
fn infinite_repeats_seek_directly_and_keep_end_dependencies_unresolved() {
    let a = node("a");
    let mut b = node("b");
    b.repeat_milli = 1000.into();
    b.start = StartCondition::Single(TimeCondition::After {
        node: id("a"),
        event: NodeEvent::End,
        delay: time(0, 1),
    });
    let mut c = b.clone();
    c.id = id("c");
    c.start = StartCondition::Single(TimeCondition::After {
        node: id("a"),
        event: NodeEvent::Begin,
        delay: time(1, 2),
    });
    let t = timeline(vec![a, b, c], None);
    let p = plan(&t);
    for at in [i64::MAX, 1, 9, 0] {
        let f = p
            .evaluate(&binding(), time(at, 1), None, &|| false)
            .unwrap();
        assert_eq!(f.state.nodes[0].phase, NodePhase::Active);
        assert_eq!(f.state.nodes[0].end, None);
        assert_eq!(
            f.state.nodes[0].iteration.as_ref().unwrap(),
            &(at / 2).to_string()
        );
        assert_eq!(
            f.state.nodes[0].progress,
            Some(exact(at % 2, if at % 2 == 0 { 1 } else { 2 }))
        );
        assert_eq!(f.state.nodes[1].phase, NodePhase::Waiting);
        assert_eq!(f.state.nodes[2].start, Some(exact(1, 2)));
    }
}
#[test]
fn finite_repeat_duration_releases_sequence_and_zero_duration_is_an_instant() {
    let mut a = node("a");
    a.repeat_duration = Some(RepeatDuration::Finite(time(5, 2)));
    let b = node("b");
    let tree = TimingTree {
        roots: vec![id("seq")],
        containers: vec![container("seq", &["a", "b"], ContainerDuration::Automatic)],
    };
    let mut t = timeline(vec![a, b], Some(tree));
    let f = sample(&t, 3, 1);
    assert_eq!(f.state.nodes[0].end, Some(exact(5, 2)));
    assert_eq!(f.state.nodes[1].start, Some(exact(5, 2)));
    assert_eq!(f.state.containers[0].end, None);
    t.nodes[0].repeat_duration = Some(RepeatDuration::Finite(time(0, 1)));
    let f = sample(&t, 0, 1);
    assert_eq!(f.state.nodes[0].phase, NodePhase::Frozen);
    assert_eq!(f.state.nodes[0].progress, Some(exact(0, 1)));
    assert_eq!(f.state.nodes[1].start, Some(exact(0, 1)));
    t.nodes[0].fill = FillMode::Remove;
    assert_eq!(sample(&t, 0, 1).state.nodes[0].phase, NodePhase::Finished);
    t.nodes[0].repeat_duration = Some(RepeatDuration::Finite(time(-1, 1)));
    assert!(TimelinePlan::compile(&t, TimelineLimits::default(), &|| false).is_err());
    t.nodes[0].repeat_duration = None;
    t.nodes[0].repeat_milli = 0.into();
    assert!(TimelinePlan::compile(&t, TimelineLimits::default(), &|| false).is_err());
}
#[test]
fn ancestor_endpoint_bounds_indefinite_reverse_without_redefining_finite_clocks() {
    let mut a = node("a");
    a.start = at(1, 2);
    a.fill = FillMode::Remove;
    a.time_transform = Some(TimeTransform {
        speed_milli_percent: -200000,
        ..Default::default()
    });
    let tree = TimingTree {
        roots: vec![id("outer")],
        containers: vec![
            container(
                "outer",
                &["inner"],
                ContainerDuration::Fixed {
                    duration: time(3, 1),
                },
            ),
            container("inner", &["a", "b"], ContainerDuration::Automatic),
        ],
    };
    let mut t = timeline(vec![a, node("b")], Some(tree));
    // 2.5 parent seconds until cutoff -> 5 local seconds -> final half-cycle.
    for (at, d, pn, pd) in [
        (1, 2, 1, 2),
        (1, 1, 0, 1),
        (3, 2, 1, 2),
        (3, 1, 0, 1),
        (9, 1, 0, 1),
        (1, 2, 1, 2),
    ] {
        let f = sample(&t, at, d);
        assert_eq!(f.state.nodes[0].progress, Some(exact(pn, pd)));
        assert_eq!(f.state.nodes[0].end, Some(exact(3, 1)));
        assert!(f.state.nodes[1].start.is_none());
    }
    // A finite declared 8-second clock is not shortened to that 5-second origin.
    t.nodes[0].repeat_milli = 4000.into();
    assert_eq!(sample(&t, 1, 2).state.nodes[0].progress, Some(exact(1, 1)));
    assert_eq!(sample(&t, 9, 1).state.nodes[0].progress, Some(exact(1, 2)));
}
#[test]
fn unbounded_reverse_reports_missing_endpoint_and_resolved_duration_recovers() {
    let mut a = node("a");
    a.start = at(1, 1);
    a.time_transform = Some(TimeTransform {
        speed_milli_percent: -100000,
        ..Default::default()
    });
    let mut t = timeline(vec![a], None);
    let p = plan(&t);
    assert_eq!(
        p.evaluate(&binding(), time(0, 1), None, &|| false)
            .unwrap()
            .state
            .nodes[0]
            .phase,
        NodePhase::Scheduled
    );
    assert!(
        matches!(p.evaluate(&binding(),time(1,1),None,&||false),Err(TimelineError::Invalid {message,..}) if message.contains("finite endpoint"))
    );
    t.nodes[0].repeat_duration = Some(RepeatDuration::Finite(time(5, 1)));
    assert_eq!(sample(&t, 1, 1).state.nodes[0].progress, Some(exact(1, 2)));
    t.nodes[0].repeat_duration = None;
    t.format = TimelineVersion::V02;
    t.tree = Some(TimingTree {
        roots: vec![id("auto")],
        containers: vec![container("auto", &["a"], ContainerDuration::Automatic)],
    });
    assert!(
        plan(&t)
            .evaluate(&binding(), time(1, 1), None, &|| false)
            .is_err()
    );
}
