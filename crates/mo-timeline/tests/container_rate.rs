use mo_common::*;
use mo_timeline::*;
use serde_json::{Value, json};

fn t(n: i64, d: u32) -> RationalTime {
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
        session: PlaybackSessionId::new("container-rate").unwrap(),
        revision: Digest::from_sha256([91; 32]),
        generation: PlaybackGeneration::new(1),
    }
}
fn at(n: i64, d: u32) -> Value {
    json!({"kind":"at","offset":{"ticks":n.to_string(),"timescale":d}})
}
fn node(id: &str, duration: i64, start: Value) -> Value {
    json!({"id":id,"start":start,"duration":{"ticks":duration.to_string(),"timescale":1},"repeatMilli":1000,"fill":"hold","effect":{"kind":"rotation","target":id,"from":0,"to":120}})
}
fn group(id: &str, speed: i32, children: &[&str]) -> Value {
    json!({"id":id,"kind":"parallel","start":at(0,1),"duration":{"kind":"automatic"},"fill":"hold","children":children,"timeTransform":{"speedMilliPercent":speed,"autoReverse":false,"accelerationMilliPercent":0,"decelerationMilliPercent":0}})
}
fn model(nodes: Vec<Value>, groups: Vec<Value>, roots: &[&str]) -> Timeline {
    serde_json::from_value(json!({"format":"musteroffice.timeline/0.2-draft","nodes":nodes,"tree":{"roots":roots,"containers":groups}})).unwrap()
}
fn plan(model: &Timeline) -> TimelinePlan {
    TimelinePlan::compile(model, TimelineLimits::default(), &|| false).unwrap()
}
fn sample(plan: &TimelinePlan, at: RationalTime, events: Option<&EventHistory>) -> FrameState {
    plan.evaluate(&binding(), at, events, &|| false)
        .unwrap()
        .state
}
fn events(values: &[(i64, u32, InputEvent)]) -> EventHistory {
    EventHistory {
        binding: binding(),
        through: t(100, 1),
        events: values
            .iter()
            .enumerate()
            .map(|(i, (n, d, event))| PlaybackEvent {
                generation: binding().generation,
                sequence: i as u32 + 1,
                at: t(*n, *d),
                event: event.clone(),
            })
            .collect(),
    }
}

#[test]
fn nested_rates_project_delays_sequences_and_cross_scope_events_in_receiver_time() {
    let mut inner = group("inner", 150_000, &["a", "b"]);
    inner["kind"] = json!("sequence");
    inner["start"] = at(1, 1);
    let mut outer = group("outer", 200_000, &["inner"]);
    outer["start"] = at(2, 1);
    let outside = node(
        "outside",
        2,
        json!({"kind":"after","node":"b","event":"end","delay":{"ticks":"1","timescale":2}}),
    );
    let m = model(
        vec![node("a", 3, at(1, 1)), node("b", 6, at(2, 1)), outside],
        vec![outer, inner],
        &["outer", "outside"],
    );
    let p = plan(&m);
    // Outer begins at 2; inner at 2+1/2; a at 5/2+1/3.
    // a lasts 1 host second. b waits another 2/3, then lasts 2 seconds.
    let f = sample(&p, t(10, 3), None);
    assert_eq!(f.nodes[0].start, Some(exact(17, 6)));
    assert_eq!(f.nodes[0].end, Some(exact(23, 6)));
    assert_eq!(f.nodes[0].progress, Some(exact(1, 2)));
    assert_eq!(f.nodes[1].start, Some(exact(9, 2)));
    assert_eq!(f.nodes[1].end, Some(exact(13, 2)));
    assert_eq!(f.nodes[2].start, Some(exact(7, 1)));
    assert_eq!(f.containers[0].end, Some(exact(13, 2)));
    assert_eq!(f.containers[1].end, Some(exact(13, 2)));
    assert_eq!(
        sample(&p, t(11, 2), None).nodes[1].progress,
        Some(exact(1, 2))
    );
    let mut retained = TimelineSampler::new(p.clone());
    for at in [t(8, 1), t(11, 2), t(10, 3), t(0, 1), t(9, 2)] {
        assert_eq!(
            retained
                .evaluate(&binding(), at, None, &|| false)
                .unwrap()
                .state,
            sample(&p, at, None)
        );
    }
}

