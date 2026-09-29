use mo_common::*;
use mo_presentation_compile::{playback::*, *};
use mo_timeline::*;
use std::collections::BTreeSet;
fn t(n: i64, d: u32) -> RationalTime {
    RationalTime::new(n, d).unwrap()
}
fn binding() -> PlaybackBinding {
    PlaybackBinding {
        session: PlaybackSessionId::new("geometry-reuse").unwrap(),
        revision: Digest::from_sha256([9; 32]),
        generation: PlaybackGeneration::new(1),
    }
}
#[test]
fn reused_geometry_tracks_precision_changes_backward_seeks_and_generation_fences() {
    let mut q: PageRenderRequest = serde_json::from_str(include_str!(
        "../../../fixtures/presentations/playback/page.json"
    ))
    .unwrap();
    let timeline = q.page.document.timelines.values_mut().next().unwrap();
    let mut scale = timeline.nodes[0].clone();
    scale.id = TimingNodeId::new("scale").unwrap();
    scale.effect = Effect::Scale {
        target: ObjectId::new("group:1").unwrap(),
        from: ScaleValue { x: 0, y: 0 },
        to: ScaleValue {
            x: 800000,
            y: 400000,
        },
    };
    timeline.nodes.push(scale);
    let original = q.page.document.clone();
    let create = |binding| {
        PlaybackPagePlan::new(q.clone(), binding, TimelineLimits::default(), &|| false).unwrap()
    };
    let mut reused = create(binding());
    let mut precisions = BTreeSet::new();
    for generation in 1..=3 {
        if generation > 1 {
            let prior = reused.binding().clone();
            reused
                .advance_generation(&prior, PlaybackGeneration::new(generation))
                .unwrap();
        }
        for at in [
            t(0, 1),
            t(1, 7),
            t(1, 2),
            t(1, 1),
            t(2, 1),
            t(1, 7),
            t(2, 1),
            t(0, 1),
        ] {
            let expected = create(reused.binding().clone())
                .compile_frame(at, None, &|| false)
                .unwrap();
            let actual = reused.compile_frame(at, None, &|| false).unwrap();
            precisions.insert(actual.page.info.curve_segments);
            assert_eq!(
                serde_json::to_value(actual).unwrap(),
                serde_json::to_value(expected).unwrap()
            );
            assert!(reused.compile_frame(at, None, &|| true).is_err());
        }
    }
    assert!(
        precisions.len() > 1,
        "exercise replacement at a different subdivision level"
    );
    assert_eq!(q.page.document, original);
}
