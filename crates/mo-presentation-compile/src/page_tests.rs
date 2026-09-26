use super::*;
use mo_geometry::Fixed;
use mo_raster::{BackendReply, RasterBackend, RasterError};
use serde_json::{Value, json};
use std::cell::Cell;

#[test]
fn incremental_plans_equal_full_compilation_across_visual_and_metadata_edits() {
    use crate::incremental::PagePlanCache;
    let mut q = request(value());
    let mut cache = PagePlanCache::new(2, 1000);
    assert!(!cache.compile(&q, &|| false).unwrap().reused);
    let mutations: [fn(&mut PageRenderRequest); 5] = [
        |q| q.page.document.title = "renamed".into(),
        |q| {
            q.page
                .document
                .objects
                .values_mut()
                .next()
                .unwrap()
                .accessibility
                .title = "accessible".into()
        },
        |q| {
            q.page
                .document
                .objects
                .values_mut()
                .next()
                .unwrap()
                .transform
                .as_mut()
                .unwrap()
                .origin
                .x = mo_common::Emu::new(35)
        },
        |q| q.defaults.page_background.red = 70,
        |q| q.viewport.coordinate_tolerance = Fixed::from_raw(1 << 20),
    ];
    for (i, mutate) in mutations.into_iter().enumerate() {
        mutate(&mut q);
        let incremental = cache.compile(&q, &|| false).unwrap();
        assert_eq!(incremental.reused, i < 2);
        let full = compile_page(&q, &|| false).unwrap();
        assert_eq!(
            serde_json::to_value(incremental.plan).unwrap(),
            serde_json::to_value(full).unwrap()
        );
    }
    // A matching visual key cannot bypass validation or cancellation.
    assert!(cache.compile(&q, &|| true).is_err());
    q.page.document.slide_order.push(q.page.slide.clone());
    assert!(matches!(
        cache.compile(&q, &|| false),
        Err(PageError::Placement(CompileError::Document(_)))
    ));
}

#[test]
fn incremental_cache_budgets_and_page_scope_are_enforced() {
    let mut q = request(value());
    let other = mo_common::SlideId::new("slide:2").unwrap();
    let mut slide = q.page.document.slides[&q.page.slide].clone();
    slide.id = other.clone();
    slide.objects.clear();
    q.page.document.slide_order.push(other.clone());
    q.page.document.slides.insert(other.clone(), slide);
    let mut cache = crate::incremental::PagePlanCache::new(2, 1000);
    cache.compile(&q, &|| false).unwrap();
    q.page.slide = other.clone();
    cache.compile(&q, &|| false).unwrap();
    q.page
        .document
        .objects
        .values_mut()
        .next()
        .unwrap()
        .transform
        .as_mut()
        .unwrap()
        .origin
        .x = mo_common::Emu::new(50);
    let cached = cache.compile(&q, &|| false).unwrap();
    assert!(cached.reused, "unrelated page was invalidated");
    assert_eq!(
        serde_json::to_value(cached.plan).unwrap(),
        serde_json::to_value(compile_page(&q, &|| false).unwrap()).unwrap()
    );
    let mut bounded = crate::incremental::PagePlanCache::new(1, 1000);
    bounded.compile(&q, &|| false).unwrap();
    q.page.slide = mo_common::SlideId::new("slide:1").unwrap();
    bounded.compile(&q, &|| false).unwrap();
    q.page.slide = other;
    assert!(!bounded.compile(&q, &|| false).unwrap().reused);
    let mut no_commands = crate::incremental::PagePlanCache::new(10, 0);
    assert!(!no_commands.compile(&q, &|| false).unwrap().reused);
    assert!(!no_commands.compile(&q, &|| false).unwrap().reused);
}