#[test]
fn fixed_duration_scales_but_own_end_conditions_and_external_inputs_use_parent_time() {
    let mut fixed = group("fixed", 200_000, &["a"]);
    fixed["start"] = at(2, 1);
    fixed["duration"] = json!({"kind":"fixed","duration":{"ticks":"4","timescale":1}});
    let m = model(
        vec![node("a", 8, at(0, 1))],
        vec![fixed.clone()],
        &["fixed"],
    );
    let f = sample(&plan(&m), t(10, 1), None);
    assert_eq!(f.containers[0].end, Some(exact(4, 1)));
    assert_eq!(f.nodes[0].progress, Some(exact(1, 2)));
    // At offsets are in the parent's clock from the enable gate: 3 is host 3,
    // not 2+3/2 and not speed-scaled. The leaf has consumed 2 of its 8 seconds.
    fixed["endConditions"] = json!([at(3, 1)]);
    let m = model(vec![node("a", 8, at(0, 1))], vec![fixed], &["fixed"]);
    let f = sample(&plan(&m), t(10, 1), None);
    assert_eq!(f.containers[0].end, Some(exact(3, 1)));
    assert_eq!(f.nodes[0].progress, Some(exact(1, 4)));
    let click = json!({"kind":"click","target":null,"delay":{"ticks":"2","timescale":1}});
    let m = model(
        vec![node("a", 4, click)],
        vec![group("fast", 200_000, &["a"])],
        &["fast"],
    );
    let history = events(&[(1, 1, InputEvent::Click { target: None })]);
    let f = sample(&plan(&m), t(3, 1), Some(&history));
    assert_eq!(f.nodes[0].start, Some(exact(2, 1)));
    assert_eq!(f.nodes[0].end, Some(exact(4, 1)));
    assert_eq!(f.nodes[0].progress, Some(exact(1, 2)));
}

#[test]
fn restarting_parent_anchors_each_paced_subtree_to_its_own_activation() {
    let mut g = group("fast", 200_000, &["a"]);
    g["start"] = json!({"kind":"click","target":null,"delay":{"ticks":"0","timescale":1}});
    g["restart"] = json!("always");
    let m = model(vec![node("a", 6, at(1, 1))], vec![g], &["fast"]);
    let p = plan(&m);
    let history = events(&[
        (2, 1, InputEvent::Click { target: None }),
        (4, 1, InputEvent::Click { target: None }),
    ]);
    let mut retained = TimelineSampler::new(p.clone());
    for (at, start) in [
        (t(3, 1), exact(5, 2)),
        (t(5, 1), exact(9, 2)),
        (t(3, 1), exact(5, 2)),
    ] {
        let f = sample(&p, at, Some(&history));
        assert_eq!(f.nodes[0].start, Some(start));
        assert_eq!(f.nodes[0].progress, Some(exact(1, 6)));
        assert_eq!(
            retained
                .evaluate(&binding(), at, Some(&history), &|| false)
                .unwrap()
                .state,
            f
        );
    }
}

