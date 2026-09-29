use mo_common::*;
use mo_timeline::*;
use serde_json::json;
use std::cell::Cell;

fn time(n: i64) -> RationalTime {
    RationalTime::new(n, 1).unwrap()
}
fn id(s: &str) -> TimingNodeId {
    TimingNodeId::new(s).unwrap()
}
fn at(n: i64) -> TimeCondition {
    TimeCondition::At { offset: time(n) }
}
fn after(s: &str, event: NodeEvent, delay: i64) -> TimeCondition {
    TimeCondition::After {
        node: id(s),
        event,
        delay: time(delay),
    }
}
fn click(target: &str) -> TimeCondition {
    TimeCondition::Click {
        target: Some(ObjectId::new(target).unwrap()),
        delay: time(0),
    }
}
fn any(conditions: Vec<TimeCondition>) -> StartCondition {
    StartCondition::AnyOf { conditions }
}
fn node(name: &str, start: impl Into<StartCondition>) -> TimingNode {
    TimingNode {
        restart: mo_timeline::RestartMode::Never,
        id: id(name),
        start: start.into(),
        end_conditions: vec![],
        duration: time(2),
        repeat_milli: 1000.into(),
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
fn graph(nodes: Vec<TimingNode>) -> Timeline {
    Timeline {
        format: TimelineVersion::V01,
        nodes,
        tree: None,
    }
}
fn plan(t: &Timeline) -> TimelinePlan {
    TimelinePlan::compile(t, TimelineLimits::default(), &|| false).unwrap()
}
fn binding() -> PlaybackBinding {
    PlaybackBinding {
        session: PlaybackSessionId::new("alternatives").unwrap(),
        revision: Digest::from_sha256([3; 32]),
        generation: PlaybackGeneration::new(1),
    }
}
fn history(entries: &[(i64, &str)]) -> EventHistory {
    EventHistory {
        binding: binding(),
        through: time(100),
        events: entries
            .iter()
            .enumerate()
            .map(|(i, (at, target))| PlaybackEvent {
                generation: binding().generation,
                sequence: i as u32 + 1,
                at: time(*at),
                event: InputEvent::Click {
                    target: Some(ObjectId::new(*target).unwrap()),
                },
            })
            .collect(),
    }
}
fn exact(n: i64) -> Option<ExactValue> {
    Some(ExactValue {
        numerator: n.to_string(),
        denominator: "1".into(),
    })
}
fn sample(t: &Timeline) -> EvaluatedFrame {
    plan(t)
        .evaluate(&binding(), time(100), None, &|| false)
        .unwrap()
}
fn frame<'a>(f: &'a EvaluatedFrame, name: &str) -> &'a NodeFrame {
    f.state.nodes.iter().find(|n| n.node == id(name)).unwrap()
}
fn sequence(t: &mut Timeline, children: &[&str], start: TimeCondition) {
    t.format = TimelineVersion::V02;
    t.tree = Some(TimingTree {
        roots: vec![id("sequence")],
        containers: vec![TimingContainer {
            time_transform: None,
            presentation: None,
            navigation: None,
            restart: mo_timeline::RestartMode::Never,
            id: id("sequence"),
            kind: ContainerKind::Sequence,
            start: start.into(),
            end_conditions: vec![],
            duration: ContainerDuration::Automatic,
            fill: FillMode::Hold,
            children: children.iter().map(|s| id(s)).collect(),
        }],
    });
}

#[test]
fn newly_resolved_begin_or_end_replaces_a_later_queued_trigger_once() {
    for event in [NodeEvent::Begin, NodeEvent::End] {
        for reverse_nodes in [false, true] {
            for reverse_conditions in [false, true] {
                let mut conditions = vec![at(20), after("source", event, 1), at(30)];
                if reverse_conditions {
                    conditions.reverse();
                }
                let mut t = graph(vec![node("target", any(conditions)), node("source", at(3))]);
                if reverse_nodes {
                    t.nodes.reverse();
                }
                let f = sample(&t);
                let begin = if event == NodeEvent::Begin { 4 } else { 6 };
                assert_eq!(frame(&f, "target").start, exact(begin));
                assert_eq!(frame(&f, "target").end, exact(begin + 2));
            }
        }
    }
}

