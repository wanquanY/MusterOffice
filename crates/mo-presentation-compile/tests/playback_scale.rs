use mo_common::*;
use mo_presentation_compile::{playback::*, *};
use mo_timeline::*;
fn t(n: i64, d: u32) -> RationalTime {
    RationalTime::new(n, d).unwrap()
}
fn binding() -> PlaybackBinding {
    PlaybackBinding {
        session: PlaybackSessionId::new("scale-placement").unwrap(),
        revision: Digest::from_sha256([3; 32]),
        generation: PlaybackGeneration::new(1),
    }
}
fn request() -> PageRenderRequest {
    serde_json::from_str(include_str!(
        "../../../fixtures/presentations/playback/page.json"
    ))
    .unwrap()
}
fn add_scale(q: &mut PageRenderRequest, target: &str, from: ScaleValue, to: ScaleValue) {
    let timeline = q.page.document.timelines.values_mut().next().unwrap();
    let mut n = timeline.nodes[0].clone();
    n.id = TimingNodeId::new(format!("scale:{target}")).unwrap();
    n.effect = Effect::Scale {
        target: ObjectId::new(target).unwrap(),
        from,
        to,
    };
    timeline.nodes.push(n);
}
#[test]
fn group_scale_changes_visual_space_around_the_same_center_and_preserves_source_layout() {
    let mut q = request();
    add_scale(
        &mut q,
        "group:1",
        ScaleValue {
            x: 100000,
            y: 100000,
        },
        ScaleValue {
            x: 200000,
            y: 50000,
        },
    );
    let digest = q.page.document.semantic_digest().unwrap();
    let mut plan =
        PlaybackPagePlan::new(q.clone(), binding(), TimelineLimits::default(), &|| false).unwrap();
    for (n, d) in [(0, 1), (1, 2), (1, 1), (2, 1), (1, 1), (0, 1)] {
        let frame = plan.compile_frame(t(n, d), None, &|| false).unwrap();
        let mut control = q.clone();
        control
            .page
            .document
            .timelines
            .values_mut()
            .next()
            .unwrap()
            .nodes
            .pop();
        let transform = control
            .page
            .document
            .objects
            .get_mut(&ObjectId::new("group:1").unwrap())
            .unwrap()
            .transform
            .as_mut()
            .unwrap();
        let w = transform.size.width.get();
        let h = transform.size.height.get();
        let nw = w * (2 * i64::from(d) + n) / (2 * i64::from(d));
        let nh = h * (4 * i64::from(d) - n) / (4 * i64::from(d));
        transform.origin.x = Emu::new(transform.origin.x.get() + (w - nw) / 2);
        transform.origin.y = Emu::new(transform.origin.y.get() + (h - nh) / 2);
        transform.size.width = Emu::new(nw);
        transform.size.height = Emu::new(nh);
        let expected =
            PlaybackPagePlan::new(control, binding(), TimelineLimits::default(), &|| false)
                .unwrap()
                .compile_frame(t(n, d), None, &|| false)
                .unwrap();
        assert_eq!(
            serde_json::to_value(&frame.page.raster).unwrap(),
            serde_json::to_value(&expected.page.raster).unwrap()
        );
        assert_eq!(frame.profile, TRANSFORM_PLAYBACK_PAGE_PROFILE);
        assert_eq!(frame.page.info.document_sha256, digest);
        assert_eq!(
            frame.placements.surfaces[0].objects[0].anchor,
            expected.placements.surfaces[0].objects[0].anchor
        );
    }
    assert_eq!(plan.document_sha256(), &digest);
}
#[test]
fn independent_scale_and_rotation_keep_fractional_samples_and_collapsed_axes_computable() {
    let mut q = request();
    add_scale(
        &mut q,
        "shape:1",
        ScaleValue { x: 0, y: 0 },
        ScaleValue {
            x: 200000,
            y: 100000,
        },
    );
    let mut plan =
        PlaybackPagePlan::new(q.clone(), binding(), TimelineLimits::default(), &|| false).unwrap();
    for at in [t(0, 1), t(1, 7), t(1, 1), t(2, 1)] {
        let frame = plan.compile_frame(at, None, &|| false).unwrap();
        let id = ObjectId::new("shape:1").unwrap();
        assert_eq!(
            frame.frame.state.scales[&id].x.denominator,
            if at == t(1, 7) { "7" } else { "1" }
        );
        assert!(frame.page.combined_coordinate_error_bound <= q.viewport.coordinate_tolerance);
        if at == t(0, 1) {
            let p = frame
                .placements
                .surfaces
                .iter()
                .flat_map(|s| &s.objects)
                .find(|p| p.object == id)
                .unwrap();
            assert!(p.affine.linear.iter().all(|v| v.raw() == 0));
        }
    }
}
