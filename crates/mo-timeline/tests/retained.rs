use mo_common::*;
use mo_timeline::*;
use serde_json::json;
use std::cell::Cell;

fn t(n: i64, d: u32) -> RationalTime {
    RationalTime::new(n, d).unwrap()
}
fn binding() -> PlaybackBinding {
    PlaybackBinding {
        session: PlaybackSessionId::new("retained").unwrap(),
        revision: Digest::from_sha256([1; 32]),
        generation: PlaybackGeneration::new(4),
    }
}
fn graph() -> Timeline {
    serde_json::from_value(json!({
        "format":"musteroffice.timeline/0.2-draft",
        "nodes":[
          {"id":"a","start":{"kind":"click","target":null,"delay":t(0,1)},"duration":t(2,1),"repeatMilli":"indefinite","fill":"hold","effect":{"kind":"rotation","target":"shape","from":0,"to":120}},
          {"id":"b","start":{"kind":"after","node":"a","event":"end","delay":t(1,3)},"duration":t(2,1),"repeatMilli":2500,"fill":"freeze","effect":{"kind":"rotation","target":"other","from":40,"to":-80}}
        ],
        "tree":{"roots":["parent","b"],"containers":[{"id":"parent","kind":"parallel","start":{"kind":"at","offset":t(0,1)},"endConditions":[{"kind":"click","target":"stop","delay":t(1,2)}],"duration":{"kind":"automatic"},"fill":"remove","children":["a"]}]}
    })).unwrap()
}
fn plan(timeline: &Timeline) -> TimelinePlan {
    TimelinePlan::compile(timeline, TimelineLimits::default(), &|| false).unwrap()
}
fn event(n: i64, d: u32, sequence: u32, target: Option<&str>) -> PlaybackEvent {
    PlaybackEvent {
        generation: binding().generation,
        sequence,
        at: t(n, d),
        event: InputEvent::Click {
            target: target.map(|v| ObjectId::new(v).unwrap()),
        },
    }
}
fn history() -> EventHistory {
    EventHistory {
        binding: binding(),
        through: t(100, 1),
        events: vec![
            event(1, 1, 1, None),
            event(2, 1, 2, Some("stop")),
            event(4, 1, 3, None),
        ],
    }
}
fn counts(s: &TimelineSampler) -> (u64, u64, u64) {
    let i = s.info();
    (
        i.schedules_built.get(),
        i.schedules_reused.get(),
        i.retained_events.get(),
    )
}
#[test]
fn retained_frames_equal_stateless_through_clicks_container_ends_and_backward_seeks() {
    let p = plan(&graph());
    let mut s = TimelineSampler::new(p.clone());
    let h = history();
    for (n, d, expected) in [
        (0, 1, (1, 0, 0)),
        (1, 2, (1, 1, 0)),
        (1, 1, (2, 1, 1)),
        (3, 2, (2, 2, 1)),
        (2, 1, (3, 2, 2)),
        (5, 2, (3, 3, 2)),
        (9, 2, (4, 3, 3)),
        (20, 1, (4, 4, 3)),
        (3, 2, (5, 4, 1)),
        (1, 2, (6, 4, 0)),
    ] {
        let actual = s
            .evaluate(&binding(), t(n, d), Some(&h), &|| false)
            .unwrap();
        assert_eq!(
            actual,
            p.evaluate(&binding(), t(n, d), Some(&h), &|| false)
                .unwrap()
        );
        assert_eq!(counts(&s), expected);
        assert_eq!(s.info().retained_intervals.get(), 3);
    }
}
#[test]
fn identity_includes_exact_times_targets_and_binding_but_not_future_or_duplicate_events() {
    let p = plan(&graph());
    let mut s = TimelineSampler::new(p.clone());
    let mut h = history();
    let initial = s
        .evaluate(&binding(), t(3, 2), Some(&h), &|| false)
        .unwrap();
    h.events[0].at = t(2, 2);
    h.events.insert(1, h.events[0].clone());
    h.events.push(event(5, 1, 4, None));
    assert_eq!(
        s.evaluate(&binding(), t(3, 2), Some(&h), &|| false)
            .unwrap(),
        initial
    );
    assert_eq!(counts(&s), (1, 1, 1));
    h.events.remove(1);
    h.events[0].at = t(1, 2);
    assert_ne!(
        s.evaluate(&binding(), t(3, 2), Some(&h), &|| false)
            .unwrap(),
        initial
    );
    assert_eq!(counts(&s), (2, 1, 1));
    h.events[0].event = InputEvent::Click {
        target: Some(ObjectId::new("different").unwrap()),
    };
    s.evaluate(&binding(), t(3, 2), Some(&h), &|| false)
        .unwrap();
    assert_eq!(counts(&s), (3, 1, 1));
    for which in 0..3 {
        let mut b = binding();
        match which {
            0 => b.session = PlaybackSessionId::new("other").unwrap(),
            1 => b.revision = Digest::from_sha256([9; 32]),
            _ => b.generation = PlaybackGeneration::new(5),
        }
        h.binding = b.clone();
        for e in &mut h.events {
            e.generation = b.generation;
        }
        assert_eq!(
            s.evaluate(&b, t(3, 2), Some(&h), &|| false).unwrap(),
            p.evaluate(&b, t(3, 2), Some(&h), &|| false).unwrap()
        );
        assert_eq!(counts(&s), (4 + which, 1, 1));
        assert_eq!(s.info().cached_binding, Some(b));
    }
    s.clear();
    assert_eq!(s.info().cached_binding, None);
    assert_eq!(counts(&s), (6, 1, 0));
    assert_eq!(s.info().retained_intervals.get(), 0);
}
#[test]
fn full_history_validation_precedes_hits_and_failures_preserve_previous_entry() {
    let timeline = graph();
    let p = TimelinePlan::compile(
        &timeline,
        TimelineLimits {
            max_events: 4,
            ..Default::default()
        },
        &|| false,
    )
    .unwrap();
    let mut s = TimelineSampler::new(p.clone());
    let h = history();
    s.evaluate(&binding(), t(3, 2), Some(&h), &|| false)
        .unwrap();
    let before = s.info();
    let mut variants = vec![];
    for change in 0..9 {
        let mut bad = h.clone();
        match change {
            0 => bad.through = t(1, 1),
            1 => bad.binding.generation = PlaybackGeneration::new(5),
            2 => bad.events[2].generation = PlaybackGeneration::new(5),
            3 => bad.events[2].sequence = 4,
            4 => bad.events[2].at = t(1, 2),
            5 => bad.events[2].at = t(101, 1),
            6 => bad.events[2].at = t(-1, 1),
            7 => {
                bad.events.push(bad.events[2].clone());
                bad.events.push(bad.events[2].clone());
            }
            _ => {
                bad.events.insert(1, event(2, 2, 1, None));
            } // equal instant is not an exact duplicate record
        }
        variants.push(bad);
    }
    for bad in variants {
        let actual = s
            .evaluate(&binding(), t(3, 2), Some(&bad), &|| false)
            .unwrap_err()
            .to_string();
        assert_eq!(
            actual,
            p.evaluate(&binding(), t(3, 2), Some(&bad), &|| false)
                .unwrap_err()
                .to_string()
        );
        assert_eq!(s.info(), before);
    }
    assert!(matches!(
        s.evaluate(&binding(), t(3, 2), None, &|| false),
        Err(TimelineError::MissingEventHistory)
    ));
    assert!(matches!(
        s.evaluate(&binding(), t(-1, 1), Some(&h), &|| false),
        Err(TimelineError::Invalid { .. })
    ));
    assert_eq!(s.info(), before);
    s.evaluate(&binding(), t(3, 2), Some(&h), &|| false)
        .unwrap();
    assert_eq!(counts(&s), (1, 1, 1));
}
#[test]
fn every_cancel_checkpoint_keeps_cache_and_counters_atomic_on_hits_and_misses() {
    let p = plan(&graph());
    let h = history();
    for at in [t(3, 2), t(5, 2)] {
        let prepare = || {
            let mut s = TimelineSampler::new(p.clone());
            s.evaluate(&binding(), t(1, 1), Some(&h), &|| false)
                .unwrap();
            s
        };
        let mut probe = prepare();
        let steps = Cell::new(0);
        let expected = probe
            .evaluate(&binding(), at, Some(&h), &|| {
                steps.set(steps.get() + 1);
                false
            })
            .unwrap();
        for stop in 0..steps.get() {
            let mut s = prepare();
            let before = s.info();
            let current = Cell::new(0);
            let result = s.evaluate(&binding(), at, Some(&h), &|| {
                let yes = current.get() == stop;
                current.set(current.get() + 1);
                yes
            });
            assert!(
                matches!(result, Err(TimelineError::Cancelled)),
                "checkpoint {stop}"
            );
            assert_eq!(s.info(), before);
            assert_eq!(
                s.evaluate(&binding(), at, Some(&h), &|| false).unwrap(),
                expected
            );
        }
    }
}
#[test]
fn semantic_sample_failure_never_publishes_a_new_prefix_or_corrupts_a_hit() {
    let mut timeline = graph();
    timeline.format = TimelineVersion::V01;
    timeline.tree = None;
    timeline.nodes.truncate(1);
    timeline.nodes[0].start = TimeCondition::At { offset: t(1, 1) };
    timeline.nodes[0].time_transform = Some(TimeTransform {
        speed_milli_percent: -100000,
        ..Default::default()
    });
    let p = plan(&timeline);
    let mut s = TimelineSampler::new(p);
    let h = history();
    s.evaluate(&binding(), t(0, 1), Some(&h), &|| false)
        .unwrap();
    let before = s.info();
    for events in [vec![], h.events.clone()] {
        let h = EventHistory {
            events,
            ..h.clone()
        };
        assert!(
            s.evaluate(&binding(), t(3, 2), Some(&h), &|| false)
                .is_err()
        );
        assert_eq!(s.info(), before);
    }
    s.evaluate(&binding(), t(0, 1), Some(&h), &|| false)
        .unwrap();
    assert_eq!(counts(&s), (1, 1, 0));
}
