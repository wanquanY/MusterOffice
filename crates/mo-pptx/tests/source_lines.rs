mod support;
use mo_opc::{Package, PackageLimits, PartName, RewritePlan};
use mo_pptx::{
    source::{line::*, *},
    *,
};

fn fixture(line: &str, style: &str) -> Vec<u8> {
    let (d, defaults) = support::input();
    let bytes = export(
        &d,
        &defaults,
        &support::resources(),
        PptxLimits::default(),
        &|| false,
    )
    .unwrap();
    let p = package(&bytes);
    let name = PartName::new("/ppt/slides/slide1.xml").unwrap();
    let mut xml = String::from_utf8(p.read_part(&name, 1 << 20, &|| false).unwrap()).unwrap();
    let first = xml.find("<p:sp>").unwrap();
    let a = first + xml[first..].find("<p:spPr>").unwrap();
    let b = a + xml[a..].find("</p:spPr>").unwrap() + 9;
    xml.replace_range(a..b, &format!("<p:spPr>{line}</p:spPr>{style}"));
    let mut plan = RewritePlan::new();
    plan.replace_part(name, xml.into_bytes()).unwrap();
    plan.to_bytes(&p, &|| false).unwrap()
}
fn package(bytes: &[u8]) -> Package<&[u8]> {
    Package::open(bytes, bytes.len() as u64, PackageLimits::default(), &|| {
        false
    })
    .unwrap()
}
fn inspect(bytes: &[u8]) -> Result<SourceIndex, PptxError> {
    inspect_source(&package(bytes), SourceLimits::default(), &|| false)
}
fn first(index: &SourceIndex) -> &SourceObject {
    &index.surfaces["/ppt/slides/slide1.xml"].objects[0]
}

