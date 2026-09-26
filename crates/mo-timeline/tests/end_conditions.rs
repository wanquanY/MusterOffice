use mo_common::*;
use mo_timeline::*;
fn time(n: i64, d: u32) -> RationalTime {
    RationalTime::new(n, d).unwrap()
}
fn id(s: &str) -> TimingNodeId {
    TimingNodeId::new(s).unwrap()
}
fn at(n: i64, d: u32) -> StartCondition {
    StartCondition::At { offset: time(n, d) }
}
fn exact(n: i64, d: i64) -> ExactValue {
    ExactValue {
        numerator: n.to_string(),
        denominator: d.to_string(),
    }
}
fn node(name: &str) -> TimingNode {
    TimingNode {
        id: id(name),
        start: at(0, 1),
        duration: time(2, 1),
        end_conditions: vec![],
        repeat_milli: RepeatCount::Indefinite,
        repeat_duration: None,
        fill: FillMode::Hold,
        time_transform: None,
        effect: Effect::Rotation {
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
fn after(name: &str, event: NodeEvent, n: i64, d: u32) -> TimeCondition {
    TimeCondition::After {
        node: id(name),
        event,
        delay: time(n, d),
    }
}
fn history(times: &[i64]) -> EventHistory {
    EventHistory {
        binding: binding(),
        through: time(100, 1),
        events: times
            .iter()
            .enumerate()
            .map(|(i, &n)| PlaybackEvent {
                generation: binding().generation,
                sequence: i as u32 + 1,
                at: time(n, 1),
                event: InputEvent::Click { target: None },
            })
            .collect(),
    }
}
#[test]
fn begin_and_end_vertices_allow_mutual_node_references_without_an_event_cycle() {
    let mut a = node("a");
    a.repeat_milli = 5000.into();
    a.end_conditions = vec![after("b", NodeEvent::Begin, 0, 1)];
    let mut b = node("b");
    b.start = after("a", NodeEvent::Begin, 1, 1);
    let mut t = timeline(vec![a, b], None);
    let f = sample(&t, 2, 1);
    assert_eq!(f.state.nodes[0].end, Some(exact(1, 1)));
    assert_eq!(f.state.nodes[0].progress, Some(exact(1, 2)));
    assert_eq!(f.state.nodes[1].start, Some(exact(1, 1)));
    t.nodes[1].start = after("a", NodeEvent::End, 0, 1);
    assert!(TimelinePlan::compile(&t, TimelineLimits::default(), &|| false).is_err());
    t.nodes.pop();
    t.nodes[0].end_conditions = vec![after("a", NodeEvent::Begin, 1, 2)];
    assert_eq!(sample(&t, 1, 1).state.nodes[0].end, Some(exact(1, 2)));
    t.nodes[0].end_conditions = vec![after("a", NodeEvent::End, 0, 1)];
    assert!(TimelinePlan::compile(&t, TimelineLimits::default(), &|| false).is_err());
}
#[test]
fn earliest_eligible_end_is_unscaled_and_redefines_reverse_before_parent_clipping() {
    let mut a = node("a");
    a.start = at(1, 1);
    a.duration = time(4, 1);
    a.repeat_milli = 1000.into();
    a.time_transform = Some(TimeTransform {
        speed_milli_percent: -200000,
        ..Default::default()
    });
    a.end_conditions = vec![at(0, 1), at(2, 1), after("a", NodeEvent::Begin, 3, 1)];
    let mut t = timeline(vec![a], None);
    let f = sample(&t, 1, 1);
    assert_eq!(f.state.nodes[0].end, Some(exact(2, 1)));
    assert_eq!(f.state.nodes[0].progress, Some(exact(1, 2)));
    t.format = TimelineVersion::V02;
    t.tree = Some(TimingTree {
        roots: vec![id("p")],
        containers: vec![container(
            "p",
            &["a"],
            ContainerDuration::Fixed {
                duration: time(3, 2),
            },
        )],
    });
    let f = sample(&t, 5, 1);
    assert_eq!(f.state.nodes[0].end, Some(exact(3, 2)));
    assert_eq!(f.state.nodes[0].progress, Some(exact(1, 4)));
    t.nodes[0].repeat_milli = RepeatCount::Indefinite;
    assert_eq!(sample(&t, 1, 1).state.nodes[0].progress, Some(exact(1, 2)));
}
#[test]
fn clicks_distinguish_activation_stop_and_successor_at_the_same_timestamp() {
    let click = TimeCondition::Click {
        target: None,
        delay: time(0, 1),
    };
    let mut a = node("a");
    a.start = click.clone();
    a.end_conditions = vec![click.clone()];
    let mut b = node("b");
    b.start = click;
    let t = timeline(
        vec![a, b],
        Some(TimingTree {
            roots: vec![id("s")],
            containers: vec![container("s", &["a", "b"], ContainerDuration::Automatic)],
        }),
    );
    let p = plan(&t);
    assert!(matches!(
        p.evaluate(&binding(), time(1, 1), None, &|| false),
        Err(TimelineError::MissingEventHistory)
    ));
    let h = history(&[1, 1, 1]);
    let f = p
        .evaluate(&binding(), time(1, 1), Some(&h), &|| false)
        .unwrap();
    assert_eq!(f.state.nodes[0].end, Some(exact(1, 1)));
    assert_eq!(f.state.nodes[0].phase, NodePhase::Frozen);
    assert_eq!(f.state.nodes[1].start, Some(exact(1, 1)));
    assert_eq!(f.state.event_cursor, 3);
    let before = p
        .evaluate(&binding(), time(0, 1), Some(&h), &|| false)
        .unwrap();
    assert_eq!(before.state.nodes[0].phase, NodePhase::Waiting);
    let h = history(&[1, 1]);
    let f = p
        .evaluate(&binding(), time(1, 1), Some(&h), &|| false)
        .unwrap();
    assert_eq!(f.state.nodes[1].phase, NodePhase::Waiting);
}
#[test]
fn before_activation_clicks_are_ignored_and_delay_is_in_parent_time() {
    let mut a = node("a");
    a.start = at(2, 1);
    a.time_transform = Some(TimeTransform {
        speed_milli_percent: 200000,
        ..Default::default()
    });
    a.end_conditions = vec![TimeCondition::Click {
        target: None,
        delay: time(1, 1),
    }];
    let t = timeline(vec![a], None);
    let h = history(&[0, 1, 3, 8]);
    let p = plan(&t);
    let f = p
        .evaluate(&binding(), time(3, 1), Some(&h), &|| false)
        .unwrap();
    assert_eq!(f.state.nodes[0].end, Some(exact(4, 1)));
    assert_eq!(f.state.nodes[0].phase, NodePhase::Active);
    let f = p
        .evaluate(&binding(), time(20, 1), Some(&h), &|| false)
        .unwrap();
    assert_eq!(f.state.nodes[0].end, Some(exact(4, 1)));
    assert_eq!(f.state.nodes[0].iteration.as_deref(), Some("1"));
    assert_eq!(f.state.nodes[0].progress, Some(exact(1, 1)));
}
#[test]
fn sequence_end_offsets_use_its_gate_and_late_stop_events_remain_available_to_successors() {
    let mut a = node("a");
    a.repeat_milli = 1000.into();
    let mut b = node("b");
    b.start = at(1, 1);
    b.end_conditions = vec![at(0, 1), at(2, 1)];
    let mut t = timeline(
        vec![a, b],
        Some(TimingTree {
            roots: vec![id("s")],
            containers: vec![container("s", &["a", "b"], ContainerDuration::Automatic)],
        }),
    );
    let f = sample(&t, 10, 1);
    assert_eq!(f.state.nodes[1].start, Some(exact(3, 1)));
    assert_eq!(f.state.nodes[1].end, Some(exact(4, 1)));
    t.nodes[0].end_conditions = vec![TimeCondition::Click {
        target: None,
        delay: time(0, 1),
    }];
    t.nodes[1].start = TimeCondition::Click {
        target: None,
        delay: time(0, 1),
    };
    t.nodes[1].end_conditions.clear();
    let h = history(&[3]);
    let f = plan(&t)
        .evaluate(&binding(), time(3, 1), Some(&h), &|| false)
        .unwrap();
    assert_eq!(f.state.nodes[0].end, Some(exact(2, 1)));
    assert_eq!(f.state.nodes[1].start, Some(exact(3, 1)));
}
#[test]
fn expired_declared_ends_diagnose_and_unknown_events_keep_the_interval_open() {
    let mut a = node("a");
    a.start = at(2, 1);
    a.end_conditions = vec![at(1, 1)];
    let mut t = timeline(vec![a], None);
    assert!(
        matches!(plan(&t).evaluate(&binding(),time(3,1),None,&||false),Err(TimelineError::Invalid {node:Some(n),..}) if n==id("a"))
    );
    t.nodes[0].end_conditions.push(TimeCondition::Click {
        target: None,
        delay: time(0, 1),
    });
    let h = history(&[]);
    let f = plan(&t)
        .evaluate(&binding(), time(3, 1), Some(&h), &|| false)
        .unwrap();
    assert_eq!(f.state.nodes[0].phase, NodePhase::Active);
    assert_eq!(f.state.nodes[0].end, None);
}
#[test]
fn end_conditions_have_shared_reference_budget_cancellation_and_wire_rules() {
    let mut a = node("a");
    a.end_conditions = vec![
        at(1, 1),
        TimeCondition::Click {
            target: Some(ObjectId::new("stop").unwrap()),
            delay: time(0, 1),
        },
    ];
    let mut t = timeline(vec![a], None);
    assert_eq!(plan(&t).targets().len(), 2);
    assert!(t.nodes[0].references_object(&ObjectId::new("stop").unwrap()));
    assert!(matches!(
        TimelinePlan::compile(
            &t,
            TimelineLimits {
                max_conditions: 2,
                ..Default::default()
            },
            &|| false
        ),
        Err(TimelineError::Limit("timing condition count"))
    ));
    assert!(
        TimelinePlan::compile(
            &t,
            TimelineLimits {
                max_conditions: 3,
                ..Default::default()
            },
            &|| false
        )
        .is_ok()
    );
    assert!(matches!(
        TimelinePlan::compile(&t, TimelineLimits::default(), &|| true),
        Err(TimelineError::Cancelled)
    ));
    t.nodes[0].end_conditions = vec![after("missing", NodeEvent::End, 0, 1)];
    assert!(TimelinePlan::compile(&t, TimelineLimits::default(), &|| false).is_err());
    t.nodes[0].end_conditions = vec![at(-1, 1)];
    assert!(TimelinePlan::compile(&t, TimelineLimits::default(), &|| false).is_err());
    t.nodes[0].end_conditions.clear();
    assert!(
        serde_json::to_value(&t.nodes[0])
            .unwrap()
            .get("endConditions")
            .is_none()
    );
}
