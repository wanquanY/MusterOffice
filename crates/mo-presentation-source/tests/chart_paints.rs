#[path = "support/chart_paints.rs"]
mod support;
use mo_opc::{Package, PackageLimits};
use mo_presentation_source::{
    PptxError,
    source::{
        SourceLimits, SourceObjectRef,
        charts::{ChartMarkupKind, paints::*},
        color::*,
        inspect_source,
    },
};
use serde_json::json;
use std::cell::Cell;
use support::*;
fn inspect(
    bytes: &[u8],
    limits: ChartPaintLimits,
    source_limits: SourceLimits,
    check: &dyn Fn() -> bool,
) -> Result<SourceChartPaints, PptxError> {
    let p = Package::open(bytes, bytes.len() as u64, PackageLimits::default(), &|| {
        false
    })?;
    let index = inspect_source(&p, Default::default(), &|| false)?;
    mo_presentation_source::source::charts::paints::query(
        &p,
        &index,
        &SourceChartPaintQuery {
            expected_source_sha256: p.sha256().clone(),
            object: SourceObjectRef {
                part: "/ppt/slides/slide1.xml".into(),
                native_id: 2,
            },
            profile: ColorProfile::Ecma3762016DraftV1,
            context: ColorContext::default(),
        },
        source_limits,
        limits,
        check,
    )
}
fn paints(bytes: &[u8]) -> SourceChartPaints {
    inspect(bytes, Default::default(), Default::default(), &|| false).unwrap()
}
fn rgba(p: &SourceChartPaints, decl: usize, color: usize) -> [u8; 4] {
    match p.declarations[decl].colors[color].outcome {
        ColorSample::Resolved { rgba8, .. } => rgba8,
        _ => panic!("unresolved"),
    }
}
const EXPLICIT: &str = r#"<c:spPr><a:solidFill><a:srgbClr val="D02010"><a:alpha val="50000"/></a:srgbClr></a:solidFill><a:ln w="9525"><a:solidFill><a:srgbClr val="EEEEEE"/></a:solidFill></a:ln></c:spPr><c:dPt><c:idx val="0"/><c:spPr><a:noFill/><a:ln><a:noFill/></a:ln></c:spPr></c:dPt>"#;
#[test]
fn declared_colors_lines_and_point_bindings_reuse_native_readers_without_defaults() {
    let bytes = fixture(EXPLICIT, None, None, None);
    let before = bytes.clone();
    let p = paints(&bytes);
    assert_eq!(p.declarations.len(), 2);
    assert_eq!(rgba(&p, 0, 0), [208, 32, 16, 128]);
    assert_eq!(rgba(&p, 0, 1), [238, 238, 238, 255]);
    assert!(p.declarations[1].colors.is_empty());
    assert_eq!(
        p.declarations[0]
            .line
            .as_ref()
            .unwrap()
            .width
            .unwrap()
            .get(),
        9525
    );
    let s = &p.chart.plots[0].series[0];
    assert_eq!(
        s.layout
            .markup
            .iter()
            .find(|m| m.kind == ChartMarkupKind::ShapeProperties)
            .unwrap()
            .source_ordinal,
        p.declarations[0].source_ordinal
    );
    assert_eq!(
        s.point_overrides[0].layout.markup[0].source_ordinal,
        p.declarations[1].source_ordinal
    );
    assert_eq!(bytes, before);
}

#[test]
fn mce_fallback_and_opaque_paint_subtrees_keep_physical_ordinals() {
    let style = r#"<c:spPr xmlns:mc="http://schemas.openxmlformats.org/markup-compatibility/2006" xmlns:f="urn:owned-future"><mc:AlternateContent><mc:Choice Requires="f"><a:solidFill><a:srgbClr val="FF0000"/></a:solidFill></mc:Choice><mc:Fallback><a:solidFill><a:srgbClr val="0000FF"/></a:solidFill></mc:Fallback></mc:AlternateContent><a:extLst><a:ext uri="owned"><c:spPr><a:solidFill><a:srgbClr val="00FF00"/></a:solidFill></c:spPr></a:ext></a:extLst></c:spPr>"#;
    let p = paints(&fixture(style, None, None, None));
    assert_eq!(p.declarations.len(), 1);
    assert_eq!(p.declarations[0].colors.len(), 1);
    assert_eq!(rgba(&p, 0, 0), [0, 0, 255, 255]);
    assert!(p.declarations[0].colors[0].source_ordinal > p.declarations[0].source_ordinal + 5);
    assert_eq!(p.declarations[0].retained_ordinals.len(), 1);
}

