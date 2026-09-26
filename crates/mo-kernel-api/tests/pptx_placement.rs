use mo_common::Digest;
use mo_kernel_api::*;
use mo_opc::{Package, PackageLimits, PartName, RewritePlan};
use mo_pptx::source::{SourceIndex, inspect_source};
use mo_presentation_compile::source_placement::{SourcePlacementError, source_placements};
use serde_json::{Value, json};
use std::sync::OnceLock;
const SLIDE: &str = "/ppt/slides/slide1.xml";
const LAYOUT: &str = "/ppt/slideLayouts/slideLayout2.xml";
const MASTER: &str = "/ppt/slideMasters/slideMaster2.xml";
const OFF: &str = "<a:off x=\"100\" y=\"200\"/><a:ext cx=\"300\" cy=\"100\"/>";
fn base() -> &'static [u8] {
    static B: OnceLock<Vec<u8>> = OnceLock::new();
    B.get_or_init(|| {
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
fn shape(id: u32, transform: &str, placeholder: bool) -> String {
    let ph = if placeholder {
        "<p:ph type=\"body\" idx=\"1\"/>"
    } else {
        ""
    };
    format!(
        "<p:sp><p:nvSpPr><p:cNvPr id=\"{id}\" name=\"Coordinate test\"/><p:cNvSpPr/><p:nvPr>{ph}</p:nvPr></p:nvSpPr><p:spPr>{transform}<a:prstGeom prst=\"rect\"/></p:spPr></p:sp>"
    )
}
fn group(transform: &str, child: &str) -> String {
    format!(
        "<p:grpSp><p:nvGrpSpPr><p:cNvPr id=\"10\" name=\"Group\"/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr><p:grpSpPr>{transform}</p:grpSpPr>{child}</p:grpSp>"
    )
}
fn source(edits: &[(&str, &str, &str)]) -> Vec<u8> {
    let p = package(base());
    let mut plan = RewritePlan::new();
    for (part, root_transform, objects) in edits {
        let part = PartName::new(*part).unwrap();
        let mut xml = String::from_utf8(p.read_part(&part, 1 << 20, &|| false).unwrap()).unwrap();
        let a = xml.find("<p:spTree>").unwrap();
        let b = xml.find("</p:spTree>").unwrap() + 11;
        xml.replace_range(a..b, &format!("<p:spTree><p:nvGrpSpPr><p:cNvPr id=\"1\" name=\"\"/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr><p:grpSpPr>{root_transform}</p:grpSpPr>{objects}</p:spTree>"));
        plan.replace_part(part, xml.into_bytes()).unwrap();
    }
    plan.to_bytes(&p, &|| false).unwrap()
}
fn index(b: &[u8]) -> SourceIndex {
    inspect_source(&package(b), SourceLimits::default(), &|| false).unwrap()
}
fn request(i: &SourceIndex, objects: &[u32]) -> SourcePlacementQuery {
    serde_json::from_value(json!({"expectedSourceSha256":i.source_sha256,"surface":SLIDE,"objects":objects,"profile":"drawingml-source-sector-scale-q96-v1-draft"})).unwrap()
}
fn result(i: &SourceIndex, ids: &[u32]) -> Value {
    serde_json::to_value(
        source_placements(
            i,
            &request(i, ids),
            SourcePlacementLimits::default(),
            &|| false,
        )
        .unwrap(),
    )
    .unwrap()
}
fn placed(v: &Value, index: usize) -> &Value {
    assert_eq!(v["objects"][index]["outcome"]["status"], "resolved", "{v}");
    &v["objects"][index]["outcome"]["placement"]
}
fn q(v: i64) -> Value {
    json!((i128::from(v) << 32).to_string())
}
#[test]
fn root_is_preserved_without_moving_children_and_request_order_is_exact() {
    let t = format!("<a:xfrm>{OFF}</a:xfrm>");
    let b = source(&[(
        SLIDE,
        "<a:xfrm rot=\"123456\"><a:off x=\"900\" y=\"500\"/></a:xfrm>",
        &shape(11, &t, false),
    )]);
    let i = index(&b);
    let before = i.clone();
    let r = result(&i, &[11, 11]);
    assert_eq!(r["rootTransformIgnored"], true);
    assert_eq!(r["objects"][0], r["objects"][1]);
    assert_eq!(placed(&r, 0)["anchor"], json!({"x":q(150),"y":q(50)}));
    assert_eq!(
        placed(&r, 0)["affine"],
        json!({"linear":[q(1),q(0),q(0),q(1)],"translation":{"x":q(250),"y":q(250)}})
    );
    assert_eq!(i, before);
    assert_eq!(
        i.surfaces[SLIDE]
            .root_group_transform
            .as_ref()
            .unwrap()
            .rotation,
        Some(123456)
    );
}
#[test]
fn child_origin_zero_axes_and_rotation_share_one_center_calculation() {
    let child = shape(
        11,
        "<a:xfrm><a:off x=\"100\" y=\"100\"/><a:ext cx=\"100\" cy=\"50\"/></a:xfrm>",
        false,
    );
    for (cx, cy, rot, sx, sy, x, y) in [
        (0, 0, 0, 1, 1, 250, 175),
        (0, 100, 0, 1, 2, 250, 250),
        (0, 0, 5_400_000, 1, 1, 425, 50),
        (200, 100, 0, 2, 2, 300, 250),
    ] {
        let t = format!(
            "<a:xfrm rot=\"{rot}\"><a:off x=\"200\" y=\"100\"/><a:ext cx=\"400\" cy=\"200\"/><a:chOff x=\"100\" y=\"50\"/><a:chExt cx=\"{cx}\" cy=\"{cy}\"/></a:xfrm>"
        );
        let i = index(&source(&[(SLIDE, "", &group(&t, &child))]));
        let r = result(&i, &[11]);
        let p = placed(&r, 0);
        assert_eq!(r["objects"][0]["depth"], 1);
        assert_eq!(p["affine"]["translation"], json!({"x":q(x),"y":q(y)}));
        assert_eq!(
            p["affine"]["linear"],
            if rot == 0 {
                json!([q(sx), q(0), q(0), q(sy)])
            } else {
                json!([q(0), q(-sy), q(sx), q(0)])
            }
        );
    }
}
#[test]
fn nearest_transform_stops_orientation_inheritance_but_coordinates_remain_per_field() {
    let master = shape(
        11,
        &format!("<a:xfrm rot=\"5400000\" flipH=\"1\">{OFF}</a:xfrm>"),
        true,
    );
    let layout = shape(11, "", true);
    for (local, rot, owner) in [
        ("", 5_400_000, MASTER),
        ("<a:xfrm/>", 0, SLIDE),
        ("<a:xfrm><a:ext cx=\"400\" cy=\"200\"/></a:xfrm>", 0, SLIDE),
    ] {
        let i = index(&source(&[
            (MASTER, "", &master),
            (LAYOUT, "", &layout),
            (SLIDE, "", &shape(11, local, true)),
        ]));
        let r = result(&i, &[11]);
        let t = &placed(&r, 0)["transform"];
        assert_eq!(t["rotation"]["value"], rot);
        assert_eq!(t["rotation"]["source"]["object"]["part"], owner);
        assert_eq!(t["origin"]["source"]["object"]["part"], MASTER);
        assert_eq!(
            t["size"]["source"]["object"]["part"],
            if local.contains("ext") { SLIDE } else { MASTER }
        );
    }
}
#[test]
fn graphic_frame_orientation_is_retained_but_not_applied() {
    let frame = format!(
        "<p:graphicFrame><p:nvGraphicFramePr><p:cNvPr id=\"11\" name=\"Frame\"/><p:cNvGraphicFramePr/><p:nvPr/></p:nvGraphicFramePr><p:xfrm rot=\"5400000\" flipH=\"1\">{OFF}</p:xfrm><a:graphic><a:graphicData uri=\"urn:test\"/></a:graphic></p:graphicFrame>"
    );
    let i = index(&source(&[(SLIDE, "", &frame)]));
    let r = result(&i, &[11]);
    let p = placed(&r, 0);
    assert_eq!(p["transform"]["graphicFrameOrientationIgnored"], true);
    assert_eq!(p["transform"]["rotation"]["value"], 5_400_000);
    assert_eq!(p["affine"]["linear"], json!([q(1), q(0), q(0), q(1)]));
}
#[test]
fn unknown_transform_semantics_block_descendants_with_precise_owner() {
    for opaque in [
        "<a:unknown><a:off x=\"999\" y=\"999\"/></a:unknown>",
        "<foreign xmlns=\"urn:future\">text</foreign>",
    ] {
        let t = format!("<a:xfrm>{OFF}{opaque}</a:xfrm>");
        let child = shape(11, &format!("<a:xfrm>{OFF}</a:xfrm>"), false);
        let i = index(&source(&[(SLIDE, "", &group(&t, &child))]));
        let r = result(&i, &[10, 11]);
        assert_eq!(r["objects"][0]["outcome"], r["objects"][1]["outcome"]);
        assert_eq!(
            r["objects"][1]["outcome"]["reason"]["object"]["nativeId"],
            10
        );
        assert_eq!(
            r["objects"][1]["outcome"]["reason"]["cause"]["kind"],
            "retainedTransform"
        );
        assert!(
            i.surfaces[SLIDE].objects[0]
                .transform
                .as_ref()
                .unwrap()
                .retained_ordinals[0]
                > 0
        );
    }
}
#[test]
fn malformed_known_transforms_are_rejected_without_partial_source_results() {
    for t in [
        "<a:xfrm><a:ext cx=\"-1\" cy=\"1\"/></a:xfrm>",
        "<a:xfrm><a:off x=\"1\" y=\"1\"/><a:off x=\"2\" y=\"2\"/></a:xfrm>",
        "<a:xfrm><a:ext cx=\"1\" cy=\"1\"/><a:off x=\"1\" y=\"1\"/></a:xfrm>",
        "<a:xfrm><a:chExt cx=\"1\" cy=\"1\"/></a:xfrm>",
        "<a:xfrm>bad</a:xfrm>",
        "<a:xfrm/><a:xfrm/>",
        "<a:xfrm><a:off x=\"1\"/></a:xfrm>",
    ] {
        let b = source(&[(SLIDE, "", &shape(11, t, false))]);
        assert!(
            inspect_source(&package(&b), SourceLimits::default(), &|| false).is_err(),
            "{t}"
        );
    }
}
#[test]
fn cancellation_budgets_binding_and_invalid_parent_graph_are_explicit() {
    let child = shape(11, &format!("<a:xfrm>{OFF}</a:xfrm>"), false);
    let b = source(&[(SLIDE, "", &group("<a:xfrm/>", &child))]);
    let mut i = index(&b);
    let q = request(&i, &[11]);
    for limits in [
        SourcePlacementLimits {
            max_queries: 0,
            ..Default::default()
        },
        SourcePlacementLimits {
            max_steps: 3,
            ..Default::default()
        },
        SourcePlacementLimits {
            max_indexed_objects: 1,
            ..Default::default()
        },
        SourcePlacementLimits {
            max_depth: 0,
            ..Default::default()
        },
    ] {
        assert!(matches!(
            source_placements(&i, &q, limits, &|| false),
            Err(SourcePlacementError::Limit(_))
        ));
    }
    let n = std::cell::Cell::new(0);
    assert!(matches!(
        source_placements(&i, &q, SourcePlacementLimits::default(), &|| {
            n.set(n.get() + 1);
            n.get() > 4
        }),
        Err(SourcePlacementError::Cancelled)
    ));
    let mut bad = q.clone();
    bad.expected_source_sha256 = Digest::from_sha256([0; 32]);
    assert!(matches!(
        source_placements(&i, &bad, SourcePlacementLimits::default(), &|| false),
        Err(SourcePlacementError::SourceConflict)
    ));
    bad = q.clone();
    bad.objects = vec![999];
    assert!(source_placements(&i, &bad, SourcePlacementLimits::default(), &|| false).is_err());
    i.surfaces.get_mut(SLIDE).unwrap().objects[0].parent_group = Some(10);
    assert!(matches!(
        source_placements(&i, &q, SourcePlacementLimits::default(), &|| false),
        Err(SourcePlacementError::Invalid(_))
    ));
}
#[test]
fn bridge_preserves_cancel_limits_and_conflicts() {
    let b = source(&[(
        SLIDE,
        "",
        &shape(11, &format!("<a:xfrm>{OFF}</a:xfrm>"), false),
    )]);
    let i = index(&b);
    let mut q = request(&i, &[11]);
    for (cancel, limits, code) in [
        (true, SourcePlacementLimits::default(), "CANCELLED"),
        (
            false,
            SourcePlacementLimits {
                max_steps: 1,
                ..Default::default()
            },
            "LIMIT_EXCEEDED",
        ),
    ] {
        let r = place_pptx_objects(
            &q,
            b.as_slice(),
            b.len() as u64,
            SourceLimits::default(),
            limits,
            &|| cancel,
        );
        assert_eq!(serde_json::to_value(r).unwrap()["error"]["code"], code);
    }
    q.expected_source_sha256 = Digest::from_sha256([0; 32]);
    let r: Value = serde_json::from_str(&place_pptx_objects_json(
        &serde_json::to_string(&q).unwrap(),
        &b,
    ))
    .unwrap();
    assert_eq!(r["error"]["code"], "SOURCE_CONFLICT");
    assert!(matches!(
        serde_json::from_str::<PptxPlacementResponse>(&place_pptx_objects_json("{}", &b)).unwrap(),
        PptxPlacementResponse::Error { .. }
    ));
}
