#[path = "support/chart_paints.rs"]
mod paints;
use mo_opc::{Package, PackageLimits};
use mo_presentation_source::{
    PptxError,
    source::{
        charts::*,
        text::{NativeTextElement as N, cascade::*, fonts::*},
        *,
    },
};
use std::cell::Cell;

fn inspect(bytes: &[u8]) -> (SourceIndex, SourceChartPart) {
    let p = Package::open(bytes, bytes.len() as u64, Default::default(), &|| false).unwrap();
    let index = inspect_source(&p, Default::default(), &|| false).unwrap();
    let charts = query(
        &p,
        &index,
        &SourceChartQuery {
            expected_source_sha256: p.sha256().clone(),
            surface: "/ppt/slides/slide1.xml".into(),
        },
        Default::default(),
        Default::default(),
        &|| false,
    )
    .unwrap();
    (index, charts.charts.into_iter().next().unwrap())
}
fn tx(attributes: &str, children: &str) -> String {
    format!(
        "<c:txPr><a:bodyPr/><a:p><a:pPr><a:defRPr {attributes}>{children}</a:defRPr></a:pPr></a:p></c:txPr>"
    )
}
fn fixture(point: &str, group: &str) -> Vec<u8> {
    paints::fixture(
        &format!("<c:dLbls><c:dLbl><c:idx val=\"0\"/>{point}</c:dLbl>{group}</c:dLbls>"),
        None,
        None,
        None,
    )
}
fn roots(chart: &SourceChartPart) -> Vec<u32> {
    chart
        .annotations
        .text_bodies
        .iter()
        .filter(|b| b.styles.nodes[&b.source_ordinal].element == N::TxPr)
        .map(|b| b.source_ordinal)
        .collect()
}
fn cascaded(value: ChartTextOutcome) -> ChartTextCascade {
    match value {
        ChartTextOutcome::Cascaded { text } => *text,
        other => panic!("{other:?}"),
    }
}
#[test]
fn declared_layers_keep_chart_origins_whole_slots_and_no_shape_defaults() {
    let bytes = fixture(
        &tx(
            "sz=\"1200\" b=\"0\"",
            "<a:noFill/><a:latin typeface=\"Point Font\"/>",
        ),
        &tx(
            "sz=\"1800\" b=\"1\" i=\"1\"",
            "<a:solidFill><a:srgbClr val=\"FF0000\"/></a:solidFill><a:latin typeface=\"Group Font\"/><a:ea typeface=\"中文字体\"/>",
        ),
    );
    let (index, chart) = inspect(&bytes);
    let ids = roots(&chart);
    let mut resolver = ChartTextResolver::new(&chart, Default::default(), &|| false).unwrap();
    let result = cascaded(resolver.resolve(ids[0], &ids[1..]).unwrap());
    let context = ChartTypefaceContext::new(
        &index,
        &SourceObjectRef {
            part: "/ppt/slides/slide1.xml".into(),
            native_id: 2,
        },
        &chart,
        None,
    )
    .unwrap();
    let TypefaceOutcome::Named { font } = context
        .resolve(
            &resolver,
            &result,
            0,
            None,
            NativeFontSlot::Latin,
            None,
            Default::default(),
            &|| false,
        )
        .unwrap()
    else {
        panic!()
    };
    assert_eq!(font.typeface, "Point Font");
    assert!(font.theme.is_none());
    let style = &result.paragraphs[0].end_style;
    assert_eq!(style.attributes.size, Some(1200));
    assert_eq!(style.attributes.bold, Some(false));
    assert_eq!(style.attributes.italic, Some(true));
    assert!(style.attributes.spacing.is_none());
    assert!(result.paragraphs[0].attributes.alignment.is_none());
    assert_eq!(style.declarations[&CharacterSlot::Fill].element, N::NoFill);
    for slot in [
        CharacterSlot::Latin,
        CharacterSlot::EastAsian,
        CharacterSlot::Fill,
    ] {
        let d = &style.declarations[&slot];
        assert!(matches!(&d.origin, TextStyleOrigin::Chart { part, .. } if part == &chart.part));
        resolver.declaration(d).unwrap();
    }
    let mut foreign = style.declarations[&CharacterSlot::Latin].clone();
    if let TextStyleOrigin::Chart {
        body_source_ordinal,
        ..
    } = &mut foreign.origin
    {
        *body_source_ordinal = ids[1];
    }
    assert!(resolver.declaration(&foreign).is_err());
    assert!(resolver.resolve(ids[0], &[ids[0]]).is_err());
    assert!(resolver.resolve(u32::MAX, &[]).is_err());
}
#[test]
fn rich_runs_breaks_and_insertion_style_share_native_cascade_without_forged_shapes() {
    let rich = "<c:tx><c:rich><a:bodyPr/><a:p><a:pPr><a:defRPr i=\"1\"/></a:pPr><a:r><a:rPr b=\"0\"/><a:t>中文</a:t></a:r><a:br/><a:r><a:rPr sz=\"1600\"/><a:t>42</a:t></a:r><a:endParaRPr sz=\"2400\"/></a:p></c:rich></c:tx>";
    let bytes = fixture(
        rich,
        &tx("sz=\"1200\" b=\"1\"", "<a:latin typeface=\"Owned\"/>"),
    );
    let (_, chart) = inspect(&bytes);
    let body = chart
        .annotations
        .text_bodies
        .iter()
        .find(|b| b.styles.nodes[&b.source_ordinal].element == N::Rich)
        .unwrap();
    let mut resolver = ChartTextResolver::new(&chart, Default::default(), &|| false).unwrap();
    let result = cascaded(
        resolver
            .resolve(body.source_ordinal, &roots(&chart))
            .unwrap(),
    );
    let p = &result.paragraphs[0];
    assert_eq!(p.runs.len(), 3);
    assert_eq!(p.runs[0].style.attributes.size, Some(1200));
    assert_eq!(p.runs[0].style.attributes.bold, Some(false));
    assert_eq!(p.runs[0].style.attributes.italic, Some(true));
    assert_eq!(p.runs[1].kind, SourceRunKind::Break);
    assert_eq!(p.runs[2].style.attributes.size, Some(1600));
    assert_eq!(p.end_style.attributes.size, Some(2400));
    for (c, s) in p.runs.iter().zip(&body.paragraphs[0].runs) {
        assert_eq!(c.source_ordinal, s.source_ordinal);
    }
}
#[test]
fn unresolved_property_structures_and_retained_attributes_are_not_shape_fallbacks() {
    let base = tx("", "");
    for invalid in [
        base.replace("<a:p>", "<a:lstStyle><a:lvl1pPr/></a:lstStyle><a:p>"),
        base.replace("</c:txPr>", "<a:p/></c:txPr>"),
        base.replace("</a:p>", "<a:r><a:t>Not a property</a:t></a:r></a:p>"),
        base.replace("<a:defRPr ", "<a:defRPr future=\"1\" "),
    ] {
        let (_, chart) = inspect(&fixture(&invalid, ""));
        let id = roots(&chart)[0];
        let mut resolver = ChartTextResolver::new(&chart, Default::default(), &|| false).unwrap();
        assert!(matches!(
            resolver.resolve(id, &[]).unwrap(),
            ChartTextOutcome::Unresolved { .. }
        ));
    }
}
#[test]
fn budget_and_cancellation_accumulate_across_chart_text_scopes() {
    let (_, chart) = inspect(&fixture(&tx("sz=\"1200\"", ""), &tx("sz=\"1800\"", "")));
    let ids = roots(&chart);
    let limits = TextCascadeLimits {
        max_paragraphs: 1,
        ..Default::default()
    };
    let mut r = ChartTextResolver::new(&chart, limits, &|| false).unwrap();
    r.resolve(ids[0], &[]).unwrap();
    assert!(matches!(r.resolve(ids[1], &[]), Err(PptxError::Limit(_))));
    let cancelled = Cell::new(false);
    let check = || cancelled.get();
    let mut r = ChartTextResolver::new(&chart, Default::default(), &check).unwrap();
    cancelled.set(true);
    assert!(matches!(r.resolve(ids[0], &[]), Err(PptxError::Cancelled)));
    for limits in [
        TextCascadeLimits {
            max_steps: 0,
            ..Default::default()
        },
        TextCascadeLimits {
            max_lexical_bytes: 0,
            ..Default::default()
        },
    ] {
        let result = ChartTextResolver::new(&chart, limits, &|| false)
            .and_then(|mut r| r.resolve(ids[0], &[]));
        assert!(matches!(result, Err(PptxError::Limit(_))));
    }
}
fn font_theme(latin: &str, hans: &str) -> String {
    format!(
        "<a:themeOverride xmlns:a=\"{}\"><a:fontScheme name=\"Owned\"><a:majorFont><a:latin typeface=\"Major\"/><a:ea typeface=\"\"/><a:cs typeface=\"\"/></a:majorFont><a:minorFont><a:latin typeface=\"{latin}\"/><a:ea typeface=\"\"/><a:cs typeface=\"\"/><a:font script=\"Hans\" typeface=\"{hans}\"/></a:minorFont></a:fontScheme></a:themeOverride>",
        paints::charts::A
    )
}
#[test]
fn chart_font_scheme_uses_real_override_or_surface_fallback_and_explicit_script() {
    let styles = format!(
        "<c:dLbls>{}</c:dLbls>",
        tx(
            "sz=\"1200\"",
            "<a:latin typeface=\"+mn-lt\"/><a:ea typeface=\"+mn-ea\"/>",
        )
    );
    let parent = font_theme("Parent Latin", "Parent Hans");
    let chart_font = font_theme("Chart Latin", "Chart Hans");
    let empty = format!("<a:themeOverride xmlns:a=\"{}\"/>", paints::charts::A);
    for (override_xml, latin, hans) in [
        (None, "Parent Latin", "Parent Hans"),
        (Some(chart_font.as_str()), "Chart Latin", "Chart Hans"),
        (Some(empty.as_str()), "Parent Latin", "Parent Hans"),
    ] {
        let bytes = paints::fixture(&styles, None, Some(&parent), override_xml);
        let (index, chart) = inspect(&bytes);
        let package = Package::open(
            bytes.as_slice(),
            bytes.len() as u64,
            PackageLimits::default(),
            &|| false,
        )
        .unwrap();
        let overlay = context::read_theme_override(
            &package,
            &chart,
            Default::default(),
            Default::default(),
            &mut (1 << 24),
            &|| false,
        )
        .unwrap();
        let object = SourceObjectRef {
            part: "/ppt/slides/slide1.xml".into(),
            native_id: 2,
        };
        let context = ChartTypefaceContext::new(
            &index,
            &object,
            &chart,
            overlay.as_ref().map(|(p, t)| (p.as_str(), t)),
        )
        .unwrap();
        let mut resolver = ChartTextResolver::new(&chart, Default::default(), &|| false).unwrap();
        let text = cascaded(resolver.resolve(roots(&chart)[0], &[]).unwrap());
        for (slot, script, expected) in [
            (NativeFontSlot::Latin, None, latin),
            (NativeFontSlot::EastAsian, Some("Hans"), hans),
        ] {
            let TypefaceOutcome::Named { font } = context
                .resolve(
                    &resolver,
                    &text,
                    0,
                    None,
                    slot,
                    script,
                    Default::default(),
                    &|| false,
                )
                .unwrap()
            else {
                panic!()
            };
            assert_eq!(font.typeface, expected);
            assert!(font.theme.is_some());
            assert!(matches!(
                font.declared_by.origin,
                TextStyleOrigin::Chart { .. }
            ));
        }
        assert!(matches!(
            context
                .resolve(
                    &resolver,
                    &text,
                    0,
                    None,
                    NativeFontSlot::EastAsian,
                    None,
                    Default::default(),
                    &|| false
                )
                .unwrap(),
            TypefaceOutcome::Unresolved {
                reason: TypefaceUnresolved::ScriptRequired {}
            }
        ));
        assert!(matches!(
            context
                .resolve(
                    &resolver,
                    &text,
                    0,
                    None,
                    NativeFontSlot::Symbol,
                    None,
                    Default::default(),
                    &|| false
                )
                .unwrap(),
            TypefaceOutcome::Unresolved {
                reason: TypefaceUnresolved::MissingDeclaration {}
            }
        ));
        let other = chart.clone();
        let foreign = ChartTextResolver::new(&other, Default::default(), &|| false).unwrap();
        assert!(
            context
                .resolve(
                    &foreign,
                    &text,
                    0,
                    None,
                    NativeFontSlot::Latin,
                    None,
                    Default::default(),
                    &|| false
                )
                .is_err()
        );
    }
}
