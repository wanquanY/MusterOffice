mod support;
use mo_opc::{Package, PackageLimits, PartName, RewritePlan};
use mo_pptx::{
    source::{
        color::{ColorContext, ColorLimits, ColorProfile},
        fill::{
            colors::*,
            resolve::{FillProfile, FillResolveLimits, FillTarget},
        },
        *,
    },
    *,
};
use serde_json::{Value, json};
const SLIDE: &str = "/ppt/slides/slide1.xml";
const LAYOUT: &str = "/ppt/slideLayouts/slideLayout2.xml";
fn package(bytes: &[u8]) -> Package<&[u8]> {
    Package::open(bytes, bytes.len() as u64, PackageLimits::default(), &|| {
        false
    })
    .unwrap()
}
fn rewrite(bytes: &[u8], part: &str, change: impl FnOnce(String) -> String) -> Vec<u8> {
    let p = package(bytes);
    let part = PartName::new(part).unwrap();
    let xml = String::from_utf8(p.read_part(&part, 1 << 20, &|| false).unwrap()).unwrap();
    let mut plan = RewritePlan::new();
    plan.replace_part(part, change(xml).into_bytes()).unwrap();
    plan.to_bytes(&p, &|| false).unwrap()
}
fn shape(mut xml: String, fill: &str, style: &str, placeholder: bool) -> String {
    let first = xml.find("<p:sp>").unwrap();
    let a = first + xml[first..].find("<p:spPr>").unwrap();
    let b = a + xml[a..].find("</p:spPr>").unwrap() + 9;
    xml.replace_range(a..b, &format!("<p:spPr>{fill}</p:spPr>{style}"));
    if placeholder {
        let a = first + xml[first..].find("<p:nvPr/>").unwrap();
        xml.replace_range(a..a + 9, "<p:nvPr><p:ph type=\"body\" idx=\"7\"/></p:nvPr>");
    }
    xml
}
fn bytes(fill: &str, style: &str) -> Vec<u8> {
    let (d, defaults) = support::input();
    let b = export(
        &d,
        &defaults,
        &support::resources(),
        PptxLimits::default(),
        &|| false,
    )
    .unwrap();
    rewrite(&b, SLIDE, |x| shape(x, fill, style, false))
}
fn setup(b: &[u8]) -> (SourceIndex, SourceFillColorQuery) {
    let i = inspect_source(&package(b), SourceLimits::default(), &|| false).unwrap();
    let q = SourceFillColorQuery {
        expected_source_sha256: i.source_sha256.clone(),
        surface: SLIDE.into(),
        targets: vec![FillTarget::Object {
            native_id: i.surfaces[SLIDE].objects[0].native_id,
        }],
        fill_profile: FillProfile::Drawingml2024DraftV1,
        color_profile: ColorProfile::Ecma3762016DraftV1,
        context: ColorContext::default(),
    };
    (i, q)
}
fn style(color: &str) -> String {
    format!("<p:style><a:fillRef idx=\"1\">{color}</a:fillRef></p:style>")
}
fn fixture(fill: &str, color: &str) -> (SourceIndex, SourceFillColorQuery) {
    setup(&bytes(fill, &style(color)))
}
fn colors(i: &SourceIndex, q: &SourceFillColorQuery) -> Value {
    serde_json::to_value(
        &query(i, q, FillColorLimits::default(), &|| false)
            .unwrap()
            .targets[0]
            .colors,
    )
    .unwrap()
}
fn ph() -> &'static str {
    "<a:schemeClr val=\"phClr\"/>"
}