#[test]
fn chart_reread_and_owned_theme_share_total_input_byte_budget() {
    let bytes = fixture(
        EXPLICIT,
        None,
        None,
        Some(&theme(r#"<a:srgbClr val="804020"/>"#)),
    );
    let p = Package::open(
        bytes.as_slice(),
        bytes.len() as u64,
        PackageLimits::default(),
        &|| false,
    )
    .unwrap();
    let len = |s: &str| p.parts()[&mo_opc::PartName::new(s).unwrap()].byte_length as usize;
    let total = len("/ppt/slides/slide1.xml")
        + 2 * len("/ppt/charts/chart1.xml")
        + len("/ppt/theme/chart-theme.xml");
    let mut limits = ChartPaintLimits::default();
    limits.source.max_total_part_bytes = total;
    inspect(&bytes, limits, Default::default(), &|| false).unwrap();
    limits.source.max_total_part_bytes = total - 1;
    assert!(matches!(
        inspect(&bytes, limits, Default::default(), &|| false),
        Err(PptxError::Limit("chart paint total part bytes"))
    ));
}
#[test]
fn chart_mapping_and_theme_overlay_replace_only_the_context_they_declare() {
    let style = r#"<c:spPr><a:solidFill><a:schemeClr val="accent1"><a:redMod val="50000"/></a:schemeClr></a:solidFill></c:spPr>"#;
    let slide = theme(r#"<a:srgbClr val="800000"/>"#);
    let chart = theme(r#"<a:srgbClr val="400000"/>"#);
    let parent = paints(&fixture(style, None, Some(&slide), None));
    assert_eq!(rgba(&parent, 0, 0), [64, 0, 0, 255]);
    let overlay = paints(&fixture(style, None, Some(&slide), Some(&chart)));
    assert_eq!(rgba(&overlay, 0, 0), [32, 0, 0, 255]);
    assert_eq!(
        overlay.color_scheme.unwrap().part,
        "/ppt/theme/chart-theme.xml"
    );
    let map = format!(
        "<c:clrMapOvr {}/>",
        MAP.replace("accent1=\"accent1\"", "accent1=\"accent2\"")
    );
    let mapped = paints(&fixture(style, Some(&map), Some(&slide), Some(&chart)));
    assert_eq!(rgba(&mapped, 0, 0), [16, 64, 128, 255]);
    assert_eq!(mapped.color_mapping.unwrap().part, "/ppt/charts/chart1.xml");
    let empty = format!(r#"<a:themeOverride xmlns:a="{}"/>"#, support::charts::A);
    let inherited = paints(&fixture(style, None, Some(&slide), Some(&empty)));
    assert_eq!(rgba(&inherited, 0, 0), [64, 0, 0, 255]);
    assert_eq!(
        inherited.color_scheme.unwrap().part,
        "/ppt/theme/slide-theme.xml"
    );
}
#[test]
fn working_color_precision_survives_theme_indirection_and_declared_transforms() {
    let style = r#"<c:spPr><a:solidFill><a:schemeClr val="accent1"><a:redMod val="50000"/></a:schemeClr></a:solidFill></c:spPr>"#;
    let theme = theme(r#"<a:srgbClr val="010000"><a:redMod val="50000"/></a:srgbClr>"#);
    let p = paints(&fixture(style, None, None, Some(&theme)));
    let c = serde_json::to_value(&p.declarations[0].colors[0]).unwrap();
    assert_eq!(c["outcome"]["rgba8"], json!([0, 0, 0, 255]));
    assert_eq!(c["outcome"]["rgba16"], json!([64, 0, 0, 65535]));
    assert_eq!(c["outcome"]["srgb"][0], json!(0.25 / 255.0));
}
#[test]
fn gradients_patterns_and_extensions_preserve_lexical_order_and_unknown_semantics() {
    let styles = r#"<c:spPr bwMode="gray"><a:gradFill><a:gsLst><a:gs pos="50000"><a:srgbClr val="FF0000"/></a:gs><a:gs pos="50000"><a:srgbClr val="0000FF"/></a:gs></a:gsLst><a:lin ang="0" scaled="0"/></a:gradFill><a:ln><a:pattFill prst="pct5"><a:fgClr><a:srgbClr val="112233"/></a:fgClr><a:bgClr><a:srgbClr val="445566"/></a:bgClr></a:pattFill></a:ln><a:effectLst/><a:extLst><a:ext uri="owned"><c:spPr><a:noFill/></c:spPr></a:ext></a:extLst></c:spPr>"#;
    let p = paints(&fixture(styles, None, None, None));
    assert_eq!(p.declarations.len(), 1);
    assert_eq!(p.declarations[0].colors.len(), 4);
    assert_eq!(rgba(&p, 0, 0), [255, 0, 0, 255]);
    assert_eq!(rgba(&p, 0, 1), [0, 0, 255, 255]);
    assert_eq!(p.declarations[0].retained_ordinals.len(), 1);
    assert!(
        p.declarations[0]
            .effects
            .as_ref()
            .unwrap()
            .is_explicitly_empty_list()
    );
}
#[test]
fn duplicate_paints_and_opaque_color_context_never_become_a_success() {
    for styles in [
        "<c:spPr><a:noFill/><a:solidFill/></c:spPr>",
        "<c:spPr><a:ln/><a:ln/></c:spPr>",
        "<c:spPr><a:solidFill>bad</a:solidFill></c:spPr>",
    ] {
        assert!(
            inspect(
                &fixture(styles, None, None, None),
                Default::default(),
                Default::default(),
                &|| false
            )
            .is_err()
        );
    }
    for map in [
        format!("<c:clrMapOvr {MAP}><a:extLst/></c:clrMapOvr>"),
        format!("<c:clrMapOvr {MAP} unknown=\"1\"/>"),
        "<c:clrMapOvr/>".into(),
    ] {
        assert!(
            inspect(
                &fixture(EXPLICIT, Some(&map), None, None),
                Default::default(),
                Default::default(),
                &|| false
            )
            .is_err()
        );
    }
}
#[test]
fn color_dependencies_remain_unresolved_without_the_authored_theme_or_host_system_color() {
    for color in [
        "<a:schemeClr val=\"accent1\"/>",
        "<a:sysClr val=\"window\"/>",
        "<a:schemeClr val=\"phClr\"/>",
    ] {
        let p = paints(&fixture(
            &format!("<c:spPr><a:solidFill>{color}</a:solidFill></c:spPr>"),
            None,
            None,
            None,
        ));
        assert!(matches!(
            p.declarations[0].colors[0].outcome,
            ColorSample::Unresolved { .. }
        ));
    }
}
#[test]
fn paint_counts_numeric_work_and_all_selected_cancellations_are_bounded() {
    let bytes = fixture(EXPLICIT, None, None, None);
    for limits in [
        ChartPaintLimits {
            max_declarations: 1,
            ..Default::default()
        },
        ChartPaintLimits {
            colors: ColorLimits {
                max_queries: 1,
                ..Default::default()
            },
            ..Default::default()
        },
        ChartPaintLimits {
            colors: ColorLimits {
                max_steps: 0,
                ..Default::default()
            },
            ..Default::default()
        },
        ChartPaintLimits {
            colors: ColorLimits {
                max_percentage_bytes: 1,
                ..Default::default()
            },
            ..Default::default()
        },
    ] {
        let error = inspect(&bytes, limits, Default::default(), &|| false).unwrap_err();
        assert!(
            error.to_string().to_lowercase().contains("limit"),
            "{error:?}"
        );
    }
    for limits in [
        SourceLimits {
            max_paint_elements: 1,
            ..Default::default()
        },
        SourceLimits {
            max_line_elements: 0,
            ..Default::default()
        },
    ] {
        assert!(inspect(&bytes, Default::default(), limits, &|| false).is_err());
    }
    let steps = Cell::new(0);
    inspect(&bytes, Default::default(), Default::default(), &|| {
        steps.set(steps.get() + 1);
        false
    })
    .unwrap();
    for stop in (0..steps.get()).step_by((steps.get() / 32).max(1)) {
        let i = Cell::new(0);
        assert!(
            inspect(&bytes, Default::default(), Default::default(), &|| {
                let n = i.get();
                i.set(n + 1);
                n >= stop
            })
            .unwrap_err()
            .to_string()
            .to_lowercase()
            .contains("cancel")
        );
    }
}
