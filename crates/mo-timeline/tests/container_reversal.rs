use mo_common::*;
use mo_timeline::*;
use serde_json::json;

fn time(n: i64, d: u32) -> RationalTime {
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
        session: PlaybackSessionId::new("container-reversal").unwrap(),
        revision: Digest::from_sha256([14; 32]),
        generation: PlaybackGeneration::new(1),
    }
}
fn model() -> Timeline {
    serde_json::from_value(json!({
        "format":"musteroffice.timeline/0.2-draft",
        "nodes":[
            {"id":"a","start":{"kind":"at","offset":{"ticks":"0","timescale":1}},"duration":{"ticks":"2","timescale":1},"repeatMilli":2000,"fill":"hold","effect":{"kind":"rotation","target":"a","from":0,"to":100}},
            {"id":"b","start":{"kind":"at","offset":{"ticks":"0","timescale":1}},"duration":{"ticks":"2","timescale":1},"repeatMilli":2000,"fill":"hold","effect":{"kind":"rotation","target":"b","from":0,"to":100},"timeTransform":{"speedMilliPercent":-100000,"autoReverse":false,"accelerationMilliPercent":0,"decelerationMilliPercent":0}}
        ],
        "tree":{"roots":["group"],"containers":[{"id":"group","kind":"parallel","start":{"kind":"at","offset":{"ticks":"1","timescale":1}},"duration":{"kind":"automatic"},"fill":"hold","children":["a","b"],"timeTransform":{"speedMilliPercent":-200000,"autoReverse":false,"accelerationMilliPercent":0,"decelerationMilliPercent":0}}]}
    })).unwrap()
}
fn sample(model: &Timeline, at: RationalTime, history: Option<&EventHistory>) -> FrameState {
    TimelinePlan::compile(model, TimelineLimits::default(), &|| false)
        .unwrap()
        .evaluate(&binding(), at, history, &|| false)
        .unwrap()
        .state
}
#[test]
fn cascaded_direction_controls_repeat_boundaries_and_external_notifications() {
    let mut m = model();
    let mut outside = m.nodes[0].clone();
    outside.id = TimingNodeId::new("outside").unwrap();
    outside.start = TimeCondition::After {
        node: m.nodes[0].id.clone(),
        event: NodeEvent::OnEnd,
        delay: time(1, 3),
    }
    .into();
    m.tree.as_mut().unwrap().roots.push(outside.id.clone());
    m.nodes.push(outside);
    for (at, pa, ia, pb, ib) in [
        (time(1, 1), exact(1, 1), "1", exact(0, 1), "0"),
        (time(3, 2), exact(1, 2), "1", exact(1, 2), "0"),
        (time(2, 1), exact(0, 1), "1", exact(0, 1), "1"),
        (time(5, 2), exact(1, 2), "0", exact(1, 2), "1"),
        (time(3, 1), exact(0, 1), "0", exact(1, 1), "1"),
        (time(8, 1), exact(0, 1), "0", exact(1, 1), "1"),
    ] {
        let f = sample(&m, at, None);
        assert_eq!(f.nodes[0].progress, Some(pa));
        assert_eq!(f.nodes[1].progress, Some(pb));
        assert_eq!(f.nodes[0].iteration.as_deref(), Some(ia));
        assert_eq!(f.nodes[1].iteration.as_deref(), Some(ib));
        assert_eq!(f.nodes[0].start, Some(exact(1, 1)));
        assert_eq!(f.nodes[0].end, Some(exact(3, 1)));
    }
    assert_eq!(
        sample(&m, time(8, 1), None).nodes[2].start,
        Some(exact(10, 3))
    );
}
#[test]
fn nested_reversal_and_easing_keep_original_endpoints_when_clipped() {
    let mut m = model();
    m.nodes.truncate(1);
    m.nodes[0].repeat_milli = 1000.into();
    let tree = m.tree.as_mut().unwrap();
    let inner = &mut tree.containers[0];
    inner.children.truncate(1);
    inner.start = TimeCondition::At { offset: time(0, 1) }.into();
    inner
        .time_transform
        .as_mut()
        .unwrap()
        .acceleration_milli_percent = 100_000;
    // Inner span is 1 second. Outer reverses it again, without flattening
    // the asymmetric easing between the two reflections: p becomes p².
    let mut outer = inner.clone();
    outer.id = TimingNodeId::new("outer").unwrap();
    outer.children = vec![inner.id.clone()];
    outer.time_transform = Some(TimeTransform {
        speed_milli_percent: -100_000,
        ..Default::default()
    });
    tree.roots = vec![outer.id.clone()];
    tree.containers.push(outer);
    assert_eq!(
        sample(&m, time(1, 4), None).nodes[0].progress,
        Some(exact(1, 16))
    );
    assert_eq!(
        sample(&m, time(1, 1), None).nodes[0].progress,
        Some(exact(1, 1))
    );
    let tree = m.tree.as_mut().unwrap();
    let mut clip = tree.containers[1].clone();
    clip.id = TimingNodeId::new("clip").unwrap();
    clip.children = tree.roots.clone();
    clip.time_transform = None;
    clip.duration = ContainerDuration::Fixed {
        duration: time(1, 4),
    };
    tree.roots = vec![clip.id.clone()];
    tree.containers.push(clip);
    let f = sample(&m, time(9, 1), None);
    assert_eq!(f.nodes[0].progress, Some(exact(1, 16)));
    assert_eq!(f.nodes[0].phase, NodePhase::Frozen);
    assert_eq!(f.nodes[0].end, Some(exact(1, 4)));
}
#[test]
fn restart_creates_a_new_reverse_origin_and_history_can_be_sampled_backwards() {
    let mut m = model();
    m.tree.as_mut().unwrap().containers[0].restart = RestartMode::Always;
    m.tree.as_mut().unwrap().containers[0].start = TimeCondition::Click {
        target: None,
        delay: time(0, 1),
    }
    .into();
    let history = EventHistory {
        binding: binding(),
        through: time(10, 1),
        events: [1, 2]
            .into_iter()
            .enumerate()
            .map(|(i, at)| PlaybackEvent {
                generation: binding().generation,
                sequence: i as u32 + 1,
                at: time(at, 1),
                event: InputEvent::Click { target: None },
            })
            .collect(),
    };
    let plan = TimelinePlan::compile(&m, TimelineLimits::default(), &|| false).unwrap();
    for (at, start, progress) in [
        (time(5, 2), 2, exact(1, 2)),
        (time(2, 1), 2, exact(1, 1)),
        (time(3, 2), 1, exact(1, 2)),
    ] {
        let f = plan
            .evaluate(&binding(), at, Some(&history), &|| false)
            .unwrap()
            .state;
        assert_eq!(f.nodes[0].start, Some(exact(start, 1)));
        assert_eq!(f.nodes[0].progress, Some(progress));
    }
}
#[test]
fn navigation_seek_and_previous_keep_reverse_endpoints_scoped_to_the_activation() {
    let mut m = model();
    let group = &mut m.tree.as_mut().unwrap().containers[0];
    group.start = TimeCondition::At { offset: time(0, 1) }.into();
    let mut sequence = group.clone();
    sequence.id = TimingNodeId::new("sequence").unwrap();
    sequence.kind = ContainerKind::Sequence;
    sequence.duration = ContainerDuration::Indefinite;
    sequence.time_transform = None;
    sequence.children = vec![group.id.clone()];
    sequence.navigation = Some(SequenceNavigation {
        concurrent: false,
        next_action: NextAction::Seek,
        previous_action: PreviousAction::None,
        next_conditions: vec![TimeCondition::Navigation {
            direction: NavigationDirection::Next,
            target: None,
            delay: time(0, 1),
        }],
        previous_conditions: vec![TimeCondition::Navigation {
            direction: NavigationDirection::Previous,
            target: None,
            delay: time(0, 1),
        }],
    });
    m.tree.as_mut().unwrap().roots = vec![sequence.id.clone()];
    m.tree.as_mut().unwrap().containers.push(sequence);
    let history = EventHistory {
        binding: binding(),
        through: time(10, 1),
        events: [
            (time(1, 2), NavigationDirection::Next),
            (time(3, 4), NavigationDirection::Previous),
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
    let f = sample(&m, time(1, 2), Some(&history));
    assert_eq!(f.nodes[0].progress, Some(exact(0, 1)));
    assert_eq!(f.nodes[1].progress, Some(exact(1, 1)));
    assert_eq!(f.nodes[0].end, Some(exact(1, 2)));
    let f = sample(&m, time(3, 4), Some(&history));
    assert_eq!(f.nodes[0].start, Some(exact(3, 4)));
    assert_eq!(f.nodes[0].progress, Some(exact(1, 1)));
    assert_eq!(f.nodes[1].progress, Some(exact(0, 1)));
}
#[test]
fn full_signed_speed_range_and_exact_arithmetic_budget_are_preserved() {
    let mut m = model();
    m.tree.as_mut().unwrap().containers[0]
        .time_transform
        .as_mut()
        .unwrap()
        .speed_milli_percent = i32::MIN;
    let f = sample(&m, time(1, 1), None);
    assert_eq!(f.nodes[0].end, Some(exact(16_780_341, 16_777_216)));
    assert_eq!(f.nodes[0].progress, Some(exact(1, 1)));
    let limits = TimelineLimits {
        max_exact_bits: 8,
        ..Default::default()
    };
    assert!(TimelinePlan::compile(&m, limits, &|| false).is_err());
    assert!(matches!(
        TimelinePlan::compile(&m, TimelineLimits::default(), &|| true),
        Err(TimelineError::Cancelled)
    ));
}
#[test]
fn noncoincident_intervals_are_not_misrepresented_as_property_reversal() {
    let changes: Vec<fn(&mut Timeline)> = vec![
        |m| m.nodes[0].duration = time(1, 1),
        |m| m.nodes[0].start = TimeCondition::At { offset: time(1, 1) }.into(),
        |m| m.nodes[0].end_conditions = vec![TimeCondition::At { offset: time(1, 1) }],
        |m| m.tree.as_mut().unwrap().containers[0].kind = ContainerKind::Sequence,
        |m| m.nodes[0].repeat_milli = RepeatCount::Indefinite,
    ];
    for change in changes {
        let mut m = model();
        change(&mut m);
        assert!(TimelinePlan::compile(&m, TimelineLimits::default(), &|| false).is_err());
    }
}
