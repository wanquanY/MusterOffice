use mo_common::*;
use mo_timeline::*;
use std::cell::Cell;

fn t(n: i64) -> RationalTime {
    RationalTime::new(n, 1).unwrap()
}
fn id(s: &str) -> TimingNodeId {
    TimingNodeId::new(s).unwrap()
}
fn at(n: i64) -> TimeCondition {
    TimeCondition::At { offset: t(n) }
}
fn click(target: &str, delay: i64) -> TimeCondition {
    TimeCondition::Click {
        target: Some(ObjectId::new(target).unwrap()),
        delay: t(delay),
    }
}
fn after(node: &str, event: NodeEvent, delay: i64) -> TimeCondition {
    TimeCondition::After {
        node: id(node),
        event,
        delay: t(delay),
    }
}
fn any(conditions: Vec<TimeCondition>) -> StartCondition {
    StartCondition::AnyOf { conditions }
}
fn node(name: &str, restart: RestartMode, start: impl Into<StartCondition>) -> TimingNode {
    TimingNode {
        id: id(name),
        restart,
        start: start.into(),
        duration: t(2),
        repeat_milli: 1000.into(),
        repeat_duration: None,
        end_conditions: vec![],
        time_transform: None,
        fill: FillMode::Hold,
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
fn parent(timeline: &mut Timeline, start: impl Into<StartCondition>, duration: ContainerDuration) {
    timeline.format = TimelineVersion::V02;
    timeline.tree = Some(TimingTree {
        roots: vec![id("parent")],
        containers: vec![TimingContainer {
            time_transform: None,
            presentation: None,
            navigation: None,
            id: id("parent"),
            restart: RestartMode::Always,
            kind: ContainerKind::Parallel,
            start: start.into(),
            duration,
            fill: FillMode::Hold,
            end_conditions: vec![],
            children: timeline.nodes.iter().map(|n| n.id.clone()).collect(),
        }],
    });
}
fn binding() -> PlaybackBinding {
    PlaybackBinding {
        session: PlaybackSessionId::new("restart").unwrap(),
        revision: Digest::from_sha256([3; 32]),
        generation: PlaybackGeneration::new(1),
    }
}
fn history(entries: &[(i64, &str)]) -> EventHistory {
    EventHistory {
        binding: binding(),
        through: t(100),
        events: entries
            .iter()
            .enumerate()
            .map(|(i, (time, target))| PlaybackEvent {
                generation: binding().generation,
                sequence: i as u32 + 1,
                at: t(*time),
                event: InputEvent::Click {
                    target: Some(ObjectId::new(*target).unwrap()),
                },
            })
            .collect(),
    }
}
fn exact(n: i64, d: i64) -> Option<ExactValue> {
    Some(ExactValue {
        numerator: n.to_string(),
        denominator: d.to_string(),
    })
}
fn plan(timeline: &Timeline) -> TimelinePlan {
    TimelinePlan::compile(timeline, TimelineLimits::default(), &|| false).unwrap()
}
fn frame(timeline: &Timeline, n: i64, d: u32, h: Option<&EventHistory>) -> EvaluatedFrame {
    plan(timeline)
        .evaluate(&binding(), RationalTime::new(n, d).unwrap(), h, &|| false)
        .unwrap()
}

#[test]
fn admission_policies_are_distinct_and_natural_end_allows_reentry() {
    let h = history(&[(0, "go"), (1, "go"), (3, "go")]);
    for (mode, start, progress) in [
        (RestartMode::Never, 0, 3),
        (RestartMode::Always, 1, 1),
        (RestartMode::WhenNotActive, 0, 3),
    ] {
        let timeline = graph(vec![node("a", mode, click("go", 0))]);
        let f = frame(&timeline, 3, 2, Some(&h));
        assert_eq!(f.state.nodes[0].start, exact(start, 1));
        assert_eq!(f.state.nodes[0].progress, exact(progress, 4));
        let f = frame(&timeline, 7, 2, Some(&h));
        assert_eq!(
            f.state.nodes[0].start,
            exact(if mode == RestartMode::Never { 0 } else { 3 }, 1)
        );
    }
    let timeline = graph(vec![node(
        "a",
        RestartMode::WhenNotActive,
        any(vec![at(0), at(2)]),
    )]);
    let f = frame(&timeline, 2, 1, None);
    assert_eq!(f.state.nodes[0].start, exact(2, 1));
    assert_eq!(f.state.nodes[0].progress, exact(0, 1));
}

#[test]
fn parent_reactivation_resets_never_children_and_preserves_clipped_end_events() {
    let mut timeline = graph(vec![node("a", RestartMode::Never, at(1))]);
    timeline.nodes[0].duration = t(4);
    parent(
        &mut timeline,
        any(vec![at(0), at(2)]),
        ContainerDuration::Fixed { duration: t(3) },
    );
    timeline.nodes.push(node(
        "observer",
        RestartMode::Never,
        after("a", NodeEvent::End, 0),
    ));
    timeline.tree.as_mut().unwrap().roots.push(id("observer"));
    let f = frame(&timeline, 5, 2, None);
    assert_eq!(f.state.nodes[0].start, exact(3, 1));
    assert_eq!(f.state.nodes[0].phase, NodePhase::Scheduled);
    assert_eq!(f.state.nodes[1].start, exact(2, 1));
    let f = frame(&timeline, 4, 1, None);
    assert_eq!(f.state.nodes[0].start, exact(3, 1));
    assert_eq!(f.state.nodes[0].end, exact(5, 1));
    assert_eq!(f.state.nodes[0].progress, exact(1, 4));
    let f = frame(&timeline, 6, 1, None);
    assert_eq!(f.state.nodes[0].progress, exact(1, 2));
}

#[test]
fn parent_reset_invalidates_old_delayed_clicks_and_does_not_reuse_the_parent_click() {
    let mut timeline = graph(vec![node("a", RestartMode::Never, click("child", 4))]);
    parent(
        &mut timeline,
        any(vec![at(0), at(2)]),
        ContainerDuration::Fixed { duration: t(10) },
    );
    let h = history(&[(1, "child"), (3, "child")]);
    let f = frame(&timeline, 5, 1, Some(&h));
    assert_eq!(f.state.nodes[0].start, exact(7, 1));
    assert_eq!(f.state.nodes[0].phase, NodePhase::Scheduled);
    timeline.nodes[0].start = click("go", 0).into();
    timeline.tree.as_mut().unwrap().containers[0].start = click("go", 0).into();
    let f = frame(&timeline, 3, 1, Some(&history(&[(0, "go"), (1, "go")])));
    assert_eq!(f.state.nodes[0].phase, NodePhase::Waiting);
    assert_eq!(f.state.containers[0].start, exact(1, 1));
}

#[test]
fn toggle_and_begin_end_notifications_follow_each_actual_occurrence() {
    let mut a = node("a", RestartMode::WhenNotActive, click("go", 0));
    a.duration = t(10);
    a.end_conditions = vec![click("go", 0)];
    let timeline = graph(vec![a]);
    let h = history(&[(0, "go"), (1, "go"), (2, "go")]);
    let f = frame(&timeline, 3, 2, Some(&h));
    assert_eq!(f.state.nodes[0].start, exact(0, 1));
    assert_eq!(f.state.nodes[0].end, exact(1, 1));
    let f = frame(&timeline, 5, 2, Some(&h));
    assert_eq!(f.state.nodes[0].start, exact(2, 1));
    let timeline = graph(vec![
        node("a", RestartMode::Always, click("go", 0)),
        node(
            "begin",
            RestartMode::Always,
            after("a", NodeEvent::Begin, 0),
        ),
        node("end", RestartMode::Always, after("a", NodeEvent::End, 0)),
    ]);
    let f = frame(&timeline, 5, 1, Some(&h));
    assert_eq!(f.state.nodes[1].start, exact(2, 1));
    assert_eq!(f.state.nodes[2].start, exact(4, 1));
}

#[test]
fn repeated_children_update_automatic_completion_and_parent_restart_resets_sequences() {
    let mut timeline = graph(vec![
        node("a", RestartMode::Always, click("go", 0)),
        node("b", RestartMode::Never, at(0)),
    ]);
    timeline.nodes[0].duration = t(1);
    timeline.nodes[1].duration = t(5);
    parent(&mut timeline, at(0), ContainerDuration::Automatic);
    let f = frame(&timeline, 6, 1, Some(&history(&[(0, "go"), (3, "go")])));
    assert_eq!(f.state.containers[0].end, exact(5, 1));
    assert_eq!(f.state.nodes[0].start, exact(3, 1));
    timeline.nodes[0].start = at(0).into();
    timeline.nodes[0].restart = RestartMode::Never;
    timeline.nodes[1].duration = t(1);
    let c = &mut timeline.tree.as_mut().unwrap().containers[0];
    c.kind = ContainerKind::Sequence;
    c.start = any(vec![at(0), at(4)]);
    let f = frame(&timeline, 6, 1, None);
    assert_eq!(f.state.nodes[0].start, exact(4, 1));
    assert_eq!(f.state.nodes[1].start, exact(5, 1));
    assert_eq!(f.state.containers[0].end, exact(6, 1));
    timeline.nodes[0].restart = RestartMode::Always;
    assert!(TimelinePlan::compile(&timeline, TimelineLimits::default(), &|| false).is_err());
}

#[test]
fn feedback_is_horizon_bounded_and_never_confused_with_repeat_clock_iterations() {
    let mut a = node(
        "a",
        RestartMode::Always,
        any(vec![at(0), after("a", NodeEvent::End, 1)]),
    );
    a.repeat_milli = 2000.into();
    let timeline = graph(vec![a]);
    let f = frame(&timeline, 13, 1, None);
    assert_eq!(f.state.nodes[0].start, exact(10, 1));
    assert_eq!(f.state.nodes[0].iteration.as_deref(), Some("1"));
    assert_eq!(f.state.nodes[0].progress, exact(1, 2));
    let limits = TimelineLimits {
        max_intervals: 2,
        ..Default::default()
    };
    let p = TimelinePlan::compile(&timeline, limits, &|| false).unwrap();
    assert!(matches!(
        p.evaluate(&binding(), t(13), None, &|| false),
        Err(TimelineError::Limit("timing interval count"))
    ));
}

#[test]
fn retained_rebuilds_cover_horizon_backwards_and_leave_no_partial_state_on_failure() {
    let timeline = graph(vec![node(
        "a",
        RestartMode::Always,
        any(vec![at(0), at(2), at(4)]),
    )]);
    let p = plan(&timeline);
    let mut sampler = TimelineSampler::new(p.clone());
    for at in [5, 1, 3, 3, 0, 8] {
        assert_eq!(
            sampler
                .evaluate(&binding(), t(at), None, &|| false)
                .unwrap(),
            p.evaluate(&binding(), t(at), None, &|| false).unwrap()
        );
    }
    assert_eq!(sampler.info().schedules_built.get(), 5);
    assert_eq!(sampler.info().schedules_reused.get(), 1);
    let before = sampler.info();
    let calls = Cell::new(0);
    assert!(matches!(
        sampler.evaluate(&binding(), t(9), None, &|| {
            calls.set(calls.get() + 1);
            calls.get() > 15
        }),
        Err(TimelineError::Cancelled)
    ));
    assert_eq!(sampler.info(), before);
    assert_eq!(
        sampler.evaluate(&binding(), t(8), None, &|| false).unwrap(),
        p.evaluate(&binding(), t(8), None, &|| false).unwrap()
    );
}

#[test]
fn restart_wire_is_strict_and_default_bytes_stay_compatible() {
    let n = node("a", RestartMode::Never, at(0));
    let mut v = serde_json::to_value(&n).unwrap();
    assert!(v.get("restart").is_none());
    assert_eq!(serde_json::from_value::<TimingNode>(v.clone()).unwrap(), n);
    v["restart"] = serde_json::json!("always");
    assert_eq!(
        serde_json::from_value::<TimingNode>(v.clone())
            .unwrap()
            .restart,
        RestartMode::Always
    );
    v["restart"] = serde_json::json!("invalid");
    assert!(serde_json::from_value::<TimingNode>(v).is_err());
}

#[test]
fn delayed_events_use_event_time_sensitivity_and_restart_discards_prior_instances() {
    let h = history(&[(1, "go"), (3, "go")]);
    let timeline = graph(vec![node(
        "a",
        RestartMode::WhenNotActive,
        any(vec![at(0), click("go", 4)]),
    )]);
    // The event at 1 occurs while active; its delayed begin at 5 is ignored.
    let f = frame(&timeline, 6, 1, Some(&h));
    assert_eq!(f.state.nodes[0].start, exact(0, 1));
    let f = frame(&timeline, 8, 1, Some(&h));
    assert_eq!(f.state.nodes[0].start, exact(7, 1));
    let timeline = graph(vec![node(
        "a",
        RestartMode::Always,
        any(vec![at(0), at(2), click("go", 4)]),
    )]);
    let f = frame(&timeline, 6, 1, Some(&h));
    assert_eq!(f.state.nodes[0].start, exact(2, 1));
    let f = frame(&timeline, 8, 1, Some(&h));
    assert_eq!(f.state.nodes[0].start, exact(7, 1));
}

#[test]
fn shared_delayed_begin_end_event_never_ends_the_old_interval_when_restart_wins() {
    let mut a = node("a", RestartMode::Always, any(vec![at(0), click("go", 4)]));
    a.duration = t(20);
    a.end_conditions = vec![click("go", 0)];
    let timeline = graph(vec![a]);
    let h = history(&[(1, "go")]);
    let f = frame(&timeline, 3, 1, Some(&h));
    assert_eq!(f.state.nodes[0].phase, NodePhase::Active);
    assert_eq!(f.state.nodes[0].start, exact(0, 1));
    let f = frame(&timeline, 6, 1, Some(&h));
    assert_eq!(f.state.nodes[0].start, exact(5, 1));
    assert_eq!(f.state.nodes[0].end, exact(25, 1));
}

#[test]
fn known_future_bounds_never_schedule_an_entry_outside_its_parent() {
    let mut a = node("a", RestartMode::Always, at(5));
    a.end_conditions = vec![at(6)];
    let mut timeline = graph(vec![a]);
    let f = frame(&timeline, 0, 1, None);
    assert_eq!(f.state.nodes[0].start, exact(5, 1));
    assert_eq!(f.state.nodes[0].end, exact(6, 1));
    timeline.nodes[0].restart = RestartMode::Never;
    parent(
        &mut timeline,
        at(0),
        ContainerDuration::Fixed { duration: t(3) },
    );
    let f = frame(&timeline, 1, 1, None);
    assert_eq!(f.state.nodes[0].start, None);
    let f = frame(&timeline, 4, 1, None);
    assert_eq!(f.state.nodes[0].phase, NodePhase::Suppressed);
}

#[test]
fn event_dispatch_work_and_retained_history_have_independent_limits() {
    let timeline = graph(vec![node("a", RestartMode::Always, click("go", 0))]);
    let h = history(&(0..100).map(|n| (n, "go")).collect::<Vec<_>>());
    let limits = TimelineLimits {
        max_schedule_steps: 100,
        max_intervals: 1000,
        ..Default::default()
    };
    let p = TimelinePlan::compile(&timeline, limits, &|| false).unwrap();
    assert!(matches!(
        p.evaluate(&binding(), t(99), Some(&h), &|| false),
        Err(TimelineError::Limit("timing schedule work"))
    ));
    let mut sampler = TimelineSampler::new(plan(&timeline));
    sampler
        .evaluate(&binding(), t(99), Some(&h), &|| false)
        .unwrap();
    assert_eq!(sampler.info().retained_intervals.get(), 100);
}

#[test]
fn same_timestamp_reactivation_uses_event_order_before_author_order() {
    let a = node("a", RestartMode::Always, click("later", 0));
    let mut b = node("b", RestartMode::Never, click("first", 0));
    b.effect = Effect::Rotation {
        composition: Default::default(),
        target: ObjectId::new("a").unwrap(),
        from: 500,
        to: 600,
    };
    let timeline = graph(vec![a, b]);
    let h = history(&[(0, "first"), (0, "later"), (0, "later")]);
    let mut sampler = TimelineSampler::new(plan(&timeline));
    let f = sampler
        .evaluate(&binding(), t(0), Some(&h), &|| false)
        .unwrap();
    assert_eq!(
        f.state.rotations[&ObjectId::new("a").unwrap()].value(),
        exact(0, 1).unwrap()
    );
    assert_eq!(sampler.info().retained_intervals.get(), 3);
}

#[test]
fn every_restart_checkpoint_preserves_the_previous_cache_on_cancellation() {
    let mut timeline = graph(vec![node("a", RestartMode::Never, at(0))]);
    parent(
        &mut timeline,
        any(vec![at(0), at(1), at(3)]),
        ContainerDuration::Fixed { duration: t(2) },
    );
    let p = plan(&timeline);
    let prepare = || {
        let mut s = TimelineSampler::new(p.clone());
        s.evaluate(&binding(), t(0), None, &|| false).unwrap();
        s
    };
    let calls = Cell::new(0);
    let expected = prepare()
        .evaluate(&binding(), t(4), None, &|| {
            calls.set(calls.get() + 1);
            false
        })
        .unwrap();
    for checkpoint in 0..calls.get() {
        let mut s = prepare();
        let before = s.info();
        let current = Cell::new(0);
        assert!(
            matches!(
                s.evaluate(&binding(), t(4), None, &|| {
                    let stop = current.get() == checkpoint;
                    current.set(current.get() + 1);
                    stop
                }),
                Err(TimelineError::Cancelled)
            ),
            "checkpoint {checkpoint}"
        );
        assert_eq!(s.info(), before);
        assert_eq!(
            s.evaluate(&binding(), t(4), None, &|| false).unwrap(),
            expected
        );
    }
}

#[test]
fn same_moment_external_event_before_parent_reset_cannot_enter_the_new_scope() {
    let mut timeline = graph(vec![node(
        "child",
        RestartMode::Never,
        after("outside", NodeEvent::End, 0),
    )]);
    parent(
        &mut timeline,
        any(vec![at(0), at(2)]),
        ContainerDuration::Fixed { duration: t(3) },
    );
    timeline
        .nodes
        .push(node("outside", RestartMode::Never, at(0)));
    timeline.tree.as_mut().unwrap().roots.push(id("outside"));
    let f = frame(&timeline, 3, 1, None);
    assert_eq!(f.state.containers[0].start, exact(2, 1));
    assert_eq!(f.state.nodes[0].phase, NodePhase::Waiting);
    assert_eq!(f.state.nodes[0].start, None);
    // The new parent's own begin happens after reset and is observable.
    timeline.nodes[0].start = after("parent", NodeEvent::Begin, 0).into();
    let f = frame(&timeline, 3, 1, None);
    assert_eq!(f.state.nodes[0].start, exact(2, 1));
    assert_eq!(f.state.nodes[0].phase, NodePhase::Active);
}
