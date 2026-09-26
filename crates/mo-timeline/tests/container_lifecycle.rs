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
        session: PlaybackSessionId::new("container").unwrap(),
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
fn descendant_begin_can_stop_parent_without_creating_a_false_cycle() {
    let mut a = node("a");
    a.start = at(1, 1);
    a.duration = time(10, 1);
    a.fill = FillMode::Remove;
    let mut p = container("p", &["a"], ContainerDuration::Indefinite);
    p.end_conditions = vec![after("a", NodeEvent::Begin, 2, 1)];
    let mut outside = node("outside");
    outside.start = after("a", NodeEvent::End, 0, 1);
    let t = timeline(
        vec![a, outside],
        Some(TimingTree {
            roots: vec![id("p"), id("outside")],
            containers: vec![p],
        }),
    );
    let f = sample(&t, 10, 1);
    assert_eq!(f.state.containers[0].end, Some(exact(3, 1)));
    assert_eq!(f.state.nodes[0].start, Some(exact(1, 1)));
    assert_eq!(f.state.nodes[0].end, Some(exact(3, 1)));
    assert_eq!(f.state.nodes[0].progress, Some(exact(1, 5)));
    assert_eq!(f.state.nodes[1].start, Some(exact(3, 1)));
}
#[test]
fn stop_cancels_future_begins_and_does_not_emit_events_for_unactivated_nodes() {
    let mut a = node("a");
    a.start = at(2, 1);
    let b = node("b");
    let mut p = container("p", &["a", "b"], ContainerDuration::Indefinite);
    p.kind = ContainerKind::Parallel;
    p.end_conditions = vec![at(1, 1)];
    let mut absent = node("absent");
    absent.start = after("a", NodeEvent::Begin, 0, 1);
    let mut ended = node("ended");
    ended.start = after("b", NodeEvent::End, 0, 1);
    let t = timeline(
        vec![a, b, absent, ended],
        Some(TimingTree {
            roots: vec![id("p"), id("absent"), id("ended")],
            containers: vec![p],
        }),
    );
    let f = sample(&t, 5, 1);
    assert_eq!(f.state.nodes[0].start, None);
    assert_eq!(f.state.nodes[0].end, None);
    assert_eq!(f.state.nodes[2].start, None);
    assert_eq!(f.state.nodes[3].start, Some(exact(1, 1)));
    assert_eq!(f.state.nodes[0].phase, NodePhase::Suppressed);
}
#[test]
fn zero_delay_descendant_stop_is_causal_and_closes_scope_before_more_children_begin() {
    let a = node("a");
    let b = node("b");
    let mut p = container("p", &["a", "b"], ContainerDuration::Indefinite);
    p.kind = ContainerKind::Parallel;
    p.end_conditions = vec![after("a", NodeEvent::Begin, 0, 1)];
    let t = timeline(
        vec![a, b],
        Some(TimingTree {
            roots: vec![id("p")],
            containers: vec![p],
        }),
    );
    let f = sample(&t, 0, 1);
    assert_eq!(f.state.containers[0].end, Some(exact(0, 1)));
    assert_eq!(f.state.nodes[0].start, Some(exact(0, 1)));
    assert_eq!(f.state.nodes[0].end, Some(exact(0, 1)));
    assert_eq!(f.state.nodes[1].start, None);
}
#[test]
fn automatic_parent_chooses_child_completion_or_explicit_end_and_keeps_sequence_closed() {
    let mut a = node("a");
    a.repeat_milli = 5000.into();
    let b = node("b");
    let mut p = container("p", &["a", "b"], ContainerDuration::Automatic);
    p.end_conditions = vec![TimeCondition::Click {
        target: None,
        delay: time(0, 1),
    }];
    let mut c = node("c");
    c.start = TimeCondition::Click {
        target: None,
        delay: time(0, 1),
    };
    let t = timeline(
        vec![a, b, c],
        Some(TimingTree {
            roots: vec![id("outer")],
            containers: vec![
                container("outer", &["p", "c"], ContainerDuration::Automatic),
                p,
            ],
        }),
    );
    let h = history(&[3, 3]);
    let f = plan(&t)
        .evaluate(&binding(), time(3, 1), Some(&h), &|| false)
        .unwrap();
    assert_eq!(f.state.containers[1].end, Some(exact(3, 1)));
    assert_eq!(f.state.nodes[0].end, Some(exact(3, 1)));
    assert_eq!(f.state.nodes[1].start, None);
    assert_eq!(f.state.nodes[2].start, Some(exact(3, 1)));
    let back = plan(&t)
        .evaluate(&binding(), time(2, 1), Some(&h), &|| false)
        .unwrap();
    assert_eq!(back.state.containers[1].end, None);
    assert_eq!(back.state.nodes[1].start, Some(exact(10, 1)));
    let mut finite = t;
    finite.nodes[1].repeat_milli = 1000.into();
    let empty = history(&[]);
    let f = plan(&finite)
        .evaluate(&binding(), time(20, 1), Some(&empty), &|| false)
        .unwrap();
    assert_eq!(f.state.containers[1].end, Some(exact(12, 1)));
}
#[test]
fn nested_automatic_scopes_preserve_ancestor_holding_and_reverse_endpoint() {
    let mut a = node("a");
    a.fill = FillMode::Remove;
    a.duration = time(4, 1);
    a.time_transform = Some(TimeTransform {
        speed_milli_percent: -100000,
        ..Default::default()
    });
    let mut p = container("p", &["auto"], ContainerDuration::Indefinite);
    p.end_conditions = vec![at(3, 1)];
    let t = timeline(
        vec![a],
        Some(TimingTree {
            roots: vec![id("p")],
            containers: vec![p, container("auto", &["a"], ContainerDuration::Automatic)],
        }),
    );
    let f = sample(&t, 20, 1);
    assert_eq!(f.state.nodes[0].progress, Some(exact(0, 1)));
    assert_eq!(f.state.containers[1].phase, NodePhase::Frozen);
    assert_eq!(sample(&t, 0, 1).state.nodes[0].progress, Some(exact(3, 4)));
}
#[test]
fn ending_on_child_completion_releases_outside_sequence_at_the_declared_delay() {
    let mut a = node("a");
    a.repeat_milli = 1000.into();
    let b = node("b");
    let mut p = container("p", &["a"], ContainerDuration::Indefinite);
    p.end_conditions = vec![after("a", NodeEvent::End, 1, 2)];
    let t = timeline(
        vec![a, b],
        Some(TimingTree {
            roots: vec![id("s")],
            containers: vec![container("s", &["p", "b"], ContainerDuration::Automatic), p],
        }),
    );
    let f = sample(&t, 4, 1);
    assert_eq!(f.state.containers[1].end, Some(exact(5, 2)));
    assert_eq!(f.state.nodes[1].start, Some(exact(5, 2)));
}
#[test]
fn container_end_references_limits_wire_and_cancellation_share_admission() {
    let mut p = container("p", &[], ContainerDuration::Indefinite);
    p.end_conditions = vec![TimeCondition::Click {
        target: Some(ObjectId::new("stop").unwrap()),
        delay: time(0, 1),
    }];
    let mut t = timeline(
        vec![],
        Some(TimingTree {
            roots: vec![id("p")],
            containers: vec![p],
        }),
    );
    assert!(plan(&t).targets().contains(&ObjectId::new("stop").unwrap()));
    assert!(matches!(
        TimelinePlan::compile(
            &t,
            TimelineLimits {
                max_conditions: 1,
                ..Default::default()
            },
            &|| false
        ),
        Err(TimelineError::Limit(_))
    ));
    let limited = TimelinePlan::compile(
        &t,
        TimelineLimits {
            max_schedule_steps: 2,
            ..Default::default()
        },
        &|| false,
    )
    .unwrap();
    assert!(matches!(
        limited.evaluate(&binding(), time(2, 1), Some(&history(&[])), &|| false),
        Err(TimelineError::Limit("timing schedule work"))
    ));
    assert!(matches!(
        plan(&t).evaluate(&binding(), time(2, 1), Some(&history(&[])), &|| true),
        Err(TimelineError::Cancelled)
    ));
    t.tree.as_mut().unwrap().containers[0]
        .end_conditions
        .clear();
    assert!(
        serde_json::to_value(&t.tree.as_ref().unwrap().containers[0])
            .unwrap()
            .get("endConditions")
            .is_none()
    );
}

#[test]
fn coincident_explicit_child_end_settles_before_automatic_fill_classification() {
    let mut a = node("a");
    a.end_conditions = vec![after("p", NodeEvent::End, 0, 1)];
    let mut p = container("p", &["inner"], ContainerDuration::Indefinite);
    p.end_conditions = vec![at(3, 1)];
    let mut inner = container("inner", &["a"], ContainerDuration::Automatic);
    inner.fill = FillMode::Remove;
    let t = timeline(
        vec![a],
        Some(TimingTree {
            roots: vec![id("p")],
            containers: vec![p, inner],
        }),
    );
    let f = sample(&t, 10, 1);
    assert_eq!(f.state.containers[1].phase, NodePhase::Finished);
    assert!(f.state.rotations.is_empty());
}