#[test]
fn absent_empty_and_zero_line_declarations_do_not_collapse() {
    assert!(first(&inspect(&fixture("", "")).unwrap()).line.is_none());
    let empty = inspect(&fixture("<a:ln><a:miter/></a:ln>", "")).unwrap();
    let line = first(&empty).line.as_ref().unwrap();
    assert_eq!(line.width, None);
    assert_eq!(line.cap, None);
    assert!(matches!(
        line.join,
        Some(SourceLineJoin::Miter { limit: None, .. })
    ));
    let zero = inspect(&fixture(
        "<a:ln w=\"0\" cap=\"flat\"><a:noFill/><a:miter lim=\"0\"/></a:ln>",
        "",
    ))
    .unwrap();
    let line = first(&zero).line.as_ref().unwrap();
    assert_eq!(line.width.unwrap().get(), 0);
    let Some(SourceLineJoin::Miter {
        limit: Some(limit), ..
    }) = &line.join
    else {
        panic!("missing explicit zero")
    };
    assert_eq!(limit.lexical(), "0");
}
#[test]
fn colors_percentages_dash_endpoints_and_style_reference_keep_native_semantics() {
    let bytes = fixture(
        r#"<a:ln w="12700" cap="rnd" cmpd="thickThin" algn="in"><a:solidFill><a:schemeClr val="accent2"><a:alpha val="50%"/><a:alpha val="+0050000"/></a:schemeClr></a:solidFill><a:custDash><a:ds d="12.345678901%" sp="0"/><a:ds d="+001" sp="25000"/></a:custDash><a:miter lim="400.00001%"/><a:headEnd type="triangle" w="lg"/><a:tailEnd/></a:ln>"#,
        r#"<p:style><a:lnRef idx="2"><a:srgbClr val="102030"/></a:lnRef></p:style>"#,
    );
    let index = inspect(&bytes).unwrap();
    let object = first(&index);
    let line = object.line.as_ref().unwrap();
    assert_eq!(line.cap, Some(NativeLineCap::Round));
    assert_eq!(line.compound, Some(NativeCompoundLine::ThickThin));
    assert_eq!(line.alignment, Some(NativePenAlignment::Inset));
    let Some(SourceLineFill::Solid {
        color: Some(color), ..
    }) = &line.fill
    else {
        panic!("lost solid color")
    };
    assert_eq!(color.transforms.len(), 2);
    let Some(SourceLineDash::Custom { stops, .. }) = &line.dash else {
        panic!("lost custom dash")
    };
    assert_eq!(stops.len(), 2);
    assert_eq!(stops[0].dash.lexical(), "12.345678901%");
    assert_eq!(
        line.head.as_ref().unwrap().width,
        Some(NativeLineEndSize::Large)
    );
    assert_eq!(line.tail.as_ref().unwrap().kind, None);
    let reference = object.line_reference.as_ref().unwrap();
    assert_eq!(reference.index, 2);
    assert!(reference.color.is_some());
    assert!(line.retained_ordinals.is_empty());
    assert!(
        index
            .themes
            .values()
            .filter_map(|t| t.format_scheme.as_ref())
            .flat_map(|s| &s.lines)
            .all(|l| l.line.is_some())
    );
}
#[test]
fn retained_extensions_cannot_inject_line_properties_or_colors() {
    let index=inspect(&fixture(r#"<a:ln cap="flat" future="retained"><a:gradFill/><a:round/><a:extLst><a:ext uri="owned"><a:ln cap="bad"/><a:solidFill><a:prstClr val="invalid"/></a:solidFill></a:ext></a:extLst></a:ln>"#,"")).unwrap();
    let line = first(&index).line.as_ref().unwrap();
    assert_eq!(line.cap, Some(NativeLineCap::Flat));
    assert_eq!(line.retained_ordinals.len(), 2);
    assert!(matches!(line.fill, Some(SourceLineFill::Gradient { .. })));
    assert!(matches!(line.join, Some(SourceLineJoin::Round { .. })));
}
#[test]
fn invalid_known_line_values_and_grammar_are_rejected() {
    for inner in [
        r#"<a:ln><a:gradFill><a:ln cap="bad"/></a:gradFill></a:ln>"#,
        r#"<a:ln w="-1"/>"#,
        r#"<a:ln w="20116801"/>"#,
        r#"<a:ln cap="round"/>"#,
        r#"<a:ln cmpd="single"/>"#,
        r#"<a:ln algn="center"/>"#,
        r#"<a:ln><a:miter lim="-1"/></a:ln>"#,
        r#"<a:ln><a:miter lim="1e2%"/></a:ln>"#,
        r#"<a:ln><a:round/><a:bevel/></a:ln>"#,
        r#"<a:ln><a:round/><a:noFill/></a:ln>"#,
        r#"<a:ln><a:custDash><a:ds d="1"/></a:custDash></a:ln>"#,
        r#"<a:ln><a:headEnd type="circle"/></a:ln>"#,
        r#"<a:ln><a:solidFill><a:srgbClr val="FFFFFF"/><a:srgbClr val="000000"/></a:solidFill></a:ln>"#,
        r#"<a:ln><a:solidFill><a:srgbClr val="FFFFFF"><a:alpha val="50000"><a:round/></a:alpha></a:srgbClr></a:solidFill></a:ln>"#,
        r#"<a:ln><a:custDash/><a:custDash/></a:ln>"#,
        r#"<a:ln><a:noFill><a:round/></a:noFill></a:ln>"#,
        r#"<a:ln/><a:ln/>"#,
    ] {
        assert!(inspect(&fixture(inner, "")).is_err(), "{inner}");
    }
    for reference in [
        r#"<a:lnRef/>"#,
        r#"<a:lnRef idx="-1"/>"#,
        r#"<a:lnRef idx="0"><a:noFill/></a:lnRef>"#,
        r#"<a:lnRef idx="0"/><a:lnRef idx="1"/>"#,
    ] {
        assert!(inspect(&fixture("", &format!("<p:style>{reference}</p:style>"))).is_err());
    }
}
#[test]
fn line_budgets_are_shared_across_surfaces_and_theme_entries() {
    let bytes = fixture(
        r#"<a:ln><a:custDash><a:ds d="1" sp="2"/></a:custDash></a:ln>"#,
        "",
    );
    let p = package(&bytes);
    for limits in [
        SourceLimits {
            max_line_elements: 3,
            ..Default::default()
        },
        SourceLimits {
            max_line_attribute_bytes: 1,
            ..Default::default()
        },
    ] {
        assert!(matches!(
            inspect_source(&p, limits, &|| false),
            Err(PptxError::Xml(mo_xml::XmlError::Limit(_)))
        ));
    }
}
#[test]
fn text_edits_preserve_line_xml_and_reindex_the_candidate() {
    let line = r#"<a:ln><a:solidFill><a:schemeClr val="accent1"/></a:solidFill><a:miter/></a:ln>"#;
    let bytes = fixture(line, "");
    let p = package(&bytes);
    let index = inspect(&bytes).unwrap();
    let obj = first(&index);
    let request = SourceTextEdits {
        expected_source_sha256: index.source_sha256.clone(),
        edits: vec![SourceTextEdit {
            target: SourceTextTarget {
                part: "/ppt/slides/slide1.xml".into(),
                object_id: obj.native_id,
                paragraph: 0,
                run: 0,
            },
            expected_text: obj.paragraphs[0][0].text.clone(),
            replacement: "edited & native".into(),
        }],
    };
    let candidate = edit_source_text(&p, &request, SourceLimits::default(), &|| false).unwrap();
    assert_eq!(first(&inspect(&candidate).unwrap()).line, obj.line);
    let part = PartName::new("/ppt/slides/slide1.xml").unwrap();
    assert!(
        String::from_utf8(
            package(&candidate)
                .read_part(&part, 1 << 20, &|| false)
                .unwrap()
        )
        .unwrap()
        .contains(line)
    );
}