#[test]
fn independently_seeded_alternative_cycles_are_valid_but_never_is_not_a_seed() {
    let mut t = graph(vec![
        node("a", any(vec![after("b", NodeEvent::End, 0), at(0)])),
        node("b", after("a", NodeEvent::End, 1)),
    ]);
    let f = sample(&t);
    assert_eq!(frame(&f, "a").start, exact(0));
    assert_eq!(frame(&f, "b").start, exact(3));
    t.nodes[0].start = any(vec![TimeCondition::Never {}, after("b", NodeEvent::End, 0)]);
    assert!(TimelinePlan::compile(&t, TimelineLimits::default(), &|| false).is_err());
    t.nodes[0].start = any(vec![at(1), after("a", NodeEvent::End, 0)]);
    assert_eq!(frame(&sample(&t), "a").end, exact(3));
    t.nodes[0].start = after("a", NodeEvent::Begin, 0).into();
    assert!(TimelinePlan::compile(&t, TimelineLimits::default(), &|| false).is_err());
}

#[test]
fn never_stays_untriggered_and_does_not_cut_off_other_conditions() {
    let mut t = graph(vec![
        node("a", TimeCondition::Never {}),
        node("b", any(vec![TimeCondition::Never {}, at(3)])),
    ]);
    t.nodes[1].end_conditions = vec![TimeCondition::Never {}];
    let f = sample(&t);
    assert_eq!(frame(&f, "a").phase, NodePhase::Waiting);
    assert_eq!(frame(&f, "a").start, None);
    assert_eq!(frame(&f, "b").end, exact(5));
    t.nodes[1].start = any(vec![TimeCondition::Never {}, TimeCondition::Never {}]);
    assert_eq!(frame(&sample(&t), "b").start, None);
}

#[test]
fn event_sequence_breaks_equal_time_ties_independently_of_list_order() {
    for reverse in [false, true] {
        let mut conditions = vec![click("late"), click("early")];
        if reverse {
            conditions.reverse();
        }
        let mut t = graph(vec![node("a", any(conditions)), node("b", click("late"))]);
        t.nodes[0].end_conditions = vec![after("a", NodeEvent::Begin, 0)];
        sequence(&mut t, &["a", "b"], at(0));
        let p = plan(&t);
        assert!(matches!(
            p.evaluate(&binding(), time(1), None, &|| false),
            Err(TimelineError::MissingEventHistory)
        ));
        let f = p
            .evaluate(
                &binding(),
                time(1),
                Some(&history(&[(1, "early"), (1, "late")])),
                &|| false,
            )
            .unwrap();
        assert_eq!(frame(&f, "a").start, exact(1));
        assert_eq!(frame(&f, "b").start, exact(1));
        assert_eq!(f.state.event_cursor, 2);
    }
}

#[test]
fn alternatives_cannot_bypass_parent_or_sequence_activation() {
    let mut t = graph(vec![
        node("a", at(0)),
        node("b", any(vec![at(5), click("trigger")])),
    ]);
    sequence(&mut t, &["a", "b"], at(3));
    let p = plan(&t);
    let h = history(&[(1, "trigger"), (4, "trigger"), (6, "trigger")]);
    let f = p
        .evaluate(&binding(), time(20), Some(&h), &|| false)
        .unwrap();
    assert_eq!(frame(&f, "b").start, exact(6));
    let f = p
        .evaluate(
            &binding(),
            time(20),
            Some(&history(&[(1, "trigger"), (4, "trigger")])),
            &|| false,
        )
        .unwrap();
    assert_eq!(frame(&f, "b").start, exact(10));
    t.tree.as_mut().unwrap().containers[0].duration =
        ContainerDuration::Fixed { duration: time(2) };
    let f = plan(&t)
        .evaluate(&binding(), time(20), Some(&h), &|| false)
        .unwrap();
    assert_eq!(frame(&f, "b").start, None);
}

