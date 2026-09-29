//! The native roundtrip is a verification oracle only; author compilation never
//! serializes a PPTX to render. Both paths must make the same curve decisions.
use mo_common::*;
use mo_presentation_compile::{playback::*, source_playback::*, source_resource_page::*, *};
use mo_presentation_model::*;
use mo_presentation_source::{author::*, source::*};
use mo_timeline::*;

struct NoImages;
impl mo_image::ImageDecoder for NoImages {
    fn decode(&mut self, _: &[u8]) -> Result<mo_image::DecoderReply, mo_image::ImageError> {
        panic!("owned vector-only document must not request image decoding")
    }
    fn invalidate(&mut self) {}
}
fn t(n: i64, d: u32) -> RationalTime {
    RationalTime::new(n, d).unwrap()
}
fn canonical_scene(mut scene: mo_render::DrawScene) -> serde_json::Value {
    use mo_geometry::PathCommand as C;
    for path in &mut scene.paths {
        let [C::Move { to: start }, rest @ ..] = path.commands.as_slice() else {
            panic!("expected one owned closed contour");
        };
        let mut pen = *start;
        let mut edges = Vec::new();
        for command in rest {
            let end = match command {
                C::Line { to } | C::Quadratic { to, .. } | C::Cubic { to, .. } => *to,
                C::Close => {
                    if pen != *start {
                        edges.push((pen, C::Line { to: *start }));
                    }
                    continue;
                }
                C::Move { .. } => panic!("unexpected second contour"),
            };
            edges.push((pen, command.clone()));
            pen = end;
        }
        // Native ellipse presets start on a different quadrant. Closed contour
        // rotation changes no segment/control point, orientation, style or fill.
        let first = edges
            .iter()
            .enumerate()
            .min_by_key(|(_, (p, _))| (p.x.raw(), p.y.raw()))
            .unwrap()
            .0;
        edges.rotate_left(first);
        path.commands = std::iter::once(C::Move { to: edges[0].0 })
            .chain(edges.into_iter().map(|(_, c)| c))
            .chain([C::Close])
            .collect();
    }
    serde_json::to_value(scene).unwrap()
}
#[test]
fn nested_animated_group_uses_the_same_curve_paths_before_and_after_native_export() {
    let mut q: PageRenderRequest = serde_json::from_str(include_str!(
        "../../../fixtures/presentations/playback/page.json"
    ))
    .unwrap();
    let doc = &mut q.page.document;
    let child = ObjectId::new("group:1").unwrap();
    let outer = ObjectId::new("group:outer").unwrap();
    let mut group = doc.objects[&child].clone();
    group.id = outer.clone();
    group.content = ObjectContent::Group {
        children: vec![child.clone()],
        viewport: doc.page_size,
    };
    let transform = group.transform.as_mut().unwrap();
    transform.origin = Point {
        x: Emu::new(150000),
        y: Emu::new(180000),
    };
    transform.size = Size {
        width: Emu::new(2800000),
        height: Emu::new(2000000),
    };
    transform.rotation = 2700000;
    transform.flip_horizontal = false;
    transform.flip_vertical = true;
    doc.objects.get_mut(&child).unwrap().parent = ContainerId::Group(outer.clone());
    doc.objects.insert(outer.clone(), group);
    doc.slides.get_mut(&q.page.slide).unwrap().objects = vec![outer];
    let timeline = doc.timelines.get_mut(&q.page.slide).unwrap();
    let mut scale = timeline.nodes[0].clone();
    scale.id = TimingNodeId::new("scale:group").unwrap();
    scale.effect = Effect::Scale {
        target: child,
        from: ScaleValue {
            x: 100000,
            y: 100000,
        },
        to: ScaleValue {
            x: 200000,
            y: 50000,
        },
    };
    timeline.nodes.push(scale);
    let original = doc.clone();
    let defaults: serde_json::Value = serde_json::from_str(include_str!(
        "../../../fixtures/presentations/native-export/request.json"
    ))
    .unwrap();
    let defaults: ExportDefaults = serde_json::from_value(defaults["defaults"].clone()).unwrap();
    let plan = AuthorPlan::new(doc, &defaults, Default::default(), &|| false).unwrap();
    let output =
        mo_pptx::export_plan_to(&plan, &NoResources, Vec::new(), Default::default(), &|| {
            false
        })
        .unwrap();
    let index = inspect_source(output.package(), SourceLimits::default(), &|| false).unwrap();
    let request = serde_json::from_value(serde_json::json!({
        "expectedSourceSha256": index.source_sha256,
        "slide": index.slides[0].part,
        "profile": "drawingml-static-solid-page-v1-draft",
        "colorContext": {"systemColors": {}, "placeholder": null},
        "viewport": q.viewport
    }))
    .unwrap();
    let binding = PlaybackBinding {
        session: PlaybackSessionId::new("roundtrip").unwrap(),
        revision: index.source_sha256.clone(),
        generation: PlaybackGeneration::new(1),
    };
    let mut source = SourcePlaybackPlan::new(
        output.package(),
        &index,
        request,
        binding.clone(),
        SourceLimits::default(),
        TimelineLimits::default(),
        &|| false,
    )
    .unwrap();
    let mut author =
        PlaybackPagePlan::new(q.clone(), binding, TimelineLimits::default(), &|| false).unwrap();
    for at in [
        t(0, 1),
        t(1, 7),
        t(1, 2),
        t(1, 1),
        t(2, 1),
        t(7, 3),
        t(1, 7),
    ] {
        let a = author.compile_frame(at, None, &|| false).unwrap();
        let s = source
            .sample(at, None, &|| false)
            .unwrap()
            .prepare(
                output.package(),
                &index,
                &mut NoImages,
                None,
                ResourcePageOptions {
                    selection: images::ImageSourceSelection::EmbeddedSnapshot,
                    sampling: mo_raster::ImageSampling::Nearest,
                    text_limits: Default::default(),
                },
                &|| false,
            )
            .unwrap()
            .plan(&|| false)
            .unwrap();
        assert_eq!(
            a.page.info.curve_segments, s.page.info.arc_segments,
            "{at:?}"
        );
        assert_eq!(
            canonical_scene(a.page.raster.scene),
            canonical_scene(s.page.raster.scene),
            "{at:?}"
        );
    }
    assert_eq!(q.page.document, original);
}
