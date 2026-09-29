use mo_common::*;
use mo_timeline::*;
use std::cell::Cell;
fn id(s: &str) -> TimingNodeId {
    TimingNodeId::new(s).unwrap()
}
fn t(n: i64) -> RationalTime {
    RationalTime::new(n, 1).unwrap()
}
fn exact(n: i64, d: i64) -> ExactValue {
    ExactValue {
        numerator: n.to_string(),
        denominator: d.to_string(),
    }
}
fn binding() -> PlaybackBinding {
    PlaybackBinding {
        session: PlaybackSessionId::new("navigation").unwrap(),
        revision: Digest::from_sha256([8; 32]),
        generation: PlaybackGeneration::new(1),
    }
}
fn nav(direction: NavigationDirection, delay: i64) -> TimeCondition {
    TimeCondition::Navigation {
        direction,
        target: None,
        delay: t(delay),
    }
}
fn model(concurrent: bool) -> Timeline {
    Timeline {
        format: TimelineVersion::V02,
        nodes: ["a", "b", "c"]
            .into_iter()
            .map(|name| TimingNode {
                id: id(name),
                restart: RestartMode::Never,
                start: TimeCondition::Never {}.into(),
                end_conditions: vec![],
                duration: t(10),
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
            })
            .collect(),
        tree: Some(TimingTree {
            roots: vec![id("seq")],
            containers: vec![TimingContainer {
                time_transform: None,
                presentation: None,
                id: id("seq"),
                restart: RestartMode::Never,
                kind: ContainerKind::Sequence,
                start: TimeCondition::At { offset: t(0) }.into(),
                end_conditions: vec![],
                duration: ContainerDuration::Indefinite,
                fill: FillMode::Hold,
                children: vec![id("a"), id("b"), id("c")],
                navigation: Some(SequenceNavigation {
                    concurrent,
                    next_action: NextAction::None,
                    previous_action: PreviousAction::None,
                    next_conditions: vec![nav(NavigationDirection::Next, 0)],
                    previous_conditions: vec![nav(NavigationDirection::Previous, 0)],
                }),
            }],
        }),
    }
}
fn history(events: &[(i64, NavigationDirection)]) -> EventHistory {
    EventHistory {
        binding: binding(),
        through: t(100),
        events: events
            .iter()
            .enumerate()
            .map(|(n, (at, d))| PlaybackEvent {
                sequence: n as u32 + 1,
                generation: binding().generation,
                at: t(*at),
                event: InputEvent::Navigation {
                    direction: *d,
                    target: None,
                },
            })
            .collect(),
    }
}
fn frame(model: &Timeline, at: i64, h: &EventHistory) -> EvaluatedFrame {
    TimelinePlan::compile(model, TimelineLimits::default(), &|| false)
        .unwrap()
        .evaluate(&binding(), t(at), Some(h), &|| false)
        .unwrap()
}
fn rotation(f: &EvaluatedFrame, name: &str) -> Option<ExactValue> {
    f.state
        .rotations
        .get(&ObjectId::new(name).unwrap())
        .map(|r| {
            assert_eq!(r.basis, RotationBasis::Absolute);
            r.value()
        })
}
fn controls(m: &mut Timeline) -> &mut SequenceNavigation {
    m.tree.as_mut().unwrap().containers[0]
        .navigation
        .as_mut()
        .unwrap()
}
use NavigationDirection::{Next, Previous};
#[test]
fn next_activates_waiting_slot_then_cuts_off_without_seeking_and_exposes_cursor() {
    let m = model(false);
    let h = history(&[(1, Next), (3, Next)]);
    let f = frame(&m, 4, &h);
    assert_eq!(rotation(&f, "a"), Some(exact(20, 1)));
    assert_eq!(rotation(&f, "b"), Some(exact(10, 1)));
    assert_eq!(rotation(&f, "c"), None);
    assert_eq!(f.state.nodes[0].end, Some(exact(3, 1)));
    assert_eq!(f.state.sequences[0].position, 1);
    assert_eq!(f.state.sequences[0].current, Some(id("b")));
    assert_eq!(frame(&m, 0, &h).state.sequences[0].position, 0);
}
#[test]
fn concurrency_retains_old_child_and_its_completion_cannot_advance_current_cursor() {
    let m = model(true);
    let h = history(&[(1, Next), (3, Next)]);
    assert_eq!(rotation(&frame(&m, 4, &h), "a"), Some(exact(30, 1)));
    let f = frame(&m, 12, &h);
    assert_eq!(f.state.sequences[0].position, 1);
    assert_eq!(f.state.nodes[1].phase, NodePhase::Active);
    assert_eq!(f.state.nodes[2].phase, NodePhase::Waiting);
    assert_eq!(frame(&m, 13, &h).state.sequences[0].position, 2);
}
#[test]
fn previous_resets_same_parent_records_and_later_held_content_then_replays_never() {
    let m = model(false);
    let h = history(&[(1, Next), (3, Next), (5, Previous), (7, Next)]);
    let f = frame(&m, 6, &h);
    assert_eq!(f.state.sequences[0].position, 0);
    assert_eq!(rotation(&f, "a"), Some(exact(10, 1)));
    assert_eq!(rotation(&f, "b"), None);
    assert_eq!(f.state.nodes[1].phase, NodePhase::Waiting);
    let f = frame(&m, 8, &h);
    assert_eq!(rotation(&f, "a"), Some(exact(20, 1)));
    assert_eq!(rotation(&f, "b"), Some(exact(10, 1)));
    // A later query cannot erase earlier activation history or change replay.
    assert_eq!(rotation(&frame(&m, 4, &h), "b"), Some(exact(10, 1)));
}
#[test]
fn navigation_inputs_do_not_alias_clicks_targets_or_reuse_scope_activation_input() {
    let mut m = model(false);
    m.tree.as_mut().unwrap().containers[0].start = nav(Next, 0).into();
    let mut h = history(&[(1, Next), (2, Next)]);
    assert_eq!(frame(&m, 1, &h).state.nodes[0].phase, NodePhase::Waiting);
    assert_eq!(frame(&m, 2, &h).state.nodes[0].start, Some(exact(2, 1)));
    h.events[1].event = InputEvent::Click { target: None };
    assert_eq!(frame(&m, 3, &h).state.nodes[0].phase, NodePhase::Waiting);
    h.events[1].event = InputEvent::Navigation {
        direction: Next,
        target: Some(ObjectId::new("button").unwrap()),
    };
    assert_eq!(frame(&m, 3, &h).state.nodes[0].phase, NodePhase::Waiting);
}
#[test]
fn condition_alternatives_dispatch_once_at_the_earliest_delay_and_distinct_host_sequences_advance()
{
    let mut m = model(false);
    controls(&mut m).next_conditions.push(nav(Next, 1));
    let h = history(&[(1, Next), (1, Next)]);
    let f = frame(&m, 4, &h);
    assert_eq!(f.state.sequences[0].position, 1);
    assert_eq!(f.state.nodes[1].start, Some(exact(1, 1)));
    assert_eq!(f.state.nodes[2].phase, NodePhase::Waiting);
}
#[test]
fn previous_skip_timed_walks_to_a_navigation_only_slot_and_resets_following_timers() {
    let mut m = model(false);
    controls(&mut m).previous_action = PreviousAction::SkipTimed;
    m.nodes[0].duration = t(1);
    m.nodes[1].duration = t(1);
    m.nodes[1].start = TimeCondition::At { offset: t(0) }.into();
    let h = history(&[(1, Next), (4, Previous)]);
    assert_eq!(frame(&m, 3, &h).state.sequences[0].position, 2);
    let f = frame(&m, 4, &h);
    assert_eq!(f.state.sequences[0].position, 0);
    assert_eq!(f.state.nodes[0].start, Some(exact(4, 1)));
    assert_eq!(f.state.nodes[1].phase, NodePhase::Waiting);
}
#[test]
fn natural_completion_and_navigation_completion_close_automatic_sequence_and_do_not_underflow() {
    let mut m = model(false);
    m.tree.as_mut().unwrap().containers[0].duration = ContainerDuration::Automatic;
    let h = history(&[
        (1, Next),
        (2, Next),
        (3, Next),
        (4, Next),
        (5, Next),
        (6, Previous),
    ]);
    let f = frame(&m, 8, &h);
    assert_eq!(f.state.containers[0].end, Some(exact(4, 1)));
    assert_eq!(f.state.sequences[0].position, 3);
    assert_eq!(f.state.sequences[0].current, None);
}
#[test]
fn navigation_is_scoped_and_stale_delayed_commands_do_not_cross_parent_restart() {
    let mut m = model(false);
    controls(&mut m).next_conditions = vec![nav(Next, 5)];
    let seq = &mut m.tree.as_mut().unwrap().containers[0];
    seq.restart = RestartMode::Always;
    seq.start = StartCondition::AnyOf {
        conditions: vec![
            TimeCondition::At { offset: t(0) },
            TimeCondition::At { offset: t(3) },
        ],
    };
    let h = history(&[(1, Next)]);
    let f = frame(&m, 8, &h);
    assert_eq!(f.state.nodes[0].phase, NodePhase::Waiting);
    assert_eq!(f.state.sequences[0].position, 0);
}
#[test]
fn explicit_navigation_has_complete_history_and_strict_contracts() {
    let mut m = model(false);
    let p = TimelinePlan::compile(&m, TimelineLimits::default(), &|| false).unwrap();
    assert!(matches!(
        p.evaluate(&binding(), t(0), None, &|| false),
        Err(TimelineError::MissingEventHistory)
    ));
    let v = serde_json::to_value(&m).unwrap();
    assert_eq!(serde_json::from_value::<Timeline>(v.clone()).unwrap(), m);
    let mut bad = v;
    bad["tree"]["containers"][0]["navigation"]["unknown"] = true.into();
    assert!(serde_json::from_value::<Timeline>(bad).is_err());
    controls(&mut m).next_action = NextAction::Seek;
    assert!(TimelinePlan::compile(&m, TimelineLimits::default(), &|| false).is_ok());
    controls(&mut m).next_action = NextAction::None;
    m.tree.as_mut().unwrap().containers[0].kind = ContainerKind::Parallel;
    assert!(TimelinePlan::compile(&m, TimelineLimits::default(), &|| false).is_err());
}
#[test]
fn backward_sampling_and_every_cancel_checkpoint_leave_retained_state_atomic() {
    let m = model(true);
    let h = history(&[(1, Next), (2, Next), (3, Previous), (4, Next)]);
    let p = TimelinePlan::compile(&m, TimelineLimits::default(), &|| false).unwrap();
    let count = Cell::new(0);
    let expected = p
        .evaluate(&binding(), t(5), Some(&h), &|| {
            count.set(count.get() + 1);
            false
        })
        .unwrap();
    for stop in 0..count.get() {
        let at = Cell::new(0);
        assert!(matches!(
            p.evaluate(&binding(), t(5), Some(&h), &|| {
                let cancel = at.get() == stop;
                at.set(at.get() + 1);
                cancel
            }),
            Err(TimelineError::Cancelled)
        ));
    }
    let mut sampler = TimelineSampler::new(p);
    assert_eq!(
        sampler
            .evaluate(&binding(), t(5), Some(&h), &|| false)
            .unwrap(),
        expected
    );
    let info = sampler.info();
    assert!(matches!(
        sampler.evaluate(&binding(), t(2), Some(&h), &|| true),
        Err(TimelineError::Cancelled)
    ));
    assert_eq!(sampler.info(), info);
    assert_eq!(
        sampler
            .evaluate(&binding(), t(2), Some(&h), &|| false)
            .unwrap(),
        frame(&m, 2, &h)
    );
}

