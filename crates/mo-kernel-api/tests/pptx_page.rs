use mo_kernel_api::*;
use mo_opc::{Package, PackageLimits, PartName, RewritePlan};
use mo_pptx::source::{SourceIndex, inspect_source};
use mo_presentation_compile::source_page;
use mo_raster::{BackendReply, RasterBackend, RasterError};
use serde_json::{Value, json};
use std::sync::OnceLock;
const SLIDE: &str = "/ppt/slides/slide1.xml";
const LAYOUT: &str = "/ppt/slideLayouts/slideLayout2.xml";
const MASTER: &str = "/ppt/slideMasters/slideMaster2.xml";
fn base() -> &'static [u8] {
    static BASE: OnceLock<Vec<u8>> = OnceLock::new();
    BASE.get_or_init(|| {
        export_pptx_json(
            include_str!("../../../fixtures/presentations/native-export/request.json"),
            include_bytes!("../../../fixtures/presentations/native-export/resources.bin"),
        )
        .unwrap()
    })
}
fn package(b: &[u8]) -> Package<&[u8]> {
    Package::open(b, b.len() as u64, PackageLimits::default(), &|| false).unwrap()
}
fn source(slide: &str, layout: &str, master: &str, flags: [Option<bool>; 2]) -> Vec<u8> {
    let p = package(base());
    let mut plan = RewritePlan::new();
    for (part, objects, flag) in [
        (SLIDE, slide, flags[0]),
        (LAYOUT, layout, flags[1]),
        (MASTER, master, None),
    ] {
        let part = PartName::new(part).unwrap();
        let mut xml = String::from_utf8(p.read_part(&part, 1 << 20, &|| false).unwrap()).unwrap();
        let a = xml.find("<p:spTree>").unwrap();
        let b = xml.find("</p:spTree>").unwrap() + 11;
        xml.replace_range(a..b, &format!("<p:spTree><p:nvGrpSpPr><p:cNvPr id=\"1\" name=\"\"/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr><p:grpSpPr/>{objects}</p:spTree>"));
        if let Some(flag) = flag {
            let root = if part.as_str() == SLIDE {
                "<p:sld "
            } else {
                "<p:sldLayout "
            };
            xml = xml.replacen(root, &format!("{root}showMasterSp=\"{flag}\" "), 1);
        }
        plan.replace_part(part, xml.into_bytes()).unwrap();
    }
    plan.to_bytes(&p, &|| false).unwrap()
}
fn edit(b: &[u8], part: &str, f: impl FnOnce(String) -> String) -> Vec<u8> {
    let p = package(b);
    let name = PartName::new(part).unwrap();
    let s = String::from_utf8(p.read_part(&name, 1 << 20, &|| false).unwrap()).unwrap();
    let mut plan = RewritePlan::new();
    plan.replace_part(name, f(s).into_bytes()).unwrap();
    plan.to_bytes(&p, &|| false).unwrap()
}
fn shape(id: u32, color: &str) -> String {
    format!(
        "<p:sp><p:nvSpPr><p:cNvPr id=\"{id}\" name=\"Owned page test\"/><p:cNvSpPr/><p:nvPr/></p:nvSpPr><p:spPr><a:xfrm><a:off x=\"1000000\" y=\"1000000\"/><a:ext cx=\"2000000\" cy=\"1000000\"/></a:xfrm><a:prstGeom prst=\"rect\"/><a:solidFill><a:srgbClr val=\"{color}\"/></a:solidFill><a:ln><a:noFill/></a:ln></p:spPr></p:sp>"
    )
}
fn index(b: &[u8]) -> SourceIndex {
    inspect_source(&package(b), SourceLimits::default(), &|| false).unwrap()
}
fn request(i: &SourceIndex) -> SourcePageRequest {
    serde_json::from_value(json!({"expectedSourceSha256":i.source_sha256,"slide":SLIDE,"profile":"drawingml-static-solid-page-v1-draft","colorContext":{"systemColors":{},"placeholder":null},"viewport":{"width":800,"height":450,"origin":{"x":"0","y":"0"},"scale":{"numerator":800,"denominator":12192000},"coordinateTolerance":"16777216","background":[255,255,255,255]}})).unwrap()
}
fn plan(b: &[u8]) -> Value {
    let i = index(b);
    let before = i.clone();
    let p = source_page::compile(&i, &request(&i), &|| false).unwrap();
    assert_eq!(i, before);
    serde_json::to_value(p).unwrap()
}
#[test]
fn native_page_paints_master_layout_slide_and_keeps_source_owners() {
    let b = source(
        &shape(22, "FF0000"),
        &shape(22, "00FF00"),
        &shape(22, "0000FF"),
        [None, None],
    );
    let p = plan(&b);
    assert_eq!(p["bindings"].as_array().unwrap().len(), 4);
    assert_eq!(p["paintSources"].as_array().unwrap().len(), 4);
    for (n, part, color) in [
        (1, MASTER, json!([0, 0, 255, 255])),
        (2, LAYOUT, json!([0, 255, 0, 255])),
        (3, SLIDE, json!([255, 0, 0, 255])),
    ] {
        assert_eq!(
            p["bindings"][n]["location"],
            json!({"part":part,"object":22})
        );
        assert_eq!(p["bindings"][n]["drawingSurface"], part);
        assert_eq!(p["raster"]["scene"]["instances"][n]["brush"]["rgba"], color);
    }
    // All three object paths and transforms are interned, preserving draws.
    assert_eq!(p["raster"]["scene"]["paths"].as_array().unwrap().len(), 2);
    assert_eq!(
        p["raster"]["scene"]["transforms"].as_array().unwrap().len(),
        1
    );
}
#[test]
fn show_master_shapes_controls_inherited_layers_and_hidden_slides_remain_previewable() {
    for (flags, expected) in [
        ([None, None], vec![true, true, true]),
        ([Some(false), None], vec![false, false, true]),
        ([None, Some(false)], vec![false, true, true]),
        ([Some(true), Some(true)], vec![true, true, true]),
    ] {
        let b = source(
            &shape(22, "FF0000"),
            &shape(23, "00FF00"),
            &shape(24, "0000FF"),
            flags,
        );
        let b = edit(&b, SLIDE, |s| s.replace("show=\"1\"", "show=\"0\""));
        let p = plan(&b);
        assert_eq!(p["info"]["hiddenSlide"], true);
        let visible: Vec<_> = p["info"]["layers"]
            .as_array()
            .unwrap()
            .iter()
            .map(|l| l["visible"].as_bool().unwrap())
            .collect();
        assert_eq!(visible, expected);
        assert_eq!(
            p["bindings"].as_array().unwrap().len(),
            1 + expected.iter().filter(|b| **b).count()
        );
    }
}
#[test]
fn hidden_groups_skip_unknown_children_without_disabling_visible_siblings() {
    let child = shape(23, "0000FF").replace("<p:spPr>", "<p:spPr future=\"1\">");
    let group = format!(
        "<p:grpSp><p:nvGrpSpPr><p:cNvPr id=\"2\" name=\"hidden\" hidden=\"1\"/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr><p:grpSpPr/>{child}</p:grpSp>"
    );
    let b = source(&(group + &shape(24, "FF0000")), "", "", [None, None]);
    let p = plan(&b);
    assert_eq!(p["info"]["layers"][2]["hiddenObjects"], json!([2, 23]));
    assert_eq!(p["info"]["layers"][2]["objects"], json!([24]));
    assert_eq!(p["bindings"].as_array().unwrap().len(), 2);
}
#[test]
fn master_art_keeps_its_color_map_while_source_owners_remain_physical() {
    let master = shape(22, "FF0000").replace(
        "<a:srgbClr val=\"FF0000\"/>",
        "<a:schemeClr val=\"accent1\"/>",
    );
    let b = source("", "", &master, [None, None]);
    let b = edit(&b, SLIDE, |s| {
        s.replace("<a:masterClrMapping/>","<a:overrideClrMapping bg1=\"lt1\" tx1=\"dk1\" bg2=\"lt2\" tx2=\"dk2\" accent1=\"accent2\" accent2=\"accent2\" accent3=\"accent3\" accent4=\"accent4\" accent5=\"accent5\" accent6=\"accent6\" hlink=\"hlink\" folHlink=\"folHlink\"/>")
    });
    let p = plan(&b);
    assert_eq!(
        p["raster"]["scene"]["instances"][1]["brush"]["rgba"],
        json!([27, 114, 232, 255])
    );
    assert_eq!(
        p["bindings"][1]["fill"]["style"]["fill"]["declaredBy"]["owner"]["part"],
        MASTER
    );
}
#[test]
fn master_use_background_fill_targets_the_consuming_slide_background() {
    let b = source(
        "",
        "",
        &shape(22, "FF0000").replace("<p:sp>", "<p:sp useBgFill=\"1\">"),
        [None, None],
    );
    let b = edit(&b, SLIDE, |s| {
        s.replace("<p:spTree>","<p:bg><p:bgPr><a:solidFill><a:srgbClr val=\"123456\"/></a:solidFill></p:bgPr></p:bg><p:spTree>")
    });
    let p = plan(&b);
    assert_eq!(
        p["raster"]["scene"]["instances"][1]["brush"]["rgba"],
        json!([18, 52, 86, 255])
    );
    assert_eq!(
        p["bindings"][1]["fill"]["style"]["redirects"][0]["target"]["part"],
        SLIDE
    );
}
#[derive(Default)]
struct Backend {
    calls: u32,
    cancelled: bool,
}
impl RasterBackend for Backend {
    fn raster(&mut self, frame: &[u32]) -> Result<BackendReply, RasterError> {
        self.calls += 1;
        self.cancelled = true;
        Ok(BackendReply {
            status: 0,
            pixels: vec![0; frame[2] as usize * frame[3] as usize * 4],
        })
    }
    fn invalidate(&mut self) {
        panic!("valid fixture backend should not be invalidated")
    }
}
#[test]
fn unsupported_visible_content_aborts_before_backend_and_never_returns_pixels() {
    let normal = shape(22, "FF0000");
    for (s,kind) in [
        (normal.replace("<p:spPr>","<p:spPr bwMode=\"gray\">"),"visual"),
        (normal.replace("</p:spPr>","<a:effectLst><a:glow rad=\"50000\"><a:srgbClr val=\"000000\"/></a:glow></a:effectLst></p:spPr>"),"effects"),
        (normal.replace("</p:sp>","<p:txBody><a:bodyPr/><a:lstStyle/><a:p/></p:txBody></p:sp>"),"text"),
        (normal.replace("<a:prstGeom prst=\"rect\"/>",""),"geometry"),
        (normal.replace("<a:solidFill><a:srgbClr val=\"FF0000\"/></a:solidFill>","<a:pattFill prst=\"cross\"/>"),"fill"),
    ] {
        let b=source(&s,"","",[None,None]); let i=index(&b); let q=request(&i); let mut backend=Backend::default();
        let (response,pixels)=render_pptx_page(&q,b.as_slice(),b.len() as u64,SourceLimits::default(),&mut backend,&||false);
        let r=serde_json::to_value(response).unwrap(); assert_eq!(r["error"]["code"],"MAPPING_NOT_IMPLEMENTED","{r}"); assert_eq!(r["error"]["issue"]["kind"],kind,"{r}"); assert_eq!(r["error"]["location"]["object"],22); assert!(pixels.is_empty()); assert_eq!(backend.calls,0);
    }
}
#[test]
fn native_geometry_and_group_coordinates_are_applied_once() {
    let child = shape(22, "FF0000");
    let group = format!(
        "<p:grpSp><p:nvGrpSpPr><p:cNvPr id=\"2\" name=\"group\"/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr><p:grpSpPr><a:xfrm><a:off x=\"2000000\" y=\"1000000\"/><a:ext cx=\"4000000\" cy=\"2000000\"/><a:chOff x=\"1000000\" y=\"1000000\"/><a:chExt cx=\"2000000\" cy=\"1000000\"/></a:xfrm></p:grpSpPr>{child}</p:grpSp>"
    );
    let p = plan(&source(&group, "", "", [None, None]));
    let t = &p["raster"]["scene"]["transforms"][0]["affine"];
    assert_eq!(t["linear"], json!(["8589934592", "0", "0", "8589934592"]));
    assert_eq!(
        t["translation"],
        json!({"x":(4000000i128<<32).to_string(),"y":(2000000i128<<32).to_string()})
    );
    assert_eq!(
        p["raster"]["scene"]["paths"][1]["commands"][0]["to"],
        json!({"x":(-1000000i128<<32).to_string(),"y":(-500000i128<<32).to_string()})
    );
}
#[test]
fn source_conflict_cancellation_and_bad_viewport_do_not_reach_backend() {
    let b = source(&shape(22, "FF0000"), "", "", [None, None]);
    let i = index(&b);
    let mut q = request(&i);
    let mut backend = Backend::default();
    let (response, pixels) = render_pptx_page(
        &q,
        b.as_slice(),
        b.len() as u64,
        SourceLimits::default(),
        &mut backend,
        &|| true,
    );
    assert_eq!(
        serde_json::to_value(response).unwrap()["error"]["code"],
        "CANCELLED"
    );
    assert!(pixels.is_empty());
    q.viewport.width += 1;
    let r = compile_pptx_page(
        &q,
        b.as_slice(),
        b.len() as u64,
        SourceLimits::default(),
        &|| false,
    );
    assert_eq!(
        serde_json::to_value(r).unwrap()["error"]["code"],
        "INPUT_INVALID"
    );
    q = request(&i);
    q.expected_source_sha256 = mo_common::Digest::from_sha256([0; 32]);
    let r = compile_pptx_page(
        &q,
        b.as_slice(),
        b.len() as u64,
        SourceLimits::default(),
        &|| false,
    );
    assert_eq!(
        serde_json::to_value(r).unwrap()["error"]["code"],
        "SOURCE_CONFLICT"
    );
    assert_eq!(backend.calls, 0);
}
#[test]
fn template_placeholders_supply_properties_without_extra_paint() {
    let master = shape(22, "FF0000").replace(
        "<p:nvPr/>",
        "<p:nvPr><p:ph type=\"body\" idx=\"1\"/></p:nvPr>",
    );
    let p = plan(&source("", "", &master, [None, None]));
    assert_eq!(p["info"]["layers"][0]["templatePlaceholders"], json!([22]));
    assert_eq!(p["bindings"].as_array().unwrap().len(), 1);
}

