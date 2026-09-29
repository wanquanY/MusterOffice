use mo_common::*;
use mo_presentation_compile::{playback::*, *};
use mo_timeline::*;
fn t(ms: i64) -> RationalTime {
    RationalTime::new(ms, 1000).unwrap()
}
#[test]
fn overlapping_children_are_isolated_as_one_group_with_nested_object_opacity() {
    let mut q: PageRenderRequest = serde_json::from_str(include_str!(
        "../../../fixtures/presentations/playback/page.json"
    ))
    .unwrap();
    let timeline = q.page.document.timelines.values_mut().next().unwrap();
    timeline.nodes.truncate(1);
    let n = &mut timeline.nodes[0];
    n.start = TimeCondition::At { offset: t(0) }.into();
    n.duration = t(1000);
    n.fill = FillMode::Remove;
    n.repeat_milli = 1000.into();
    n.time_transform = None;
    n.effect = Effect::Fade {
        target: ObjectId::new("group:1").unwrap(),
        transition: FadeTransition::In,
    };
    let mut child = n.clone();
    child.id = TimingNodeId::new("child-fade").unwrap();
    child.effect = Effect::Fade {
        target: ObjectId::new("shape:2").unwrap(),
        transition: FadeTransition::Out,
    };
    timeline.nodes.push(child);
    let digest = q.page.document.semantic_digest().unwrap();
    let binding = PlaybackBinding {
        session: PlaybackSessionId::new("fade-group").unwrap(),
        revision: digest.clone(),
        generation: PlaybackGeneration::new(1),
    };
    let mut plan = PlaybackPagePlan::new(q, binding, TimelineLimits::default(), &|| false).unwrap();
    let base = plan.compile_frame(t(1000), None, &|| false).unwrap();
    for ms in [250, 500, 750, 0, 500] {
        let f = plan.compile_frame(t(ms), None, &|| false).unwrap();
        let scene = &f.page.raster.scene;
        let groups = &scene.opacity_groups;
        assert_eq!(groups.len(), 2);
        assert_eq!(groups[0].first_draw, 0);
        assert_eq!(groups[0].end_draw, scene.instances.len() as u32);
        let child_paints: Vec<_> = f
            .page
            .paint_sources
            .iter()
            .enumerate()
            .filter(|(_, p)| p.object.as_ref().is_some_and(|o| o.as_str() == "shape:2"))
            .map(|(i, _)| i as u32)
            .collect();
        assert_eq!(groups[1].first_draw, child_paints[0]);
        assert_eq!(groups[1].end_draw, child_paints.last().unwrap() + 1);
        assert_eq!(groups[0].opacity, ((ms as u32 * 65535 + 500) / 1000) as u16);
        assert_eq!(
            groups[1].opacity,
            (((1000 - ms) as u32 * 65535 + 500) / 1000) as u16
        );
        assert_eq!(
            serde_json::to_value(&scene.instances).unwrap(),
            serde_json::to_value(&base.page.raster.scene.instances).unwrap()
        );
        assert_eq!(f.page.info.document_sha256, digest);
    }
    assert!(base.page.raster.scene.opacity_groups.is_empty());
}