#[test]
fn navigation_seek_preserves_scaled_deadlines_and_unrelated_branch_clocks() {
    let mut seq = group("seq", 200_000, &["inner", "next"]);
    seq["kind"] = json!("sequence");
    seq["duration"] = json!({"kind":"indefinite"});
    seq["navigation"] = json!({"concurrent":false,"nextAction":"seek","previousAction":"none","nextConditions":[{"kind":"navigation","direction":"next","target":null,"delay":{"ticks":"0","timescale":1}}],"previousConditions":[{"kind":"navigation","direction":"previous","target":null,"delay":{"ticks":"0","timescale":1}}]});
    let mut inner = group("inner", 150_000, &["a", "delayed"]);
    inner["kind"] = json!("sequence");
    let outside = node("outside", 4, at(0, 1));
    let notified = node(
        "notified",
        4,
        json!({"kind":"after","node":"delayed","event":"end","delay":{"ticks":"1","timescale":4}}),
    );
    let m = model(
        vec![
            node("a", 3, at(0, 1)),
            node("delayed", 3, at(1, 1)),
            node("next", 4, at(0, 1)),
            outside,
            notified,
        ],
        vec![seq, inner],
        &["seq", "outside", "notified"],
    );
    let p = plan(&m);
    let history = events(&[(
        1,
        2,
        InputEvent::Navigation {
            direction: NavigationDirection::Next,
            target: None,
        },
    )]);
    let f = sample(&p, t(1, 2), Some(&history));
    for i in [0, 1] {
        assert_eq!(f.nodes[i].end, Some(exact(1, 2)));
        assert_eq!(f.nodes[i].progress, Some(exact(1, 1)));
    }
    assert_eq!(f.nodes[2].start, Some(exact(1, 2)));
    assert_eq!(f.nodes[2].progress, Some(exact(0, 1)));
    assert_eq!(f.nodes[3].progress, Some(exact(1, 8)));
    assert_eq!(
        sample(&p, t(1, 1), Some(&history)).nodes[4].start,
        Some(exact(3, 4))
    );
    assert_eq!(
        sample(&p, t(3, 2), Some(&history)).nodes[2].progress,
        Some(exact(1, 2))
    );
    // Previous creates fresh child clocks; no jump from the old child may leak.
    let history = events(&[
        (
            1,
            2,
            InputEvent::Navigation {
                direction: NavigationDirection::Next,
                target: None,
            },
        ),
        (
            1,
            1,
            InputEvent::Navigation {
                direction: NavigationDirection::Previous,
                target: None,
            },
        ),
    ]);
    let f = sample(&p, t(3, 2), Some(&history));
    assert_eq!(f.nodes[0].start, Some(exact(1, 1)));
    assert_eq!(f.nodes[0].progress, Some(exact(1, 2)));
    assert_eq!(f.nodes[3].progress, Some(exact(3, 8)));
}

#[test]
fn coincident_easing_cascades_with_rates_without_changing_child_origins() {
    let mut outer = group("outer", 200_000, &["middle"]);
    outer["start"] = at(1, 1);
    outer["timeTransform"]["accelerationMilliPercent"] = json!(100_000);
    let middle = group("middle", 150_000, &["inner"]);
    let mut inner = group("inner", 50_000, &["a"]);
    inner["timeTransform"]["accelerationMilliPercent"] = json!(50_000);
    inner["timeTransform"]["decelerationMilliPercent"] = json!(50_000);
    let m = model(
        vec![node("a", 6, at(0, 1))],
        vec![outer, middle, inner],
        &["outer"],
    );
    let p = plan(&m);
    // Total speed 3/2: host span 4. At 2, normalized time 1/4;
    // outer easing maps to 1/16, inner easing to 1/128.
    let f = sample(&p, t(2, 1), None);
    assert_eq!(f.nodes[0].progress, Some(exact(1, 128)));
    assert_eq!(f.nodes[0].end, Some(exact(5, 1)));
    assert_eq!(
        sample(&p, t(5, 1), None).nodes[0].progress,
        Some(exact(1, 1))
    );
}

#[test]
fn delayed_reverse_is_not_admitted_and_rate_arithmetic_is_bounded() {
    let mut m = model(
        vec![node("a", 3, at(1, 1))],
        vec![group("g", -100_000, &["a"])],
        &["g"],
    );
    assert!(TimelinePlan::compile(&m, TimelineLimits::default(), &|| false).is_err());
    m.tree.as_mut().unwrap().containers[0]
        .time_transform
        .as_mut()
        .unwrap()
        .speed_milli_percent = 123_457;
    let mut groups = Vec::new();
    for i in 0..10 {
        let child = if i == 9 {
            "a".to_owned()
        } else {
            format!("g{}", i + 1)
        };
        groups.push(group(&format!("g{i}"), 123_457, &[&child]));
    }
    m = model(vec![node("a", 3, at(0, 1))], groups, &["g0"]);
    let limits = TimelineLimits {
        max_exact_bits: 128,
        ..Default::default()
    };
    assert!(matches!(
        TimelinePlan::compile(&m, limits, &|| false),
        Err(TimelineError::Limit(_))
    ));
    assert!(matches!(
        TimelinePlan::compile(&m, TimelineLimits::default(), &|| true),
        Err(TimelineError::Cancelled)
    ));
}