fn override_slide_map(s: String) -> String {
    s.replace("<a:masterClrMapping/>", "<a:overrideClrMapping bg1=\"lt1\" tx1=\"dk1\" bg2=\"lt2\" tx2=\"dk2\" accent1=\"accent2\" accent2=\"accent2\" accent3=\"accent3\" accent4=\"accent4\" accent5=\"accent5\" accent6=\"accent6\" hlink=\"hlink\" folHlink=\"folHlink\"/>")
}
#[test]
fn mixed_background_and_object_colors_share_budget_without_leaking_context() {
    let scheme = shape(23, "FF0000").replace(
        "<a:srgbClr val=\"FF0000\"/>",
        "<a:schemeClr val=\"accent1\"/>",
    );
    let bg = scheme
        .replace("id=\"23\"", "id=\"22\"")
        .replace("<p:sp>", "<p:sp useBgFill=\"1\">");
    let line = scheme.replace("id=\"23\"","id=\"24\"").replace("<a:ln><a:noFill/></a:ln>","<a:ln w=\"100000\"><a:solidFill><a:schemeClr val=\"accent1\"/></a:solidFill><a:round/></a:ln>");
    let b = source("", "", &(bg + &scheme + &line), [None, None]);
    let b = edit(&b, SLIDE, |s| {
        override_slide_map(s).replace("<p:spTree>","<p:bg><p:bgPr><a:solidFill><a:schemeClr val=\"accent1\"/></a:solidFill></p:bgPr></p:bg><p:spTree>")
    });
    let p = plan(&b);
    assert_eq!(
        p["bindings"][1]["fill"]["contextOverride"]["surface"],
        SLIDE
    );
    assert_eq!(
        p["bindings"][1]["fill"]["colors"]["color"]["outcome"]["rgba8"],
        json!([38, 185, 154, 255])
    );
    for n in [2, 3] {
        assert!(p["bindings"][n]["fill"].get("contextOverride").is_none());
        assert_eq!(
            p["bindings"][n]["fill"]["colors"]["color"]["outcome"]["rgba8"],
            json!([27, 114, 232, 255])
        );
    }
    assert_eq!(
        p["bindings"][3]["line"]["paint"]["outcome"]["rgba8"],
        json!([27, 114, 232, 255])
    );
}
#[test]
fn slide_placeholder_inherits_paint_without_inheriting_master_drawing_context() {
    let ph = |s: String| {
        s.replace(
            "<p:nvPr/>",
            "<p:nvPr><p:ph type=\"body\" idx=\"1\"/></p:nvPr>",
        )
    };
    let master = ph(shape(24, "FF0000")).replace(
        "<a:srgbClr val=\"FF0000\"/>",
        "<a:schemeClr val=\"accent1\"/>",
    );
    let strip = |s: String| s.replace("<a:solidFill><a:srgbClr val=\"FF0000\"/></a:solidFill>", "");
    let b = source(
        &strip(ph(shape(22, "FF0000"))),
        &strip(ph(shape(23, "FF0000"))),
        &master,
        [None, None],
    );
    let p = plan(&edit(&b, SLIDE, override_slide_map));
    assert_eq!(p["bindings"].as_array().unwrap().len(), 2);
    assert_eq!(p["bindings"][1]["drawingSurface"], SLIDE);
    assert_eq!(
        p["bindings"][1]["fill"]["colors"]["color"]["outcome"]["rgba8"],
        json!([38, 185, 154, 255])
    );
    assert_eq!(
        p["bindings"][1]["fill"]["style"]["fill"]["declaredBy"]["owner"]["part"],
        MASTER
    );
}
#[test]
fn cancellation_after_backend_discards_all_pixels() {
    use std::cell::Cell;
    struct Cancels<'a>(&'a Cell<bool>, &'a Cell<bool>);
    impl RasterBackend for Cancels<'_> {
        fn raster(&mut self, frame: &[u32]) -> Result<BackendReply, RasterError> {
            self.0.set(true);
            Ok(BackendReply {
                status: 0,
                pixels: vec![0; frame[2] as usize * frame[3] as usize * 4],
            })
        }
        fn invalidate(&mut self) {
            self.1.set(true);
        }
    }
    let b = source(&shape(22, "FF0000"), "", "", [None, None]);
    let i = index(&b);
    let flag = Cell::new(false);
    let invalid = Cell::new(false);
    let (response, pixels) = render_pptx_page(
        &request(&i),
        b.as_slice(),
        b.len() as u64,
        SourceLimits::default(),
        &mut Cancels(&flag, &invalid),
        &|| flag.get(),
    );
    assert!(flag.get());
    assert!(invalid.get());
    assert!(pixels.is_empty());
    assert_eq!(
        serde_json::to_value(response).unwrap()["error"]["code"],
        "CANCELLED"
    );
}
#[test]
fn physical_visual_issues_are_preserved_and_unmodeled_content_never_vanishes() {
    let b = source(
        &shape(22, "FF0000").replace("<p:spPr>", "<p:spPr future=\"yes\">"),
        "",
        "",
        [Some(false), None],
    );
    let i = index(&b);
    let issue = &i.surfaces[SLIDE].objects[0].visual_issues[0];
    assert_eq!(issue.local_name, "future");
    assert!(issue.source_ordinal > 0);
    let r = compile_pptx_page(
        &request(&i),
        b.as_slice(),
        b.len() as u64,
        SourceLimits::default(),
        &|| false,
    );
    assert_eq!(
        serde_json::to_value(r).unwrap()["error"]["issue"]["issue"]["sourceOrdinal"],
        issue.source_ordinal
    );
    let b = edit(&b, SLIDE, |s| {
        s.replace("<p:spPr future=\"yes\">", "<p:spPr>unexpected")
    });
    assert_eq!(
        serde_json::from_str::<Value>(&inspect_pptx_json(&b)).unwrap()["error"]["code"],
        "INPUT_INVALID"
    );
}