fn seeking(concurrent: bool) -> Timeline {
    let mut m = model(concurrent);
    controls(&mut m).next_action = NextAction::Seek;
    m
}
fn nested_seeking() -> Timeline {
    let mut m = seeking(true);
    for n in &mut m.nodes[..2] {
        n.start = TimeCondition::At { offset: t(0) }.into();
    }
    let tree = m.tree.as_mut().unwrap();
    tree.containers[0].children = vec![id("group"), id("c")];
    tree.containers.push(TimingContainer {
        time_transform: None,
        presentation: None,
        id: id("group"),
        restart: RestartMode::Never,
        kind: ContainerKind::Parallel,
        start: TimeCondition::Never {}.into(),
        end_conditions: vec![],
        duration: ContainerDuration::Indefinite,
        fill: FillMode::Hold,
        children: vec![id("a"), id("b")],
        navigation: None,
    });
    m
}
#[test]
fn seek_finishes_finite_clock_and_successor_starts_at_the_unchanged_host_time() {
    let m = seeking(false);
    let h = history(&[(1, Next), (3, Next)]);
    let f = frame(&m, 4, &h);
    assert_eq!(rotation(&f, "a"), Some(exact(100, 1)));
    assert_eq!(rotation(&f, "b"), Some(exact(10, 1)));
    assert_eq!(f.state.nodes[0].end, Some(exact(3, 1)));
    assert_eq!(f.state.nodes[1].start, Some(exact(3, 1)));
    assert_eq!(f.state.sequences[0].position, 1);
    assert_eq!(rotation(&frame(&m, 2, &h), "a"), Some(exact(10, 1)));
}
#[test]
fn seek_uses_first_infinite_loop_but_concurrency_keeps_the_clock_running() {
    let mut m = seeking(true);
    m.nodes[0].repeat_milli = RepeatCount::Indefinite;
    let h = history(&[(1, Next), (3, Next)]);
    let f = frame(&m, 4, &h);
    assert_eq!(rotation(&f, "a"), Some(exact(10, 1)));
    assert_eq!(f.state.nodes[0].iteration.as_deref(), Some("1"));
    assert_eq!(f.state.nodes[0].phase, NodePhase::Active);
    controls(&mut m).concurrent = false;
    let f = frame(&m, 4, &h);
    assert_eq!(rotation(&f, "a"), Some(exact(100, 1)));
    assert_eq!(f.state.nodes[0].iteration.as_deref(), Some("0"));
    assert_eq!(f.state.nodes[0].phase, NodePhase::Frozen);
}
#[test]
fn seeking_an_infinite_child_past_its_first_loop_never_rewinds_it() {
    let mut m = seeking(true);
    m.nodes[0].repeat_milli = RepeatCount::Indefinite;
    let h = history(&[(1, Next), (15, Next)]);
    assert_eq!(rotation(&frame(&m, 16, &h), "a"), Some(exact(50, 1)));
}
#[test]
fn nested_seek_preserves_finished_child_origin_and_only_advances_its_subtree() {
    let mut m = nested_seeking();
    m.nodes[1].start = TimeCondition::At { offset: t(5) }.into();
    m.nodes[1].duration = t(2);
    let mut outside = m.nodes[0].clone();
    outside.id = id("outside");
    outside.duration = t(100);
    outside.effect = Effect::Rotation {
        composition: Default::default(),
        target: ObjectId::new("outside").unwrap(),
        from: 0,
        to: 100,
    };
    m.nodes.push(outside);
    m.tree.as_mut().unwrap().roots.push(id("outside"));
    let f = frame(&m, 4, &history(&[(1, Next), (3, Next)]));
    assert_eq!(rotation(&f, "a"), Some(exact(100, 1)));
    assert_eq!(rotation(&f, "b"), Some(exact(100, 1)));
    assert_eq!(rotation(&f, "c"), Some(exact(10, 1)));
    assert_eq!(rotation(&f, "outside"), Some(exact(4, 1)));
    assert_eq!(f.state.containers[1].phase, NodePhase::Active);
    assert_eq!(f.state.containers[1].end, None);
    assert_eq!(f.state.nodes[1].start, Some(exact(3, 1)));
    assert_eq!(f.state.nodes[1].end, Some(exact(3, 1)));
}
#[test]
fn internal_end_dependencies_run_during_seek_and_outside_delays_use_host_clock() {
    let mut m = nested_seeking();
    m.nodes[1].start = TimeCondition::After {
        node: id("a"),
        event: NodeEvent::End,
        delay: t(2),
    }
    .into();
    let mut outside = m.nodes[0].clone();
    outside.id = id("outside");
    outside.start = TimeCondition::After {
        node: id("b"),
        event: NodeEvent::End,
        delay: t(2),
    }
    .into();
    outside.effect = Effect::Rotation {
        composition: Default::default(),
        target: ObjectId::new("outside").unwrap(),
        from: 0,
        to: 100,
    };
    m.nodes.push(outside);
    m.tree.as_mut().unwrap().roots.push(id("outside"));
    let h = history(&[(1, Next), (3, Next)]);
    let f = frame(&m, 4, &h);
    assert_eq!(rotation(&f, "a"), Some(exact(100, 1)));
    assert_eq!(rotation(&f, "b"), Some(exact(100, 1)));
    assert_eq!(rotation(&f, "outside"), None);
    assert_eq!(f.state.nodes[3].start, Some(exact(5, 1)));
    assert_eq!(rotation(&frame(&m, 6, &h), "outside"), Some(exact(10, 1)));
}
#[test]
fn seek_respects_reverse_speed_repeat_and_automatic_container_end() {
    let mut m = nested_seeking();
    m.tree.as_mut().unwrap().containers[1].duration = ContainerDuration::Automatic;
    m.nodes[0].time_transform = Some(TimeTransform {
        speed_milli_percent: -200_000,
        ..Default::default()
    });
    m.nodes[1].repeat_milli = RepeatCount::Finite(2500);
    let f = frame(&m, 4, &history(&[(1, Next), (2, Next)]));
    assert_eq!(rotation(&f, "a"), Some(exact(0, 1)));
    assert_eq!(rotation(&f, "b"), Some(exact(50, 1)));
    assert_eq!(f.state.nodes[1].iteration.as_deref(), Some("2"));
    assert_eq!(f.state.containers[1].end, Some(exact(2, 1)));
    assert_eq!(f.state.sequences[0].position, 1);
}
#[test]
fn backward_navigation_replaces_seek_clock_and_all_cancellation_points_are_atomic() {
    let m = seeking(false);
    let h = history(&[(1, Next), (3, Next), (5, Previous)]);
    let f = frame(&m, 6, &h);
    assert_eq!(rotation(&f, "a"), Some(exact(10, 1)));
    assert_eq!(rotation(&f, "b"), None);
    let p = TimelinePlan::compile(&m, TimelineLimits::default(), &|| false).unwrap();
    let count = Cell::new(0);
    let expected = p
        .evaluate(&binding(), t(6), Some(&h), &|| {
            count.set(count.get() + 1);
            false
        })
        .unwrap();
    let mut sampler = TimelineSampler::new(p);
    sampler
        .evaluate(&binding(), t(2), Some(&h), &|| false)
        .unwrap();
    let before = sampler.info();
    for stop in 0..count.get() {
        let n = Cell::new(0);
        assert!(matches!(
            sampler.evaluate(&binding(), t(6), Some(&h), &|| {
                let cancel = n.get() == stop;
                n.set(n.get() + 1);
                cancel
            }),
            Err(TimelineError::Cancelled)
        ));
        assert_eq!(sampler.info(), before);
    }
    assert_eq!(
        sampler
            .evaluate(&binding(), t(6), Some(&h), &|| false)
            .unwrap(),
        expected
    );
}

