use mo_common::*;
use mo_timeline::*;
use std::cell::Cell;

fn t(ms: i64) -> RationalTime {
    RationalTime::new(ms, 1000).unwrap()
}
fn id(s: &str) -> TimingNodeId {
    TimingNodeId::new(s).unwrap()
}
fn effect(name: &str, duration: i64) -> PresentationEffect {
    PresentationEffect {
        id: id(name),
        delay: t(0),
        duration: t(duration),
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
fn batch(effects: Vec<PresentationEffect>) -> PresentationBatch {
    PresentationBatch {
        delay: t(0),
        effects,
    }
}
fn sequence(batches: Vec<PresentationBatch>) -> PresentationSequence {
    PresentationSequence {
        groups: vec![PresentationGroup {
            start: PresentationGroupStart::Automatic,
            batches,
        }],
    }
}
fn compile(s: &PresentationSequence) -> Timeline {
    s.compile(TimelineLimits::default(), &|| false).unwrap()
}
fn binding() -> PlaybackBinding {
    PlaybackBinding {
        session: PlaybackSessionId::new("authoring").unwrap(),
        revision: Digest::from_sha256([29; 32]),
        generation: PlaybackGeneration::new(1),
    }
}
fn frame(timeline: &Timeline, at: i64, events: &[(i64, NavigationDirection)]) -> EvaluatedFrame {
    let history = EventHistory {
        binding: binding(),
        through: t(100_000),
        events: events
            .iter()
            .enumerate()
            .map(|(i, (at, direction))| PlaybackEvent {
                generation: binding().generation,
                sequence: i as u32 + 1,
                at: t(*at),
                event: InputEvent::Navigation {
                    direction: *direction,
                    target: None,
                },
            })
            .collect(),
    };
    TimelinePlan::compile(timeline, TimelineLimits::default(), &|| false)
        .unwrap()
        .evaluate(&binding(), t(at), Some(&history), &|| false)
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
fn exact(n: i64, d: i64) -> Option<ExactValue> {
    Some(ExactValue {
        numerator: n.to_string(),
        denominator: d.to_string(),
    })
}

#[test]
fn next_batch_waits_for_longest_parallel_effect_and_recompilation_updates_it() {
    // WPS native control: 2 s + 1 s parallel; the next batch starts at 2 s.
    let mut s = sequence(vec![
        batch(vec![effect("long", 2000), effect("short", 1000)]),
        batch(vec![effect("after", 1000)]),
    ]);
    let timeline = compile(&s);
    assert_eq!(rotation(&frame(&timeline, 1999, &[]), "after"), None);
    assert_eq!(rotation(&frame(&timeline, 2000, &[]), "after"), exact(0, 1));
    assert_eq!(
        rotation(&frame(&timeline, 2500, &[]), "after"),
        exact(60, 1)
    );
    assert_eq!(
        rotation(&frame(&timeline, 3000, &[]), "after"),
        exact(120, 1)
    );
    s.groups[0].batches[0].effects[0].duration = t(3000);
    let longer = compile(&s);
    assert_eq!(rotation(&frame(&longer, 2500, &[]), "after"), None);
    assert_eq!(rotation(&frame(&longer, 3500, &[]), "after"), exact(60, 1));
    assert_eq!(compile(&s), longer);
}

#[test]
fn delays_fractional_repeats_and_transformed_duration_use_exact_time() {
    let mut a = effect("a", 1000);
    a.delay = t(200);
    a.repeat_milli = 1500.into();
    a.time_transform = Some(TimeTransform {
        speed_milli_percent: 125000,
        auto_reverse: true,
        ..TimeTransform::default()
    });
    // 1 s * 2 * 1.5 / 1.25 = 2.4 s; +0.2 effect +0.1 batch = 2.7.
    let mut first = batch(vec![a]);
    first.delay = t(100);
    let mut second = batch(vec![effect("b", 1000)]);
    second.delay = t(300);
    let timeline = compile(&sequence(vec![first, second]));
    assert_eq!(rotation(&frame(&timeline, 2999, &[]), "b"), None);
    assert_eq!(rotation(&frame(&timeline, 3500, &[]), "b"), exact(60, 1));
    let mut third = effect("c", 1000);
    third.time_transform = Some(TimeTransform {
        speed_milli_percent: 300000,
        ..TimeTransform::default()
    });
    let timeline = compile(&sequence(vec![
        batch(vec![third]),
        batch(vec![effect("d", 1000)]),
    ]));
    let f = frame(&timeline, 500, &[]);
    assert_eq!(rotation(&f, "d"), exact(20, 1));
    let d = f.state.nodes.iter().find(|n| n.node == id("d")).unwrap();
    assert_eq!(d.start, exact(1, 3));
}

#[test]
fn indefinite_final_batch_is_valid_but_cannot_supply_a_successor_offset() {
    let mut a = effect("a", 1000);
    a.repeat_milli = RepeatCount::Indefinite;
    let mut s = sequence(vec![batch(vec![a])]);
    compile(&s);
    s.groups[0].batches.push(batch(vec![effect("b", 1000)]));
    assert!(
        s.compile(TimelineLimits::default(), &|| false)
            .unwrap_err()
            .to_string()
            .contains("unbounded")
    );
    s.groups[0].batches[0].effects[0].repeat_duration = Some(RepeatDuration::Finite(t(1750)));
    assert_eq!(rotation(&frame(&compile(&s), 2250, &[]), "b"), exact(60, 1));
}

#[test]
fn main_sequence_waits_for_next_and_previous_can_replay_a_finished_group() {
    let mut s = sequence(vec![batch(vec![effect("a", 1000)])]);
    s.groups[0].start = PresentationGroupStart::Next;
    s.groups.push(PresentationGroup {
        start: PresentationGroupStart::Next,
        batches: vec![batch(vec![effect("b", 1000)])],
    });
    let timeline = compile(&s);
    let events = [
        (1000, NavigationDirection::Next),
        (2500, NavigationDirection::Next),
        (4000, NavigationDirection::Previous),
        (4500, NavigationDirection::Next),
    ];
    assert_eq!(rotation(&frame(&timeline, 999, &events), "a"), None);
    assert_eq!(
        rotation(&frame(&timeline, 1500, &events), "a"),
        exact(60, 1)
    );
    assert_eq!(
        rotation(&frame(&timeline, 3000, &events), "b"),
        exact(60, 1)
    );
    assert_eq!(rotation(&frame(&timeline, 4000, &events), "b"), None);
    assert_eq!(
        rotation(&frame(&timeline, 5000, &events), "b"),
        exact(60, 1)
    );
}

#[test]
fn automatic_group_uses_node_notification_without_replaying_after_previous() {
    let timeline = compile(&sequence(vec![batch(vec![effect("a", 1000)])]));
    let tree = timeline.tree.as_ref().unwrap();
    let main = tree
        .containers
        .iter()
        .find(|c| c.presentation == Some(PresentationRole::MainSequence))
        .unwrap();
    let group = tree
        .containers
        .iter()
        .find(|c| c.id == main.children[0])
        .unwrap();
    assert!(
        matches!(&group.start.conditions()[1], TimeCondition::After { node, event: NodeEvent::OnBegin, .. } if node == &main.id)
    );
    let events = [
        (1500, NavigationDirection::Previous),
        (2500, NavigationDirection::Next),
    ];
    assert_eq!(rotation(&frame(&timeline, 500, &events), "a"), exact(60, 1));
    assert_eq!(rotation(&frame(&timeline, 1500, &events), "a"), None);
    assert_eq!(rotation(&frame(&timeline, 2499, &events), "a"), None);
    assert_eq!(
        rotation(&frame(&timeline, 3000, &events), "a"),
        exact(60, 1)
    );
    assert_eq!(rotation(&frame(&timeline, 500, &events), "a"), exact(60, 1));
}

#[test]
fn entrance_baseline_and_editorial_roles_are_preserved() {
    let mut a = effect("appear", 1);
    a.effect = Effect::SetVisibility {
        target: ObjectId::new("object").unwrap(),
        value: Visibility::Visible,
    };
    let mut s = sequence(vec![batch(vec![a]), batch(vec![effect("spin", 1000)])]);
    s.groups[0].start = PresentationGroupStart::Next;
    let timeline = compile(&s);
    let roles: Vec<_> = timeline
        .tree
        .as_ref()
        .unwrap()
        .containers
        .iter()
        .filter_map(|c| c.presentation)
        .collect();
    assert!(roles.contains(&PresentationRole::Effect {
        preset: PresentationPreset::Appear,
        trigger: PresentationTrigger::Click
    }));
    assert!(roles.contains(&PresentationRole::Effect {
        preset: PresentationPreset::Spin,
        trigger: PresentationTrigger::AfterPrevious
    }));
    assert_eq!(
        frame(&timeline, 0, &[]).state.visibility[&ObjectId::new("object").unwrap()],
        Visibility::Hidden
    );
}

#[test]
fn generated_ids_cannot_collide_with_caller_ids_and_limits_include_containers() {
    let s = sequence(vec![batch(vec![
        effect("mo.presentation.0", 1000),
        effect("mo.presentation.1", 1000),
    ])]);
    let timeline = compile(&s);
    assert_eq!(timeline.node_count(), 7);
    assert!(matches!(
        s.compile(
            TimelineLimits {
                max_nodes: 6,
                ..TimelineLimits::default()
            },
            &|| false
        ),
        Err(TimelineError::Limit(_))
    ));
    let calls = Cell::new(0);
    assert!(matches!(
        s.compile(TimelineLimits::default(), &|| {
            let n = calls.get() + 1;
            calls.set(n);
            n > 3
        }),
        Err(TimelineError::Cancelled)
    ));
}

#[test]
fn invalid_author_programs_are_rejected_without_inference() {
    let base = sequence(vec![batch(vec![effect("a", 1000)])]);
    for change in 0..6 {
        let mut s = base.clone();
        match change {
            0 => s.groups[0].batches[0].effects.push(effect("a", 1000)),
            1 => s.groups[0].batches[0].effects[0].delay = t(-1),
            2 => s.groups[0].batches[0].delay = t(-1),
            3 => s.groups[0].batches.clear(),
            4 => s.groups[0].batches[0].effects.clear(),
            _ => s.groups.push(s.groups[0].clone()),
        }
        assert!(
            s.compile(TimelineLimits::default(), &|| false).is_err(),
            "case {change}"
        );
    }
    let mut value = serde_json::to_value(&base).unwrap();
    value["groups"][0]["batches"][0]["effects"][0]["start"] = serde_json::json!({"kind":"never"});
    assert!(serde_json::from_value::<PresentationSequence>(value).is_err());
}