fn value() -> Value {
    let mut d: Value = serde_json::from_str(include_str!(
        "../../../fixtures/presentations/basic-shape.json"
    ))
    .unwrap();
    d["pageSize"] = json!({"width":"800","height":"600"});
    let o = &mut d["objects"]["shape:1"];
    o["content"]["text"] = Value::Null;
    o["transform"]["origin"] = json!({"x":"10","y":"20"});
    o["transform"]["size"] = json!({"width":"200","height":"100"});
    o["appearance"] = json!({
        "fill":{"kind":"value","value":{"kind":"solid","color":{"kind":"srgb","rgba":{"red":80,"green":90,"blue":100,"alpha":255}}}},
        "stroke":{"kind":"value","value":{"kind":"none"}}
    });
    json!({"page":{"document":d,"slide":"slide:1"},
        "viewport":{"width":800,"height":600,"origin":{"x":"0","y":"0"},"scale":{"numerator":1,"denominator":1},"coordinateTolerance":"16777216","background":[0,0,0,0]},
        "defaults":{"themeColors":{},"pageBackground":{"red":255,"green":255,"blue":255,"alpha":255}}
    })
}
fn request(v: Value) -> PageRenderRequest {
    serde_json::from_value(v).unwrap()
}
fn stroke(v: &mut Value, join: Value) {
    v["page"]["document"]["objects"]["shape:1"]["appearance"]["stroke"] = json!({
        "kind":"value","value":{"kind":"solid","width":"12","cap":"square","join":join,
        "color":{"kind":"srgb","rgba":{"red":255,"green":0,"blue":0,"alpha":255}}}
    });
}
#[test]
fn fill_and_stroke_share_geometry_and_keep_paint_order() {
    let mut v = value();
    stroke(&mut v, json!({"kind":"round"}));
    let q = request(v);
    let before = serde_json::to_string(&q).unwrap();
    let plan = compile_page(&q, &|| false).unwrap();
    assert_eq!(plan.raster.scene.paths.len(), 1);
    assert_eq!(plan.raster.scene.transforms.len(), 1);
    assert_eq!(
        plan.paint_sources
            .iter()
            .map(|s| s.paint)
            .collect::<Vec<_>>(),
        [PagePaintKind::Fill, PagePaintKind::Stroke]
    );
    assert_eq!(
        plan.raster.scene.instances[0].path,
        plan.raster.scene.instances[1].path
    );
    assert!(plan.raster.scene.instances[0].stroke.is_none());
    assert_eq!(
        plan.raster.scene.instances[1].stroke.unwrap().width.raw(),
        12i128 << 32
    );
    assert_eq!(plan.info.generated_commands, 10);
    assert_eq!(serde_json::to_string(&q).unwrap(), before);
}
#[test]
fn stroke_only_and_rational_miter_have_separate_parameter_budget() {
    let mut v = value();
    stroke(&mut v, json!({"kind":"miter","limit":314159}));
    v["page"]["document"]["objects"]["shape:1"]["appearance"]["fill"] =
        json!({"kind":"value","value":{"kind":"none"}});
    let q = request(v);
    let plan = compile_page(&q, &|| false).unwrap();
    assert_eq!(plan.paint_sources.len(), 1);
    assert_eq!(plan.paint_sources[0].paint, PagePaintKind::Stroke);
    assert_eq!(plan.info.author_miter_limit_error_bound.raw(), 1);
    assert_eq!(plan.combined_coordinate_error_bound.raw(), 0);
    let mo_raster::StrokeJoin::Miter { limit } =
        plan.raster.scene.instances[0].stroke.unwrap().join
    else {
        panic!("lost miter")
    };
    assert!((limit.raw() * 100000 - 314159 * (1i128 << 32)).abs() <= 50000);
}
#[test]
fn miter_without_a_resolved_limit_is_not_silently_bevelled() {
    let mut v = value();
    stroke(&mut v, json!({"kind":"miter"}));
    let q = request(v);
    assert!(matches!(
        compile_page(&q, &|| false),
        Err(PageError::Unsupported {
            feature: PageFeature::UnresolvedStrokeParameters,
            ..
        })
    ));
    assert_eq!(
        serde_json::to_value(&q).unwrap()["page"]["document"]["objects"]["shape:1"]["appearance"]["stroke"]
            ["value"]["join"],
        json!({"kind":"miter"})
    );
}
#[test]
fn unresolved_stroke_and_unrepresentable_ink_fail_before_render() {
    for mode in 0..4 {
        let mut v = value();
        stroke(&mut v, json!({"kind":"round"}));
        let s = &mut v["page"]["document"]["objects"]["shape:1"]["appearance"]["stroke"]["value"];
        match mode {
            0 => {
                s.as_object_mut().unwrap().remove("cap");
            }
            1 => {
                s.as_object_mut().unwrap().remove("join");
            }
            2 => {
                s["join"] = json!({"kind":"miter","limit":102400001});
            }
            _ => {
                s["width"] = json!("32768");
            }
        }
        assert!(render_page(&request(v), &mut Never, &|| false).is_err());
    }
}
#[test]
fn plan_certifies_device_lowering_and_interns_identical_shapes_and_transforms() {
    let mut v = value();
    let d = &mut v["page"]["document"];
    let mut copy = d["objects"]["shape:1"].clone();
    copy["id"] = json!("shape:2");
    d["objects"]["shape:2"] = copy;
    d["slides"]["slide:1"]["objects"] = json!(["shape:1", "shape:2"]);
    let q = request(v);
    let before = serde_json::to_string(&q).unwrap();
    let plan = compile_page(&q, &|| false).unwrap();
    assert_eq!(plan.raster.scene.paths.len(), 1);
    assert_eq!(plan.raster.scene.transforms.len(), 1);
    assert_eq!(plan.paint_sources.len(), 2);
    assert_eq!(plan.device_work.compiled_paths, 1);
    assert_eq!(plan.combined_coordinate_error_bound, Fixed::ZERO);
    assert_eq!(before, serde_json::to_string(&q).unwrap());
}
#[test]
fn analytic_shapes_reserve_error_before_device_lowering() {
    for geometry in [
        json!({"kind":"ellipse"}),
        json!({"kind":"roundRectangle","radius":"37"}),
    ] {
        let mut v = value();
        v["page"]["document"]["objects"]["shape:1"]["content"]["geometry"] = geometry;
        v["page"]["document"]["objects"]["shape:1"]["transform"]["rotation"] = json!(1234567);
        let q = request(v);
        let plan = compile_page(&q, &|| false).unwrap();
        assert!(plan.info.curve_segments >= 16);
        assert!(plan.info.author_coordinate_error_bound > Fixed::ZERO);
        assert!(plan.info.geometry_coordinate_error_bound > Fixed::ZERO);
        assert!(plan.combined_coordinate_error_bound <= q.viewport.coordinate_tolerance);
        assert!(plan.raster.viewport.coordinate_tolerance < q.viewport.coordinate_tolerance);
        assert_eq!(
            plan.combined_coordinate_error_bound.raw(),
            plan.info.author_coordinate_error_bound.raw()
                + plan.info.geometry_coordinate_error_bound.raw()
                + plan.device_work.combined_coordinate_error_bound.raw()
        );
    }
}
struct Never;
impl RasterBackend for Never {
    fn raster(&mut self, _: &[u32]) -> Result<BackendReply, RasterError> {
        panic!("rejected page reached backend")
    }
    fn invalidate(&mut self) {
        panic!("unstarted component invalidated")
    }
}
#[test]
fn unsupported_or_unrepresentable_pages_never_call_backend() {
    let mut text = value();
    text["page"]["document"]["objects"]["shape:1"]["content"]["text"] =
        serde_json::from_str::<Value>(include_str!(
            "../../../fixtures/presentations/basic-shape.json"
        ))
        .unwrap()["objects"]["shape:1"]["content"]["text"]
            .clone();
    let mut inherited = value();
    inherited["page"]["document"]["objects"]["shape:1"]["appearance"] = json!({});
    let mut viewport = value();
    viewport["viewport"]["width"] = json!(799);
    let mut radius = value();
    radius["page"]["document"]["objects"]["shape:1"]["content"]["geometry"] =
        json!({"kind":"roundRectangle","radius":"51"});
    let mut range = value();
    range["page"]["document"]["objects"]["shape:1"]["transform"]["origin"]["x"] =
        json!("1000000000");
    for v in [text, inherited, viewport, radius, range] {
        let q = request(v);
        assert!(compile_page(&q, &|| false).is_err());
        assert!(render_page(&q, &mut Never, &|| false).is_err());
    }
}
#[test]
fn explicit_none_background_differs_from_inherited_host_default() {
    let q = request(value());
    assert_eq!(compile_page(&q, &|| false).unwrap().paint_sources.len(), 1);
    let mut v = value();
    v["page"]["document"]["slides"]["slide:1"]["background"] = json!({"kind":"inherit"});
    let plan = compile_page(&request(v), &|| false).unwrap();
    assert_eq!(plan.paint_sources.len(), 2);
    assert!(plan.paint_sources[0].object.is_none());
    assert_eq!(
        plan.raster.scene.instances[0].brush,
        mo_raster::Brush::Solid { rgba: [255; 4] }
    );
    assert_eq!(plan.raster.scene.instances[0].transform, None);
}
#[test]
fn selected_page_cancellation_points_publish_no_partial_plan() {
    let mut v = value();
    v["page"]["document"]["objects"]["shape:1"]["content"]["geometry"] = json!({"kind":"ellipse"});
    let q = request(v);
    let count = Cell::new(0);
    compile_page(&q, &|| {
        count.set(count.get() + 1);
        false
    })
    .unwrap();
    let total = count.get();
    assert!(total > 100);
    for stop in (0..total).step_by(97).chain([total - 1]) {
        let calls = Cell::new(0);
        let result = render_page(&q, &mut Never, &|| {
            let n = calls.get();
            calls.set(n + 1);
            n == stop
        });
        assert!(matches!(
            result,
            Err(PageError::Placement(CompileError::Cancelled)
                | PageError::Raster(RasterError::Cancelled))
        ));
    }
}