#[test]
fn cached_event_end_conditions_are_resolved_in_the_receiving_clock() {
    let mut m = nested_seeking();
    m.nodes[1].start = TimeCondition::At { offset: t(5) }.into();
    m.nodes[1].duration = t(20);
    m.nodes[1].end_conditions = vec![TimeCondition::After {
        node: id("a"),
        event: NodeEvent::Begin,
        delay: t(7),
    }];
    let h = history(&[(1, Next), (3, Next)]);
    let f = frame(&m, 4, &h);
    assert_eq!(rotation(&f, "a"), Some(exact(100, 1)));
    assert_eq!(rotation(&f, "b"), Some(exact(10, 1)));
    // This end lies inside an earlier jump, before b's activation, even
    // though both map to the same host timestamp.
    m.nodes[1].end_conditions = vec![TimeCondition::After {
        node: id("a"),
        event: NodeEvent::Begin,
        delay: t(2),
    }];
    let p = TimelinePlan::compile(&m, TimelineLimits::default(), &|| false).unwrap();
    assert!(
        p.evaluate(&binding(), t(4), Some(&h), &|| false)
            .unwrap_err()
            .to_string()
            .contains("precede activation")
    );
}
#[test]
fn nested_navigation_jumps_compose_with_a_later_parent_seek_at_the_same_host_time() {
    let mut m = nested_seeking();
    m.nodes[1].start = TimeCondition::Never {}.into();
    let group = &mut m.tree.as_mut().unwrap().containers[1];
    group.kind = ContainerKind::Sequence;
    group.navigation = Some(SequenceNavigation {
        concurrent: true,
        next_action: NextAction::Seek,
        previous_action: PreviousAction::None,
        next_conditions: vec![TimeCondition::At { offset: t(2) }],
        previous_conditions: vec![],
    });
    let f = frame(&m, 4, &history(&[(1, Next), (3, Next)]));
    assert_eq!(rotation(&f, "a"), Some(exact(100, 1)));
    assert_eq!(rotation(&f, "b"), Some(exact(100, 1)));
    assert_eq!(rotation(&f, "c"), Some(exact(10, 1)));
    assert_eq!(f.state.sequences[0].position, 1);
    assert_eq!(f.state.sequences[1].position, 2);
}
#[test]
fn distinct_inputs_at_one_host_timestamp_each_seek_and_advance_once() {
    let m = seeking(false);
    let f = frame(&m, 2, &history(&[(1, Next), (1, Next), (1, Next)]));
    assert_eq!(rotation(&f, "a"), Some(exact(100, 1)));
    assert_eq!(rotation(&f, "b"), Some(exact(100, 1)));
    assert_eq!(rotation(&f, "c"), Some(exact(10, 1)));
    assert_eq!(f.state.sequences[0].position, 2);
}

