use mo_common::*;
use mo_presentation_compile::{playback::*, *};
use mo_timeline::*;
fn point(x: &str, y: &str) -> MotionPoint {
    MotionPoint {
        x: x.to_owned().try_into().unwrap(),
        y: y.to_owned().try_into().unwrap(),
    }
}
fn binding() -> PlaybackBinding {
    PlaybackBinding {
        session: PlaybackSessionId::new("motion-placement").unwrap(),
        revision: Digest::from_sha256([3; 32]),
        generation: PlaybackGeneration::new(1),
    }
}
#[test]
fn child_and_parent_motion_are_slide_axis_offsets_preserving_local_geometry() {
    let q: PageRenderRequest = serde_json::from_str(include_str!(
        "../../../fixtures/presentations/playback/page.json"
    ))
    .unwrap();
    let mut moving = q.clone();
    let timeline = moving.page.document.timelines.values_mut().next().unwrap();
    for (target, to) in [
        ("group:1", point("0.25", "-0.125")),
        ("shape:1", point("-0.125", "0.25")),
    ] {
        let mut node = timeline.nodes[0].clone();
        node.id = TimingNodeId::new(format!("motion:{target}")).unwrap();
        node.effect = Effect::MotionLine {
            target: ObjectId::new(target).unwrap(),
            from: point("0", "0"),
            to,
        };
        timeline.nodes.push(node);
    }
    let mut base =
        PlaybackPagePlan::new(q.clone(), binding(), TimelineLimits::default(), &|| false).unwrap();
    let mut plan = PlaybackPagePlan::new(
        moving.clone(),
        binding(),
        TimelineLimits::default(),
        &|| false,
    )
    .unwrap();
    let digest = moving.page.document.semantic_digest().unwrap();
    // Fixture duration is two seconds; rotations are sampled concurrently.
    for ms in [0, 500, 1000, 2000, 500, 0] {
        let at = RationalTime::new(ms, 1000).unwrap();
        let a = base.compile_frame(at, None, &|| false).unwrap();
        let b = plan.compile_frame(at, None, &|| false).unwrap();
        assert_eq!(b.profile, MOTION_PLAYBACK_PAGE_PROFILE);
        for (a, b) in a
            .placements
            .surfaces
            .iter()
            .flat_map(|s| &s.objects)
            .zip(b.placements.surfaces.iter().flat_map(|s| &s.objects))
        {
            assert_eq!(a.object, b.object);
            assert_eq!(a.affine.linear, b.affine.linear);
            assert_eq!(a.anchor, b.anchor);
            let (x, y) = if a.object.as_str() == "shape:1" {
                (1, 1)
            } else {
                (2, -1)
            };
            let expected = [
                i128::from(q.page.document.page_size.width.get()) * x * ms as i128 / 16000,
                i128::from(q.page.document.page_size.height.get()) * y * ms as i128 / 16000,
            ];
            assert_eq!(
                b.affine.translation.x.raw() - a.affine.translation.x.raw(),
                expected[0] << 32
            );
            assert_eq!(
                b.affine.translation.y.raw() - a.affine.translation.y.raw(),
                expected[1] << 32
            );
        }
        assert_eq!(b.page.info.document_sha256, digest);
    }
}
