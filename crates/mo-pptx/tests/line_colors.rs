mod support;
use mo_opc::{Package, PackageLimits, PartName, RewritePlan};
use mo_pptx::{
    source::{
        color::{ColorContext, ColorLimits, ColorProfile},
        line::{colors::*, resolve::LineProfile},
        *,
    },
    *,
};
use serde_json::{Value, json};
const SLIDE: &str = "/ppt/slides/slide1.xml";
fn package(bytes: &[u8]) -> Package<&[u8]> {
    Package::open(bytes, bytes.len() as u64, PackageLimits::default(), &|| {
        false
    })
    .unwrap()
}
fn fixture(
    fill: &str,
    reference: &str,
    theme_color: Option<&str>,
) -> (SourceIndex, SourceLineColorQuery) {
    let (document, defaults) = support::input();
    let bytes = export(
        &document,
        &defaults,
        &support::resources(),
        PptxLimits::default(),
        &|| false,
    )
    .unwrap();
    let p = package(&bytes);
    let mut plan = RewritePlan::new();
    let part = PartName::new(SLIDE).unwrap();
    let mut xml = String::from_utf8(p.read_part(&part, 1 << 20, &|| false).unwrap()).unwrap();
    let object = xml.find("name=\"title:1\"").unwrap();
    let a = object + xml[object..].find("<p:spPr>").unwrap();
    let b = a + xml[a..].find("</p:spPr>").unwrap() + 9;
    xml.replace_range(a..b, &format!("<p:spPr><a:ln>{fill}</a:ln></p:spPr><p:style><a:lnRef idx=\"1\">{reference}</a:lnRef></p:style>"));
    plan.replace_part(part, xml.into_bytes()).unwrap();
    if let Some(color) = theme_color {
        let part = PartName::new("/ppt/theme/theme2.xml").unwrap();
        let mut xml = String::from_utf8(p.read_part(&part, 1 << 20, &|| false).unwrap()).unwrap();
        let a = xml.find("<a:accent1>").unwrap() + 11;
        let b = a + xml[a..].find("</a:accent1>").unwrap();
        xml.replace_range(a..b, color);
        plan.replace_part(part, xml.into_bytes()).unwrap();
    }
    let bytes = plan.to_bytes(&p, &|| false).unwrap();
    let index = inspect_source(&package(&bytes), SourceLimits::default(), &|| false).unwrap();
    let req = SourceLineColorQuery {
        expected_source_sha256: index.source_sha256.clone(),
        surface: SLIDE.into(),
        objects: vec![
            index.surfaces[SLIDE]
                .objects
                .iter()
                .find(|o| o.name == "title:1")
                .unwrap()
                .native_id,
        ],
        line_profile: LineProfile::Drawingml2024DraftV1,
        color_profile: ColorProfile::Ecma3762016DraftV1,
        context: ColorContext::default(),
    };
    (index, req)
}
fn paint(index: &SourceIndex, req: &SourceLineColorQuery) -> Value {
    serde_json::to_value(
        &query(index, req, LineColorLimits::default(), &|| false)
            .unwrap()
            .objects[0]
            .paint,
    )
    .unwrap()
}
#[test]
fn native_placeholder_composition_never_quantizes_between_transforms() {
    let (index, req) = fixture(
        "<a:solidFill><a:schemeClr val=\"phClr\"><a:redMod val=\"50000\"/></a:schemeClr></a:solidFill>",
        "<a:srgbClr val=\"010000\"><a:redMod val=\"50000\"/></a:srgbClr>",
        None,
    );
    let before = serde_json::to_value(&index).unwrap();
    let p = paint(&index, &req);
    assert_eq!(p["outcome"]["rgba8"], json!([0, 0, 0, 255]));
    assert_eq!(p["outcome"]["rgba16"], json!([64, 0, 0, 65535]));
    assert_eq!(p["outcome"]["srgb"][0], json!(0.25 / 255.0));
    assert_eq!(p["dependencies"], json!([{"kind":"placeholder"}]));
    assert_eq!(serde_json::to_value(index).unwrap(), before);
}
#[test]
fn placeholder_hsl_state_survives_achromatic_reference_boundary() {
    let (index, req) = fixture(
        "<a:solidFill><a:schemeClr val=\"phClr\"><a:sat val=\"100000\"/></a:schemeClr></a:solidFill>",
        "<a:hslClr hue=\"14400000\" sat=\"0\" lum=\"50000\"/>",
        None,
    );
    assert_eq!(
        paint(&index, &req)["outcome"]["rgba8"],
        json!([0, 0, 255, 255])
    );
}
#[test]
fn unused_reference_is_lazy_and_nested_phclr_uses_explicit_host_context() {
    let (index, req) = fixture(
        "<a:solidFill><a:srgbClr val=\"FF2200\"/></a:solidFill>",
        "<a:sysClr val=\"windowText\"/>",
        None,
    );
    let p = paint(&index, &req);
    assert_eq!(p["dependencies"], json!([]));
    assert_eq!(p["outcome"]["rgba8"], json!([255, 34, 0, 255]));
    let (index, mut req) = fixture(
        "<a:solidFill><a:schemeClr val=\"phClr\"><a:alphaMod val=\"50000\"/></a:schemeClr></a:solidFill>",
        "<a:schemeClr val=\"phClr\"><a:alphaMod val=\"50000\"/></a:schemeClr>",
        None,
    );
    assert_eq!(
        paint(&index, &req)["outcome"]["reason"]["kind"],
        "missingPlaceholder"
    );
    req.context.placeholder = Some([255, 0, 0, 128]);
    let p = paint(&index, &req);
    assert_eq!(p["outcome"]["rgba8"], json!([255, 0, 0, 32]));
    assert_eq!(
        p["dependencies"],
        json!([{"kind":"placeholder"},{"kind":"placeholder"}])
    );
}
#[test]
fn theme_cycle_through_style_context_is_detected_without_guessing() {
    let (index, req) = fixture(
        "<a:solidFill><a:schemeClr val=\"accent1\"/></a:solidFill>",
        "<a:schemeClr val=\"accent1\"/>",
        Some("<a:schemeClr val=\"phClr\"/>"),
    );
    let p = paint(&index, &req);
    assert_eq!(
        p["outcome"]["reason"],
        json!({"kind":"schemeCycle","slot":"accent1"})
    );
    assert_eq!(p["dependencies"][0]["kind"], "theme");
    assert_eq!(p["dependencies"][1]["kind"], "placeholder");
}
#[test]
fn source_system_color_and_host_override_keep_dependency_origin() {
    let (index, mut req) = fixture(
        "<a:solidFill><a:schemeClr val=\"phClr\"/></a:solidFill>",
        "<a:sysClr val=\"windowText\" lastClr=\"112233\"/>",
        None,
    );
    let p = paint(&index, &req);
    assert_eq!(p["outcome"]["rgba8"], json!([17, 34, 51, 255]));
    assert_eq!(p["dependencies"][1]["origin"], "fileLastColor");
    req.context
        .system_colors
        .insert(theme::SystemColor::WindowText, [44, 55, 66]);
    let p = paint(&index, &req);
    assert_eq!(p["outcome"]["rgba8"], json!([44, 55, 66, 255]));
    assert_eq!(p["dependencies"][1]["origin"], "hostContext");
}
#[test]
fn unpainted_unresolved_and_out_of_gamut_are_distinct() {
    let (index, req) = fixture("<a:noFill/>", "<a:sysClr val=\"windowText\"/>", None);
    assert_eq!(paint(&index, &req), json!({"kind":"none"}));
    let (index, req) = fixture("<a:gradFill/>", "", None);
    assert_eq!(paint(&index, &req), json!({"kind":"unresolvedStyle"}));
    let (index, req) = fixture(
        "<a:solidFill><a:scrgbClr r=\"200000\" g=\"-50000\" b=\"50000\"/></a:solidFill>",
        "",
        None,
    );
    let p = paint(&index, &req);
    assert_eq!(p["outcome"]["linear"], json!([2.0, -0.5, 0.5, 1.0]));
    assert_eq!(p["outcome"]["clippedForSrgb"], true);
    assert!(p["outcome"]["srgb"][0].as_f64().unwrap() > 1.0);
}
#[test]
fn both_budgets_and_cancel_fail_the_whole_batch() {
    let (index, mut req) = fixture(
        "<a:solidFill><a:schemeClr val=\"phClr\"><a:redMod val=\"50000\"/></a:schemeClr></a:solidFill>",
        "<a:srgbClr val=\"123456\"/>",
        None,
    );
    req.objects = vec![req.objects[0]; 2];
    for colors in [
        ColorLimits {
            max_queries: 1,
            ..Default::default()
        },
        ColorLimits {
            max_steps: 4,
            ..Default::default()
        },
        ColorLimits {
            max_percentage_bytes: 5,
            ..Default::default()
        },
    ] {
        assert!(matches!(
            query(
                &index,
                &req,
                LineColorLimits {
                    colors,
                    ..Default::default()
                },
                &|| false
            ),
            Err(PptxError::Limit(_))
        ));
    }
    assert!(matches!(
        query(
            &index,
            &req,
            LineColorLimits {
                lines: super_line_limits(),
                ..Default::default()
            },
            &|| false
        ),
        Err(PptxError::Limit(_))
    ));
    assert!(matches!(
        query(&index, &req, LineColorLimits::default(), &|| true),
        Err(PptxError::Cancelled)
    ));
    req.expected_source_sha256 = mo_common::Digest::from_sha256([0; 32]);
    assert!(matches!(
        query(&index, &req, LineColorLimits::default(), &|| false),
        Err(PptxError::SourceConflict(_))
    ));
}
fn super_line_limits() -> line::resolve::LineResolveLimits {
    line::resolve::LineResolveLimits {
        max_values: 1,
        ..Default::default()
    }
}