#[test]
fn zero_delay_cross_scope_feedback_can_schedule_another_child_during_seek() {
    let mut m = nested_seeking();
    let mut outside = m.nodes[0].clone();
    outside.id = id("outside");
    outside.duration = t(100);
    outside.start = TimeCondition::After {
        node: id("a"),
        event: NodeEvent::End,
        delay: t(0),
    }
    .into();
    outside.effect = Effect::Rotation {
        composition: Default::default(),
        target: ObjectId::new("outside").unwrap(),
        from: 0,
        to: 100,
    };
    m.nodes.push(outside);
    m.tree.as_mut().unwrap().roots.push(id("outside"));
    m.nodes[1].start = TimeCondition::After {
        node: id("outside"),
        event: NodeEvent::Begin,
        delay: t(2),
    }
    .into();
    let f = frame(&m, 4, &history(&[(1, Next), (3, Next)]));
    assert_eq!(rotation(&f, "b"), Some(exact(100, 1)));
    assert_eq!(rotation(&f, "outside"), Some(exact(1, 1)));
}
#[test]
fn causal_reactivation_loops_during_seek_stop_at_the_declared_work_budget() {
    let mut m = nested_seeking();
    m.nodes[0].restart = RestartMode::Always;
    m.nodes[0].start = StartCondition::AnyOf {
        conditions: vec![
            TimeCondition::At { offset: t(0) },
            TimeCondition::After {
                node: id("b"),
                event: NodeEvent::End,
                delay: t(0),
            },
        ],
    };
    m.nodes[1].restart = RestartMode::Always;
    m.nodes[1].start = TimeCondition::After {
        node: id("a"),
        event: NodeEvent::End,
        delay: t(0),
    }
    .into();
    let p = TimelinePlan::compile(
        &m,
        TimelineLimits {
            max_schedule_steps: 1000,
            ..Default::default()
        },
        &|| false,
    )
    .unwrap();
    assert!(matches!(
        p.evaluate(
            &binding(),
            t(4),
            Some(&history(&[(1, Next), (3, Next)])),
            &|| false
        ),
        Err(TimelineError::Limit("timing schedule work"))
    ));
}

