use mo_common::*;
use mo_presentation_compile::{playback::*, *};
use mo_timeline::*;
fn time(ms: i64) -> RationalTime {
    RationalTime::new(ms, 1000).unwrap()
}
#[test]
fn hidden_group_suppresses_children_without_mutation_or_hidden_state_leaking_across_seek() {
    let mut q: PageRenderRequest = serde_json::from_str(include_str!(
        "../../../fixtures/presentations/playback/page.json"
    ))
    .unwrap();
    let timeline = q.page.document.timelines.values_mut().next().unwrap();
    timeline.nodes.truncate(1);
    timeline.nodes[0].effect = Effect::SetVisibility {
        target: ObjectId::new("group:1").unwrap(),
        value: Visibility::Hidden,
    };
    timeline.nodes[0].start = TimeCondition::At { offset: time(500) }.into();
    timeline.nodes[0].duration = time(500);
    timeline.nodes[0].fill = FillMode::Remove;
    let mut child = timeline.nodes[0].clone();
    child.id = TimingNodeId::new("child-visible").unwrap();
    child.effect = Effect::SetVisibility {
        target: ObjectId::new("shape:2").unwrap(),
        value: Visibility::Visible,
    };
    timeline.nodes.push(child);
    let digest = q.page.document.semantic_digest().unwrap();
    let binding = PlaybackBinding {
        session: PlaybackSessionId::new("visible-author").unwrap(),
        revision: digest.clone(),
        generation: PlaybackGeneration::new(1),
    };
    let mut plan = PlaybackPagePlan::new(q, binding, TimelineLimits::default(), &|| false).unwrap();
    let before = plan.compile_frame(time(0), None, &|| false).unwrap();
    for ms in [500, 750, 999] {
        let frame = plan.compile_frame(time(ms), None, &|| false).unwrap();
        let objects = frame
            .placements
            .surfaces
            .iter()
            .flat_map(|s| &s.objects)
            .map(|p| p.object.as_str())
            .collect::<Vec<_>>();
        assert!(objects.is_empty(), "both shapes belong to the hidden group");
        assert_eq!(frame.profile, PROPERTY_PLAYBACK_PAGE_PROFILE);
        assert_eq!(frame.page.info.document_sha256, digest);
    }
    for ms in [1000, 2000, 0] {
        let frame = plan.compile_frame(time(ms), None, &|| false).unwrap();
        assert_eq!(
            serde_json::to_value(&frame.page.raster).unwrap(),
            serde_json::to_value(&before.page.raster).unwrap()
        );
    }
    assert_eq!(plan.document_sha256(), &digest);
}