#[test]
fn retained_event_prefixes_and_backward_seeks_reselect_the_eligible_alternative() {
    let t = graph(vec![node("a", any(vec![at(10), click("trigger")]))]);
    let p = plan(&t);
    let mut s = TimelineSampler::new(p.clone());
    let h = history(&[(2, "trigger"), (20, "trigger")]);
    for (at, start) in [(0, 10), (1, 10), (2, 2), (3, 2), (20, 2), (1, 10)] {
        let f = s
            .evaluate(&binding(), time(at), Some(&h), &|| false)
            .unwrap();
        assert_eq!(
            f,
            p.evaluate(&binding(), time(at), Some(&h), &|| false)
                .unwrap()
        );
        assert_eq!(frame(&f, "a").start, exact(start));
    }
    assert_eq!(s.info().schedules_built.get(), 4);
    assert_eq!(s.info().schedules_reused.get(), 2);
}

#[test]
fn validation_checks_every_alternative_and_preserves_old_single_wire_bytes() {
    let legacy = r#"{"kind":"at","offset":{"ticks":"3","timescale":1}}"#;
    let single: StartCondition = serde_json::from_str(legacy).unwrap();
    assert_eq!(serde_json::to_string(&single).unwrap(), legacy);
    assert_eq!(
        serde_json::to_value(StartCondition::from(TimeCondition::Never {})).unwrap(),
        json!({"kind":"never"})
    );
    for value in [
        json!({"kind":"never","delay":0}),
        json!({"kind":"anyOf","conditions":[{"kind":"anyOf","conditions":[{"kind":"never"}]}]}),
    ] {
        assert!(serde_json::from_value::<StartCondition>(value).is_err());
    }
    for conditions in [
        vec![],
        vec![at(0), at(-1)],
        vec![at(0), after("missing", NodeEvent::Begin, 0)],
    ] {
        assert!(
            TimelinePlan::compile(
                &graph(vec![node("a", any(conditions))]),
                TimelineLimits::default(),
                &|| false
            )
            .is_err()
        );
    }
    let t = graph(vec![node(
        "a",
        any(vec![at(0), TimeCondition::Never {}, click("hidden")]),
    )]);
    assert!(matches!(
        TimelinePlan::compile(
            &t,
            TimelineLimits {
                max_conditions: 2,
                ..Default::default()
            },
            &|| false
        ),
        Err(TimelineError::Limit(_))
    ));
    let p = plan(&t);
    assert!(p.targets().contains(&ObjectId::new("hidden").unwrap()));
    assert!(matches!(
        p.evaluate(&binding(), time(1), None, &|| false),
        Err(TimelineError::MissingEventHistory)
    ));
}

#[test]
fn indexed_dependency_notifications_fit_linear_work_budget() {
    let count = 1000;
    let mut nodes: Vec<_> = (0..count).map(|i| node(&format!("s{i}"), at(i))).collect();
    nodes.push(node(
        "target",
        any((0..count)
            .map(|i| after(&format!("s{i}"), NodeEvent::End, 3 * count - 2 * i))
            .collect()),
    ));
    let p = TimelinePlan::compile(
        &graph(nodes),
        TimelineLimits {
            max_schedule_steps: 40_000,
            ..Default::default()
        },
        &|| false,
    )
    .unwrap();
    let f = p.evaluate(&binding(), time(5000), None, &|| false).unwrap();
    assert_eq!(frame(&f, "target").start, exact(2 * count + 3));
}

#[test]
fn cancellation_is_observed_inside_alternative_validation_and_scheduling() {
    let t = graph(vec![node("a", any((0..40).rev().map(at).collect()))]);
    let calls = Cell::new(0);
    let p = TimelinePlan::compile(&t, TimelineLimits::default(), &|| {
        calls.set(calls.get() + 1);
        false
    })
    .unwrap();
    let total = calls.get();
    for cut in 0..total {
        calls.set(0);
        assert!(matches!(
            TimelinePlan::compile(&t, TimelineLimits::default(), &|| {
                let n = calls.get();
                calls.set(n + 1);
                n == cut
            }),
            Err(TimelineError::Cancelled)
        ));
    }
    calls.set(0);
    p.evaluate(&binding(), time(100), None, &|| {
        calls.set(calls.get() + 1);
        false
    })
    .unwrap();
    let total = calls.get();
    for cut in 0..total {
        calls.set(0);
        assert!(matches!(
            p.evaluate(&binding(), time(100), None, &|| {
                let n = calls.get();
                calls.set(n + 1);
                n == cut
            }),
            Err(TimelineError::Cancelled)
        ));
    }
}