#[test]
fn equal_time_end_deadlines_keep_the_earliest_causal_input_sequence() {
    let mut m = model(false);
    m.tree = Some(TimingTree {
        roots: vec![id("a"), id("b"), id("c")],
        containers: vec![],
    });
    m.nodes[0].start = TimeCondition::At { offset: t(0) }.into();
    m.nodes[0].duration = t(20);
    let click = |name: &str, delay| TimeCondition::Click {
        target: Some(ObjectId::new(name).unwrap()),
        delay: t(delay),
    };
    m.nodes[0].end_conditions = vec![
        click("a", 7),
        TimeCondition::After {
            node: id("b"),
            event: NodeEvent::Begin,
            delay: t(5),
        },
    ];
    m.nodes[1].start = click("b", 4).into();
    m.nodes[1].duration = t(5);
    m.nodes[2].start = TimeCondition::After {
        node: id("a"),
        event: NodeEvent::End,
        delay: t(0),
    }
    .into();
    m.nodes[2].end_conditions = vec![TimeCondition::After {
        node: id("b"),
        event: NodeEvent::End,
        delay: t(0),
    }];
    let mut h = history(&[(1, Next), (3, Next)]);
    h.events[0].event = InputEvent::Click {
        target: Some(ObjectId::new("b").unwrap()),
    };
    h.events[1].event = InputEvent::Click {
        target: Some(ObjectId::new("a").unwrap()),
    };
    let f = frame(&m, 10, &h);
    assert_eq!(f.state.nodes[2].phase, NodePhase::Frozen);
    assert_eq!(f.state.nodes[2].start, Some(exact(10, 1)));
    assert_eq!(f.state.nodes[2].end, Some(exact(10, 1)));
}
