use NavigationDirection::{Next, Previous};
use mo_common::*;
use mo_timeline::*;
use std::cell::Cell;

fn time(ms: i64) -> RationalTime {
    RationalTime::new(ms, 1000).unwrap()
}
fn binding() -> PlaybackBinding {
    PlaybackBinding {
        session: PlaybackSessionId::new("step").unwrap(),
        revision: Digest::from_sha256([7; 32]),
        generation: PlaybackGeneration::new(1),
    }
}
fn effect(name: &str) -> PresentationEffect {
    PresentationEffect {
        id: TimingNodeId::new(name).unwrap(),
        delay: time(0),
        duration: time(1000),
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
fn sequence() -> Timeline {
    PresentationSequence {
        groups: ["a", "b"]
            .into_iter()
            .map(|name| PresentationGroup {
                start: PresentationGroupStart::Next,
                batches: vec![PresentationBatch {
                    delay: time(0),
                    effects: vec![effect(name)],
                }],
            })
            .collect(),
    }
    .compile(TimelineLimits::default(), &|| false)
    .unwrap()
}
fn history(steps: &[(i64, NavigationDirection)]) -> EventHistory {
    EventHistory {
        binding: binding(),
        through: time(100_000),
        events: steps
            .iter()
            .enumerate()
            .map(|(index, &(at, direction))| PlaybackEvent {
                generation: binding().generation,
                sequence: index as u32 + 1,
                at: time(at),
                event: InputEvent::PresentationStep { direction },
            })
            .collect(),
    }
}
fn plan(timeline: &Timeline) -> TimelinePlan {
    TimelinePlan::compile(timeline, TimelineLimits::default(), &|| false).unwrap()
}
fn frame(timeline: &Timeline, at: i64, steps: &[(i64, NavigationDirection)]) -> EvaluatedFrame {
    plan(timeline)
        .evaluate(&binding(), time(at), Some(&history(steps)), &|| false)
        .unwrap()
}
fn receipt(frame: &EvaluatedFrame, sequence: u32, direction: NavigationDirection, consumed: bool) {
    let value = frame.state.presentation_step.as_ref().unwrap();
    assert_eq!(value.sequence, sequence);
    assert_eq!(value.direction, direction);
    assert_eq!(
        value.outcome,
        if consumed {
            PresentationStepOutcome::Consumed
        } else {
            PresentationStepOutcome::PageBoundary {
                entry: PresentationPageEntry::Initial,
            }
        }
    );
    assert_eq!(
        frame.sha256,
        digest(&frame.state.profile, &frame.state).unwrap()
    );
}

#[test]
fn empty_page_reports_the_exact_requested_boundary_without_inventing_an_animation() {
    let timeline = Timeline {
        format: TimelineVersion::V02,
        nodes: vec![],
        tree: Some(TimingTree {
            roots: vec![],
            containers: vec![],
        }),
    };
    assert!(frame(&timeline, 0, &[]).state.presentation_step.is_none());
    for direction in [Next, Previous] {
        let value = frame(&timeline, 100, &[(100, direction)]);
        receipt(&value, 1, direction, false);
        assert_eq!(
            value.state.presentation_step.unwrap().at,
            ExactValue {
                numerator: "1".into(),
                denominator: "10".into()
            }
        );
    }
}

#[test]
fn sequence_steps_seek_then_leave_and_previous_undoes_groups_before_leaving() {
    let timeline = sequence();
    let events = [
        (100, Next),
        (200, Next),
        (300, Next),
        (400, Next),
        (500, Previous),
        (600, Previous),
        (700, Previous),
    ];
    for index in 0..events.len() {
        let (at, direction) = events[index];
        let value = frame(&timeline, at, &events[..=index]);
        receipt(
            &value,
            index as u32 + 1,
            direction,
            ![3, 6].contains(&index),
        );
        // The new presentation gesture has exactly the same geometry/cursor
        // as the corresponding native navigation event on this document.
        let mut native = history(&events[..=index]);
        for event in &mut native.events {
            let InputEvent::PresentationStep { direction } = event.event else {
                unreachable!()
            };
            event.event = InputEvent::Navigation {
                direction,
                target: None,
            };
        }
        let raw = plan(&timeline)
            .evaluate(&binding(), time(at), Some(&native), &|| false)
            .unwrap();
        assert_eq!(value.state.rotations, raw.state.rotations);
        assert_eq!(value.state.sequences, raw.state.sequences);
        assert!(raw.state.presentation_step.is_none());
    }
}

#[test]
fn equal_timestamp_steps_use_event_order_and_only_the_latest_receipt_is_retained() {
    let timeline = sequence();
    let events = [(100, Next); 4];
    let mut log = history(&events);
    log.events.insert(1, log.events[0].clone());
    let value = plan(&timeline)
        .evaluate(&binding(), time(100), Some(&log), &|| false)
        .unwrap();
    receipt(&value, 4, Next, false);
    assert_eq!(value.state.event_cursor, 4);
    assert_eq!(value.state.sequences[0].position, 2);
    assert_eq!(
        serde_json::to_value(value.state).unwrap()["presentationStep"]["sequence"],
        4
    );
}

fn click_timeline(target: Option<ObjectId>, delay: i64) -> Timeline {
    Timeline {
        format: TimelineVersion::V01,
        tree: None,
        nodes: vec![TimingNode {
            id: TimingNodeId::new("click").unwrap(),
            restart: RestartMode::Never,
            start: TimeCondition::Click {
                target,
                delay: time(delay),
            }
            .into(),
            end_conditions: vec![],
            duration: time(1000),
            repeat_milli: 1000.into(),
            repeat_duration: None,
            fill: FillMode::Hold,
            time_transform: None,
            effect: effect("a").effect,
        }],
    }
}

#[test]
fn forward_gesture_routes_global_clicks_but_never_targeted_clicks_or_previous() {
    let global = click_timeline(None, 0);
    receipt(&frame(&global, 100, &[(100, Next)]), 1, Next, true);
    receipt(&frame(&global, 100, &[(100, Previous)]), 1, Previous, false);
    receipt(
        &frame(&global, 200, &[(100, Next), (200, Next)]),
        2,
        Next,
        false,
    );
    let targeted = click_timeline(Some(ObjectId::new("button").unwrap()), 0);
    let value = frame(&targeted, 100, &[(100, Next)]);
    receipt(&value, 1, Next, false);
    assert!(value.state.rotations.is_empty());
}

#[test]
fn delayed_admission_is_consumed_without_claiming_future_events_or_replaying_raw_clicks() {
    let timeline = click_timeline(None, 1000);
    let log = history(&[(100, Next), (1200, Next)]);
    let mut sampler = TimelineSampler::new(plan(&timeline));
    let first = sampler
        .evaluate(&binding(), time(100), Some(&log), &|| false)
        .unwrap();
    receipt(&first, 1, Next, true);
    assert_eq!(first.state.nodes[0].phase, NodePhase::Scheduled);
    let later = sampler
        .evaluate(&binding(), time(1100), Some(&log), &|| false)
        .unwrap();
    receipt(&later, 1, Next, true);
    assert_eq!(later.state.nodes[0].phase, NodePhase::Active);
    assert_eq!(sampler.info().schedules_reused.get(), 1);
    receipt(
        &sampler
            .evaluate(&binding(), time(1200), Some(&log), &|| false)
            .unwrap(),
        2,
        Next,
        false,
    );
    // Seeking backward selects the original prefix, not the last cached receipt.
    assert_eq!(
        sampler
            .evaluate(&binding(), time(100), Some(&log), &|| false)
            .unwrap(),
        first
    );
    let mut raw = log.clone();
    raw.events.truncate(1);
    raw.events[0].event = InputEvent::Click { target: None };
    let value = sampler
        .evaluate(&binding(), time(100), Some(&raw), &|| false)
        .unwrap();
    assert!(value.state.presentation_step.is_none());
    assert_eq!(value.state.nodes, first.state.nodes);
}

#[test]
fn combined_conditions_dispatch_from_one_pre_event_state_and_do_not_double_activate() {
    let mut timeline = click_timeline(None, 0);
    timeline.nodes[0].restart = RestartMode::Always;
    timeline.nodes[0].start = StartCondition::AnyOf {
        conditions: vec![
            TimeCondition::Click {
                target: None,
                delay: time(0),
            },
            TimeCondition::Navigation {
                direction: Next,
                target: None,
                delay: time(0),
            },
        ],
    };
    let mut sampler = TimelineSampler::new(plan(&timeline));
    let log = history(&[(100, Next), (200, Next)]);
    let value = sampler
        .evaluate(&binding(), time(200), Some(&log), &|| false)
        .unwrap();
    receipt(&value, 2, Next, true);
    assert_eq!(sampler.info().retained_intervals.get(), 2);
}

#[test]
fn eligible_end_conditions_consume_once_and_closed_scopes_do_not_consume() {
    let mut timeline = click_timeline(None, 0);
    timeline.nodes[0].start = TimeCondition::At { offset: time(0) }.into();
    timeline.nodes[0].end_conditions = vec![TimeCondition::Click {
        target: None,
        delay: time(0),
    }];
    let ended = frame(&timeline, 100, &[(100, Next)]);
    receipt(&ended, 1, Next, true);
    assert_eq!(ended.state.nodes[0].phase, NodePhase::Frozen);
    receipt(
        &frame(&timeline, 200, &[(100, Next), (200, Next)]),
        2,
        Next,
        false,
    );

    let mut timeline = sequence();
    let main = timeline
        .tree
        .as_mut()
        .unwrap()
        .containers
        .iter_mut()
        .find(|node| node.presentation == Some(PresentationRole::MainSequence))
        .unwrap();
    main.end_conditions = vec![TimeCondition::At { offset: time(250) }];
    let events = [(100, Next), (300, Next), (400, Previous)];
    receipt(&frame(&timeline, 100, &events[..1]), 1, Next, true);
    receipt(&frame(&timeline, 300, &events[..2]), 2, Next, false);
    receipt(&frame(&timeline, 400, &events), 3, Previous, false);
}

#[test]
fn concurrent_global_conditions_share_one_step_and_targeted_navigation_is_not_routed() {
    let mut timeline = click_timeline(None, 0);
    let mut second = timeline.nodes[0].clone();
    second.id = TimingNodeId::new("second").unwrap();
    second.effect = effect("b").effect;
    timeline.nodes.push(second);
    let value = frame(&timeline, 200, &[(100, Next)]);
    receipt(&value, 1, Next, true);
    assert_eq!(value.state.rotations.len(), 2);
    receipt(
        &frame(&timeline, 200, &[(100, Next), (200, Next)]),
        2,
        Next,
        false,
    );
    for node in &mut timeline.nodes {
        node.start = TimeCondition::Navigation {
            direction: Next,
            target: Some(ObjectId::new("button").unwrap()),
            delay: time(0),
        }
        .into();
    }
    let value = frame(&timeline, 200, &[(100, Next)]);
    receipt(&value, 1, Next, false);
    assert!(value.state.rotations.is_empty());
}

#[test]
fn later_raw_inputs_do_not_reattribute_the_last_step_receipt() {
    let timeline = click_timeline(Some(ObjectId::new("button").unwrap()), 0);
    let mut log = history(&[(100, Next)]);
    log.events.push(PlaybackEvent {
        generation: binding().generation,
        sequence: 2,
        at: time(200),
        event: InputEvent::Click {
            target: Some(ObjectId::new("button").unwrap()),
        },
    });
    let value = plan(&timeline)
        .evaluate(&binding(), time(200), Some(&log), &|| false)
        .unwrap();
    receipt(&value, 1, Next, false);
    assert_eq!(value.state.event_cursor, 2);
    assert_eq!(value.state.nodes[0].phase, NodePhase::Active);
}

#[test]
fn boundary_steps_do_not_schedule_delayed_navigation_into_a_later_reset() {
    let mut timeline = sequence();
    let main = timeline
        .tree
        .as_mut()
        .unwrap()
        .containers
        .iter_mut()
        .find(|node| node.presentation == Some(PresentationRole::MainSequence))
        .unwrap();
    let TimeCondition::Navigation { delay, .. } =
        &mut main.navigation.as_mut().unwrap().next_conditions[0]
    else {
        unreachable!()
    };
    *delay = time(50);
    let events = [
        (100, Next),
        (300, Next),
        (500, Next),
        (700, Next),
        (710, Previous),
    ];
    receipt(&frame(&timeline, 700, &events[..4]), 4, Next, false);
    let value = frame(&timeline, 760, &events);
    receipt(&value, 5, Previous, true);
    assert_eq!(value.state.sequences[0].position, 1);
    assert!(value.state.sequences[0].current.is_some());
}

#[test]
fn stale_generation_invalid_future_history_and_all_cancellation_points_cannot_return_receipts() {
    let timeline = sequence();
    let p = plan(&timeline);
    let log = history(&[(100, Next)]);
    let calls = Cell::new(0);
    p.evaluate(&binding(), time(100), Some(&log), &|| {
        calls.set(calls.get() + 1);
        false
    })
    .unwrap();
    for stop in 1..=calls.get() {
        let current = Cell::new(0);
        assert!(matches!(
            p.evaluate(&binding(), time(100), Some(&log), &|| {
                current.set(current.get() + 1);
                current.get() == stop
            }),
            Err(TimelineError::Cancelled)
        ));
    }
    let mut bad = log.clone();
    bad.events[0].generation = PlaybackGeneration::new(2);
    assert!(
        p.evaluate(&binding(), time(100), Some(&bad), &|| false)
            .is_err()
    );
    let mut future = log.clone();
    future.events.push(PlaybackEvent {
        sequence: 3,
        at: time(200),
        ..log.events[0].clone()
    });
    assert!(
        p.evaluate(&binding(), time(100), Some(&future), &|| false)
            .is_err()
    );
    let bounded = TimelineLimits {
        max_events: 0,
        ..TimelineLimits::default()
    };
    let bounded = TimelinePlan::compile(&timeline, bounded, &|| false).unwrap();
    assert!(
        bounded
            .evaluate(&binding(), time(100), Some(&log), &|| false)
            .is_err()
    );
}
