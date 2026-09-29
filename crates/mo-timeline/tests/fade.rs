use mo_common::*;
use mo_timeline::*;
fn t(ms: i64) -> RationalTime {
    RationalTime::new(ms, 1000).unwrap()
}
fn target() -> ObjectId {
    ObjectId::new("shape").unwrap()
}
fn binding() -> PlaybackBinding {
    PlaybackBinding {
        session: PlaybackSessionId::new("fade").unwrap(),
        revision: Digest::from_sha256([17; 32]),
        generation: PlaybackGeneration::new(1),
    }
}
fn effect(id: &str, transition: FadeTransition) -> PresentationEffect {
    PresentationEffect {
        id: TimingNodeId::new(id).unwrap(),
        delay: t(0),
        duration: t(1000),
        repeat_milli: 1000.into(),
        repeat_duration: None,
        fill: FillMode::Remove,
        time_transform: None,
        effect: Effect::Fade {
            target: target(),
            transition,
        },
    }
}
fn sequence(effects: Vec<PresentationEffect>) -> PresentationSequence {
    PresentationSequence {
        groups: vec![PresentationGroup {
            start: PresentationGroupStart::Automatic,
            batches: effects
                .into_iter()
                .map(|e| PresentationBatch {
                    delay: t(0),
                    effects: vec![e],
                })
                .collect(),
        }],
    }
}
fn frame(timeline: &Timeline, ms: i64) -> FrameState {
    TimelinePlan::compile(timeline, TimelineLimits::default(), &|| false)
        .unwrap()
        .evaluate(
            &binding(),
            t(ms),
            Some(&EventHistory {
                binding: binding(),
                through: t(10000),
                events: vec![],
            }),
            &|| false,
        )
        .unwrap()
        .state
}
fn opacity(f: &FrameState, n: &str, d: &str) {
    assert_eq!(
        f.opacity[&target()],
        ExactValue {
            numerator: n.into(),
            denominator: d.into()
        }
    );
}
#[test]
fn native_compound_fades_have_exact_opacity_and_independent_visibility() {
    let mut s = sequence(vec![
        effect("in", FadeTransition::In),
        effect("out", FadeTransition::Out),
    ]);
    s.groups[0].batches[0].delay = t(100);
    let timeline = s.compile(TimelineLimits::default(), &|| false).unwrap();
    assert_eq!(timeline.nodes.len(), 4);
    assert_eq!(
        frame(&timeline, 99).visibility[&target()],
        Visibility::Hidden
    );
    opacity(&frame(&timeline, 100), "0", "1");
    opacity(&frame(&timeline, 350), "1", "4");
    assert_eq!(
        frame(&timeline, 350).visibility[&target()],
        Visibility::Visible
    );
    opacity(&frame(&timeline, 1350), "3", "4");
    assert_eq!(
        frame(&timeline, 2098).visibility[&target()],
        Visibility::Visible
    );
    assert_eq!(
        frame(&timeline, 2099).visibility[&target()],
        Visibility::Hidden
    );
    assert_eq!(
        frame(&timeline, 2100).visibility[&target()],
        Visibility::Hidden
    );
    assert!(frame(&timeline, 2100).opacity.is_empty());
    assert_eq!(
        frame(&timeline, 0).visibility[&target()],
        Visibility::Hidden
    );
}
#[test]
fn compound_helpers_follow_exact_transformed_active_span_and_unbounded_exit() {
    let mut fade = effect("out", FadeTransition::Out);
    fade.repeat_milli = 1500.into();
    fade.time_transform = Some(TimeTransform {
        speed_milli_percent: 200000,
        auto_reverse: true,
        ..TimeTransform::default()
    });
    let timeline = sequence(vec![fade.clone()])
        .compile(TimelineLimits::default(), &|| false)
        .unwrap();
    opacity(&frame(&timeline, 250), "1", "2");
    assert!(!frame(&timeline, 1498).visibility.contains_key(&target()));
    assert_eq!(
        frame(&timeline, 1499).visibility[&target()],
        Visibility::Hidden
    );
    fade.repeat_milli = RepeatCount::Indefinite;
    let timeline = sequence(vec![fade])
        .compile(TimelineLimits::default(), &|| false)
        .unwrap();
    assert!(!frame(&timeline, 9000).visibility.contains_key(&target()));
}
#[test]
fn preset_rejects_cross_object_helpers_and_generated_helpers_count_towards_budget() {
    let s = sequence(vec![effect("in", FadeTransition::In)]);
    assert!(matches!(
        s.compile(
            TimelineLimits {
                max_nodes: 5,
                ..TimelineLimits::default()
            },
            &|| false
        ),
        Err(TimelineError::Limit(_))
    ));
    let mut timeline = s.compile(TimelineLimits::default(), &|| false).unwrap();
    timeline.nodes[0].effect = Effect::SetVisibility {
        target: ObjectId::new("other").unwrap(),
        value: Visibility::Visible,
    };
    assert!(TimelinePlan::compile(&timeline, TimelineLimits::default(), &|| false).is_err());
    assert!(matches!(
        s.compile(TimelineLimits::default(), &|| true),
        Err(TimelineError::Cancelled)
    ));
}