#[test]
fn native_fill_reference_composes_at_working_precision_and_preserves_hsl_state() {
    let (i, q) = fixture(
        "<a:solidFill><a:schemeClr val=\"phClr\"><a:redMod val=\"50000\"/></a:schemeClr></a:solidFill>",
        "<a:srgbClr val=\"010000\"><a:redMod val=\"50000\"/></a:srgbClr>",
    );
    let before = serde_json::to_value(&i).unwrap();
    let c = colors(&i, &q);
    assert_eq!(c["color"]["outcome"]["rgba8"], json!([0, 0, 0, 255]));
    assert_eq!(c["color"]["outcome"]["rgba16"], json!([64, 0, 0, 65535]));
    assert_eq!(c["color"]["outcome"]["srgb"][0], 0.25 / 255.0);
    let reference = i.surfaces[SLIDE].objects[0]
        .fill_reference
        .as_ref()
        .unwrap();
    assert_eq!(
        c["color"]["placeholder"]["referenceOrdinal"],
        reference.source_ordinal
    );
    assert_eq!(
        c["color"]["placeholder"]["colorOrdinal"],
        reference.color.as_ref().unwrap().source_ordinal
    );
    assert_eq!(before, serde_json::to_value(&i).unwrap());
    let (i, q) = fixture(
        "<a:solidFill><a:schemeClr val=\"phClr\"><a:sat val=\"100000\"/></a:schemeClr></a:solidFill>",
        "<a:hslClr hue=\"14400000\" sat=\"0\" lum=\"50000\"/>",
    );
    assert_eq!(
        colors(&i, &q)["color"]["outcome"]["rgba8"],
        json!([0, 0, 255, 255])
    );
}
#[test]
fn unused_retained_reference_does_not_block_rgb_but_used_context_reports_native_node() {
    let style = style("<a:sysClr val=\"windowText\"/>")
        .replace("idx=\"1\"", "idx=\"4294967295\" future=\"x\"");
    let (i, q) = setup(&bytes(
        "<a:solidFill><a:srgbClr val=\"12AB34\"/></a:solidFill>",
        &style,
    ));
    let c = colors(&i, &q);
    assert_eq!(c["color"]["outcome"]["rgba8"], json!([18, 171, 52, 255]));
    assert_eq!(c["color"]["placeholder"], Value::Null);
    let (i, mut q) = setup(&bytes(
        &format!("<a:solidFill>{}</a:solidFill>", ph()),
        &style,
    ));
    q.context.placeholder = Some([0, 255, 0, 255]);
    let c = colors(&i, &q);
    assert_eq!(
        c["color"]["outcome"]["reason"]["kind"],
        "retainedPlaceholderContext"
    );
    assert_eq!(
        c["color"]["outcome"]["reason"]["sourceOrdinal"],
        i.surfaces[SLIDE].objects[0]
            .fill_reference
            .as_ref()
            .unwrap()
            .source_ordinal
    );
}
#[test]
fn theme_indirection_reaches_lazy_context_and_cycles_span_that_context() {
    let b = bytes(
        "<a:solidFill><a:schemeClr val=\"accent1\"/></a:solidFill>",
        &style("<a:srgbClr val=\"334455\"/>"),
    );
    let b = rewrite(&b, "/ppt/theme/theme2.xml", |mut x| {
        let a = x.find("<a:accent1>").unwrap() + 11;
        let b = a + x[a..].find("</a:accent1>").unwrap();
        x.replace_range(a..b, ph());
        x
    });
    let (i, q) = setup(&b);
    let c = colors(&i, &q);
    assert_eq!(c["color"]["outcome"]["rgba8"], json!([51, 68, 85, 255]));
    assert_eq!(c["color"]["dependencies"][0]["kind"], "theme");
    assert_eq!(c["color"]["dependencies"][1]["kind"], "placeholder");
    let b = rewrite(&b, SLIDE, |x| {
        x.replace(
            "<a:srgbClr val=\"334455\"/>",
            "<a:schemeClr val=\"accent1\"/>",
        )
    });
    let (i, q) = setup(&b);
    assert_eq!(
        colors(&i, &q)["color"]["outcome"]["reason"],
        json!({"kind":"schemeCycle","slot":"accent1"})
    );
}
#[test]
fn nested_placeholder_uses_explicit_host_and_missing_native_color_is_distinct() {
    let (i, mut q) = fixture(
        &format!("<a:solidFill>{}</a:solidFill>", ph()),
        "<a:schemeClr val=\"phClr\"><a:alphaMod val=\"50000\"/></a:schemeClr>",
    );
    assert_eq!(
        colors(&i, &q)["color"]["outcome"]["reason"]["kind"],
        "missingPlaceholder"
    );
    q.context.placeholder = Some([0, 255, 0, 128]);
    let c = colors(&i, &q);
    assert_eq!(c["color"]["outcome"]["rgba8"], json!([0, 255, 0, 64]));
    assert_eq!(c["color"]["dependencies"].as_array().unwrap().len(), 2);
    let (i, mut q) = fixture(&format!("<a:solidFill>{}</a:solidFill>", ph()), "");
    q.context.placeholder = Some([1, 2, 3, 4]);
    let c = colors(&i, &q);
    assert!(c["color"]["placeholder"].is_object());
    assert_eq!(c["color"]["placeholder"]["colorOrdinal"], Value::Null);
    assert_eq!(c["color"]["outcome"]["rgba8"], json!([1, 2, 3, 4]));
}
#[test]
fn gradient_preserves_stop_order_duplicates_and_individual_color_failures() {
    let fill = format!(
        "<a:gradFill><a:gsLst><a:gs pos=\"50000\"><a:srgbClr val=\"FF0000\"/></a:gs><a:gs pos=\"0\">{}</a:gs><a:gs pos=\"50000\"><a:srgbClr val=\"0000FF\"/></a:gs></a:gsLst></a:gradFill>",
        ph()
    );
    let (i, q) = setup(&bytes(&fill, ""));
    let out = query(&i, &q, FillColorLimits::default(), &|| false).unwrap();
    let v = serde_json::to_value(out).unwrap();
    let r = &v["targets"][0];
    assert_eq!(
        r["colors"]["stops"][0]["outcome"]["rgba8"],
        json!([255, 0, 0, 255])
    );
    assert_eq!(
        r["colors"]["stops"][1]["outcome"]["reason"]["kind"],
        "missingPlaceholder"
    );
    assert_eq!(
        r["colors"]["stops"][2]["outcome"]["rgba8"],
        json!([0, 0, 255, 255])
    );
    let stops = r["style"]["fill"]["gradient"]["stops"]["value"]
        .as_array()
        .unwrap();
    assert_eq!(
        stops
            .iter()
            .map(|s| s["position"]["value"].as_str().unwrap())
            .collect::<Vec<_>>(),
        ["50000", "0", "50000"]
    );
}
#[test]
fn inherited_pattern_colors_keep_distinct_native_context_owners() {
    let b = bytes("", "");
    let b = rewrite(&b, SLIDE, |x| {
        shape(
            x,
            &format!(
                "<a:pattFill prst=\"cross\"><a:fgClr>{}</a:fgClr></a:pattFill>",
                ph()
            ),
            &style("<a:srgbClr val=\"FF0000\"/>"),
            true,
        )
    });
    let b = rewrite(&b, LAYOUT, |x| {
        shape(
            x,
            &format!("<a:pattFill><a:bgClr>{}</a:bgClr></a:pattFill>", ph()),
            &style("<a:srgbClr val=\"0000FF\"/>"),
            true,
        )
    });
    let (i, q) = setup(&b);
    let c = colors(&i, &q);
    assert_eq!(c["foreground"]["outcome"]["rgba8"], json!([255, 0, 0, 255]));
    assert_eq!(c["background"]["outcome"]["rgba8"], json!([0, 0, 255, 255]));
    assert_eq!(c["foreground"]["placeholder"]["owner"]["part"], SLIDE);
    assert_eq!(c["background"]["placeholder"]["owner"]["part"], LAYOUT);
}
#[test]
fn line_fill_uses_line_reference_and_background_uses_its_background_reference() {
    let b = bytes(
        &format!("<a:ln><a:solidFill>{}</a:solidFill></a:ln>", ph()),
        "<p:style><a:lnRef idx=\"1\"><a:srgbClr val=\"0000FF\"/></a:lnRef><a:fillRef idx=\"1\"><a:srgbClr val=\"FF0000\"/></a:fillRef></p:style>",
    );
    let (i, mut q) = setup(&b);
    q.targets = vec![FillTarget::Line {
        native_id: i.surfaces[SLIDE].objects[0].native_id,
    }];
    assert_eq!(
        colors(&i, &q)["color"]["outcome"]["rgba8"],
        json!([0, 0, 255, 255])
    );
    let b = rewrite(&b, SLIDE, |x| {
        x.replacen(
            "<p:spTree>",
            "<p:bg><p:bgRef idx=\"1001\"><a:srgbClr val=\"123456\"/></p:bgRef></p:bg><p:spTree>",
            1,
        )
    });
    let b = rewrite(&b, "/ppt/theme/theme2.xml", |mut x| {
        let a = x.find("<a:bgFillStyleLst>").unwrap() + 18;
        let b = a + x[a..].find("</a:bgFillStyleLst>").unwrap();
        x.replace_range(
            a..b,
            &format!("<a:solidFill>{}</a:solidFill><a:noFill/><a:noFill/>", ph()),
        );
        x
    });
    let (i, mut q) = setup(&b);
    q.targets = vec![FillTarget::Background {}];
    let c = colors(&i, &q);
    assert_eq!(c["color"]["outcome"]["rgba8"], json!([18, 52, 86, 255]));
    assert_eq!(
        c["color"]["placeholder"]["owner"]["target"]["kind"],
        "background"
    );
}
#[test]
fn image_resources_no_fill_and_unresolved_styles_are_not_fake_transparent_colors() {
    for (fill, kind) in [
        ("<a:noFill/>", "none"),
        (
            "<a:blipFill><a:blip r:embed=\"owned\"/></a:blipFill>",
            "imageResourcesRequired",
        ),
        ("<a:blipFill/>", "unresolvedStyle"),
    ] {
        let (i, q) = setup(&bytes(fill, ""));
        let mut limits = FillColorLimits::default();
        limits.colors.max_queries = 0;
        let v = serde_json::to_value(query(&i, &q, limits, &|| false).unwrap()).unwrap();
        assert_eq!(v["targets"][0]["colors"]["kind"], kind);
    }
}
#[test]
fn system_fallback_host_override_and_out_of_gamut_working_channels_survive() {
    let (i, mut q) = fixture(
        &format!("<a:solidFill>{}</a:solidFill>", ph()),
        "<a:sysClr val=\"windowText\" lastClr=\"112233\"/>",
    );
    assert_eq!(
        colors(&i, &q)["color"]["outcome"]["rgba8"],
        json!([17, 34, 51, 255])
    );
    q.context
        .system_colors
        .insert(drawingml::SystemColor::WindowText, [3, 4, 5]);
    let c = colors(&i, &q);
    assert_eq!(c["color"]["dependencies"][1]["origin"], "hostContext");
    let (i, q) = fixture(
        &format!("<a:solidFill>{}</a:solidFill>", ph()),
        "<a:scrgbClr r=\"200000\" g=\"-50000\" b=\"50000\"/>",
    );
    let c = colors(&i, &q);
    assert_eq!(
        c["color"]["outcome"]["linear"],
        json!([2.0, -0.5, 0.5, 1.0])
    );
    assert_eq!(c["color"]["outcome"]["clippedForSrgb"], true);
}
#[test]
fn all_selected_color_slots_and_lookup_work_share_batch_limits() {
    let (i, mut q) = fixture(
        "<a:solidFill><a:schemeClr val=\"phClr\"><a:redMod val=\"50000\"/></a:schemeClr></a:solidFill>",
        "<a:srgbClr val=\"123456\"/>",
    );
    q.targets.push(q.targets[0].clone());
    for colors in [
        ColorLimits {
            max_queries: 1,
            ..Default::default()
        },
        ColorLimits {
            max_steps: 2,
            ..Default::default()
        },
        ColorLimits {
            max_percentage_bytes: 4,
            ..Default::default()
        },
    ] {
        assert!(matches!(
            query(
                &i,
                &q,
                FillColorLimits {
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
            &i,
            &q,
            FillColorLimits {
                fills: FillResolveLimits {
                    max_values: 1,
                    ..Default::default()
                },
                ..Default::default()
            },
            &|| false
        ),
        Err(PptxError::Limit(_))
    ));
    assert!(matches!(
        query(&i, &q, FillColorLimits::default(), &|| true),
        Err(PptxError::Cancelled)
    ));
    q.expected_source_sha256 = mo_common::Digest::from_sha256([0; 32]);
    assert!(matches!(
        query(&i, &q, FillColorLimits::default(), &|| false),
        Err(PptxError::SourceConflict(_))
    ));
}

#[test]
fn many_stops_share_source_object_lookup_without_repeated_full_scans() {
    let stops = format!("<a:gs pos=\"50000\">{}</a:gs>", ph()).repeat(100);
    let (i, q) = fixture(
        &format!("<a:gradFill><a:gsLst>{stops}</a:gsLst></a:gradFill>"),
        "<a:srgbClr val=\"123456\"/>",
    );
    let mut limits = FillColorLimits::default();
    limits.colors.max_steps = 450;
    let result = query(&i, &q, limits, &|| false).unwrap();
    let FillPaintColors::Gradient { stops } = &result.targets[0].colors else {
        panic!()
    };
    assert_eq!(stops.len(), 100);
    assert!(
        stops
            .iter()
            .all(|c| matches!(c.outcome, color::ColorSample::Resolved { .. }))
    );
}
