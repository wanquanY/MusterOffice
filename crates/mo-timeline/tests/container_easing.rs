use mo_common::*;
use mo_timeline::*;
fn t(n: i64, d: u32) -> RationalTime {
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
fn binding() -> PlaybackBinding {
    PlaybackBinding {
        session: PlaybackSessionId::new("container-easing").unwrap(),
        revision: Digest::from_sha256([8; 32]),
        generation: PlaybackGeneration::new(1),
    }
}
fn node(name: &str) -> TimingNode {
    TimingNode {
        id: id(name),
        restart: RestartMode::Always,
        start: TimeCondition::At { offset: t(0, 1) }.into(),
        end_conditions: vec![],
        duration: t(2, 1),
        repeat_milli: 1000.into(),
        repeat_duration: None,
        fill: FillMode::Hold,
        time_transform: None,
        effect: Effect::Rotation {
            composition: Default::default(),
            target: ObjectId::new(name).unwrap(),
            from: 0,
            to: 100,
        },
    }
}
fn container(name: &str, children: &[&str]) -> TimingContainer {
    TimingContainer {
        id: id(name),
        presentation: None,
        navigation: None,
        restart: RestartMode::Always,
        start: TimeCondition::At { offset: t(0, 1) }.into(),
        end_conditions: vec![],
        kind: ContainerKind::Parallel,
        duration: ContainerDuration::Automatic,
        fill: FillMode::Hold,
        children: children.iter().map(|s| id(s)).collect(),
        time_transform: Some(TimeTransform {
            acceleration_milli_percent: 50_000,
            deceleration_milli_percent: 50_000,
            ..Default::default()
        }),
    }
}
fn model() -> Timeline {
    Timeline {
        format: TimelineVersion::V02,
        nodes: vec![node("a"), node("b")],
        tree: Some(TimingTree {
            roots: vec![id("group")],
            containers: vec![container("group", &["a", "b"])],
        }),
    }
}
fn sample(m: &Timeline, at: RationalTime, history: Option<&EventHistory>) -> FrameState {
    TimelinePlan::compile(m, TimelineLimits::default(), &|| false)
        .unwrap()
        .evaluate(&binding(), at, history, &|| false)
        .unwrap()
        .state
}
#[test]
fn shared_clock_cascades_before_leaf_repeat_and_keeps_external_event_endpoints() {
    let mut m = model();
    // The second leaf repeats twice during the same container span. Its own
    // iterations consume already-filtered parent time, not independent easing.
    m.nodes[1].duration = t(1, 1);
    m.nodes[1].repeat_milli = 2000.into();
    let mut outside = node("outside");
    outside.start = TimeCondition::After {
        node: id("a"),
        event: NodeEvent::End,
        delay: t(1, 3),
    }
    .into();
    m.nodes.push(outside);
    m.tree.as_mut().unwrap().roots.push(id("outside"));
    let f = sample(&m, t(1, 2), None);
    assert_eq!(f.nodes[0].progress, Some(exact(1, 8)));
    assert_eq!(f.nodes[1].progress, Some(exact(1, 4)));
    // Restart-capable plans do not fire future causal events before the horizon.
    assert_eq!(f.nodes[2].start, None);
    assert_eq!(sample(&m, t(3, 1), None).nodes[2].start, Some(exact(7, 3)));
    assert_eq!(f.nodes[0].end, Some(exact(2, 1)));
    let f = sample(&m, t(3, 2), None);
    assert_eq!(f.nodes[0].progress, Some(exact(7, 8)));
    assert_eq!(f.nodes[1].progress, Some(exact(3, 4)));
    assert_eq!(f.nodes[1].iteration.as_deref(), Some("1"));
    assert_eq!(
        sample(&m, t(10, 1), None).nodes[0].progress,
        Some(exact(1, 1))
    );
}
#[test]
fn nested_filters_compose_in_order_and_ancestor_clipping_samples_the_original_clock() {
    let mut m = model();
    let mut outer = container("outer", &["group"]);
    outer.time_transform = Some(TimeTransform {
        acceleration_milli_percent: 100_000,
        ..Default::default()
    });
    m.tree.as_mut().unwrap().roots = vec![id("outer")];
    m.tree.as_mut().unwrap().containers.push(outer);
    // At one quarter: outer maps p -> p² = 1/16, inner maps -> 2p² = 1/128.
    assert_eq!(
        sample(&m, t(1, 2), None).nodes[0].progress,
        Some(exact(1, 128))
    );
    let mut clip = container("clip", &["outer"]);
    clip.time_transform = None;
    clip.duration = ContainerDuration::Fixed { duration: t(1, 2) };
    m.tree.as_mut().unwrap().roots = vec![id("clip")];
    m.tree.as_mut().unwrap().containers.push(clip);
    for at in [t(1, 2), t(3, 1), t(100, 1)] {
        let f = sample(&m, at, None);
        assert_eq!(f.nodes[0].progress, Some(exact(1, 128)));
        assert_eq!(f.nodes[0].phase, NodePhase::Frozen);
        assert_eq!(f.nodes[0].end, Some(exact(1, 2)));
    }
}
#[test]
fn restart_and_scoped_seek_use_each_activation_clock_without_touching_siblings() {
    let mut m = model();
    m.tree.as_mut().unwrap().containers[0].start = TimeCondition::Click {
        target: None,
        delay: t(0, 1),
    }
    .into();
    let events = EventHistory {
        binding: binding(),
        through: t(100, 1),
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
    assert_eq!(
        sample(&m, t(3, 2), Some(&events)).nodes[0].progress,
        Some(exact(1, 8))
    );
    let f = sample(&m, t(5, 2), Some(&events));
    assert_eq!(f.nodes[0].start, Some(exact(2, 1)));
    assert_eq!(f.nodes[0].progress, Some(exact(1, 8)));
    let mut seq = container("sequence", &["group"]);
    seq.kind = ContainerKind::Sequence;
    seq.duration = ContainerDuration::Indefinite;
    seq.time_transform = None;
    seq.navigation = Some(SequenceNavigation {
        concurrent: false,
        next_action: NextAction::Seek,
        previous_action: PreviousAction::None,
        next_conditions: vec![TimeCondition::Navigation {
            direction: NavigationDirection::Next,
            target: None,
            delay: t(0, 1),
        }],
        previous_conditions: vec![],
    });
    m.tree.as_mut().unwrap().roots = vec![id("sequence"), id("outside")];
    m.tree.as_mut().unwrap().containers.push(seq);
    m.tree.as_mut().unwrap().containers[0].start = TimeCondition::At { offset: t(0, 1) }.into();
    m.nodes.push(node("outside"));
    let events = EventHistory {
        binding: binding(),
        through: t(100, 1),
        events: vec![PlaybackEvent {
            generation: binding().generation,
            sequence: 1,
            at: t(1, 2),
            event: InputEvent::Navigation {
                direction: NavigationDirection::Next,
                target: None,
            },
        }],
    };
    let f = sample(&m, t(1, 2), Some(&events));
    assert_eq!(f.nodes[0].progress, Some(exact(1, 1)));
    assert_eq!(f.nodes[0].end, Some(exact(1, 2)));
    assert_eq!(f.nodes[2].progress, Some(exact(1, 4)));
}
#[test]
fn nonlinear_deadline_domains_are_diagnosed_not_flattened_or_rounded() {
    let cases: Vec<fn(&mut Timeline)> = vec![
        |m| m.nodes[1].duration = t(1, 1),
        |m| m.nodes[0].start = TimeCondition::At { offset: t(1, 2) }.into(),
        |m| m.nodes[0].repeat_milli = RepeatCount::Indefinite,
        |m| m.nodes[0].end_conditions = vec![TimeCondition::At { offset: t(1, 1) }],
        |m| m.tree.as_mut().unwrap().containers[0].kind = ContainerKind::Sequence,
        |m| m.tree.as_mut().unwrap().containers[0].duration = ContainerDuration::Indefinite,
        |m| {
            m.tree.as_mut().unwrap().containers[0]
                .time_transform
                .as_mut()
                .unwrap()
                .acceleration_milli_percent = 50_001
        },
    ];
    for change in cases {
        let mut m = model();
        change(&mut m);
        assert!(TimelinePlan::compile(&m, TimelineLimits::default(), &|| false).is_err());
    }
    assert!(matches!(
        TimelinePlan::compile(&model(), TimelineLimits::default(), &|| true),
        Err(TimelineError::Cancelled)
    ));
}
