#[path = "support/charts.rs"]
mod support;
use mo_opc::{Package, PackageLimits};
use mo_presentation_source::{
    PptxError,
    source::{SourceLimits, SourceRunKind, charts::*, inspect_source, text::*},
};
use support::*;

fn inspect(
    xml: &str,
    limits: SourceChartLimits,
    text_limits: SourceLimits,
) -> Result<SourceCharts, PptxError> {
    let bytes = package(xml, &frame(2), CT, "chart", false);
    let p = Package::open(
        bytes.as_slice(),
        bytes.len() as u64,
        PackageLimits::default(),
        &|| false,
    )?;
    let index = inspect_source(&p, Default::default(), &|| false)?;
    query(
        &p,
        &index,
        &SourceChartQuery {
            expected_source_sha256: p.sha256().clone(),
            surface: "/ppt/slides/slide1.xml".into(),
        },
        text_limits,
        limits,
        &|| false,
    )
}
fn tx(size: u32, font: &str) -> String {
    format!(
        r#"<c:txPr><a:bodyPr/><a:lstStyle/><a:p><a:pPr><a:defRPr sz="{size}">{font}</a:defRPr></a:pPr></a:p></c:txPr>"#
    )
}
fn labels() -> String {
    format!(
        r#"<c:dLbls><c:dLbl><c:idx val="2"/><c:numFmt formatCode="General" sourceLinked="0"/>{}<c:showVal val="1"/><c:showCatName val="0"/><c:showPercent val="0"/><c:separator/></c:dLbl><c:numFmt formatCode="0.0%"/>{}<c:showVal val="0"/><c:showCatName/><c:showPercent val="1"/><c:separator> / </c:separator></c:dLbls>"#,
        tx(1200, r#"<a:latin typeface="Arial"/>"#),
        tx(1800, r#"<a:latin typeface="Arial"/>"#)
    )
}
fn annotated() -> String {
    let ser = series().replace("</c:ser>", &format!("{}</c:ser>", labels()));
    chart(&ser).replace("</c:chart>", &format!(r#"<c:legend><c:legendPos val="r"/><c:legendEntry><c:idx val="9"/><c:delete/></c:legendEntry><c:layout><c:manualLayout><c:layoutTarget val="outer"/><c:xMode/><c:yMode val="edge"/><c:wMode val="factor"/><c:x val="-0.00000000000000001"/><c:y val="2e-1"/><c:w val="0.2"/></c:manualLayout></c:layout><c:overlay val="0"/>{}</c:legend></c:chart>"#, tx(1400, "")))
}
fn property(n: &SourceChartAnnotation, kind: ChartPropertyKind) -> Option<&SourceChartProperty> {
    n.declarations.properties.iter().find(|p| p.kind == kind)
}
#[test]
fn preserves_group_point_legend_and_manual_declarations_without_defaults() {
    let result = inspect(&annotated(), Default::default(), Default::default()).unwrap();
    let part = &result.charts[0];
    let a = &part.annotations;
    assert_eq!(
        a.nodes.iter().map(|n| n.kind).collect::<Vec<_>>(),
        [
            ChartAnnotationKind::DataLabels,
            ChartAnnotationKind::DataLabel,
            ChartAnnotationKind::Legend,
            ChartAnnotationKind::LegendEntry,
            ChartAnnotationKind::Layout,
            ChartAnnotationKind::ManualLayout,
        ]
    );
    assert_eq!(
        a.nodes[0].parent_ordinal,
        part.plots[0].series[0].source_ordinal
    );
    assert_eq!(a.nodes[1].parent_ordinal, a.nodes[0].source_ordinal);
    assert_eq!(a.nodes[1].index, Some(2));
    assert_eq!(a.nodes[3].index, Some(9));
    assert_eq!(
        property(&a.nodes[0], ChartPropertyKind::ShowCategoryName)
            .unwrap()
            .value,
        None
    );
    assert_eq!(
        property(&a.nodes[1], ChartPropertyKind::ShowCategoryName)
            .unwrap()
            .value
            .as_deref(),
        Some("0")
    );
    assert_eq!(
        property(&a.nodes[0], ChartPropertyKind::Separator)
            .unwrap()
            .value
            .as_deref(),
        Some(" / ")
    );
    assert_eq!(
        property(&a.nodes[1], ChartPropertyKind::Separator)
            .unwrap()
            .value
            .as_deref(),
        Some("")
    );
    assert_eq!(
        property(&a.nodes[5], ChartPropertyKind::X)
            .unwrap()
            .value
            .as_deref(),
        Some("-0.00000000000000001")
    );
    assert_eq!(
        property(&a.nodes[5], ChartPropertyKind::XMode)
            .unwrap()
            .value,
        None
    );
    assert_eq!(
        a.nodes[0].number_format.as_ref().unwrap().source_linked,
        None
    );
    assert_eq!(
        a.nodes[1]
            .number_format
            .as_ref()
            .unwrap()
            .source_linked
            .as_deref(),
        Some("0")
    );
    assert_eq!(a.text_bodies.len(), 3);
    for (body, size) in a.text_bodies.iter().zip([1200, 1800, 1400]) {
        assert_eq!(
            body.styles.nodes[&body.source_ordinal].element,
            NativeTextElement::TxPr
        );
        assert!(body.styles.roots[0].owner.is_none());
        assert!(body.paragraphs[0].runs.is_empty());
        let v = serde_json::to_value(&body.styles).unwrap();
        assert!(
            v["nodes"]
                .as_object()
                .unwrap()
                .values()
                .any(|n| n["value"]["attributes"]["size"] == size)
        );
    }
    assert!(
        !a.text_bodies[2]
            .styles
            .nodes
            .values()
            .any(|n| n.element == NativeTextElement::Latin)
    );
    assert_eq!(
        serde_json::from_value::<SourceCharts>(serde_json::to_value(&result).unwrap()).unwrap(),
        result
    );
}
fn rich_title() -> String {
    format!(
        r#"<c:title><c:tx><c:rich><a:bodyPr/><a:lstStyle/><a:p><a:pPr><a:defRPr sz="2200"><a:solidFill><a:srgbClr val="126734"/></a:solidFill></a:defRPr></a:pPr><a:r><a:rPr lang="zh-CN"/><a:t xml:space="preserve">  中文 &amp;  text  </a:t></a:r><a:br/><a:fld id="{{11111111-1111-1111-1111-111111111111}}" type="slidenum"><a:t>7</a:t></a:fld><a:endParaRPr lang="en-US"/></a:p><a:p><a:r><a:t/></a:r></a:p></c:rich></c:tx>{}</c:title>"#,
        tx(1600, "")
    )
}
#[test]
fn rich_titles_use_shared_native_text_grammar_and_real_physical_bindings() {
    let xml = chart(&series()).replace("<c:plotArea>", &format!("{}<c:plotArea>", rich_title()));
    let a = inspect(&xml, Default::default(), Default::default())
        .unwrap()
        .charts
        .remove(0)
        .annotations;
    let body = &a.text_bodies[0];
    assert_eq!(
        body.styles.nodes[&body.source_ordinal].element,
        NativeTextElement::Rich
    );
    assert_eq!(body.paragraphs.len(), 2);
    let r = &body.paragraphs[0].runs;
    assert_eq!(
        r.iter()
            .map(|r| (r.kind, r.text.as_str()))
            .collect::<Vec<_>>(),
        [
            (SourceRunKind::Text, "  中文 &  text  "),
            (SourceRunKind::Break, ""),
            (SourceRunKind::Field, "7"),
        ]
    );
    assert!(r[1].text_source_ordinal.is_none());
    assert!(r[2].text_source_ordinal.is_some());
    assert_eq!(body.paragraphs[1].runs[0].text, "");
    assert!(body.paragraphs[1].runs[0].text_source_ordinal.is_some());
    for p in &body.paragraphs {
        assert_eq!(
            body.styles.nodes[&p.source_ordinal].element,
            NativeTextElement::P
        );
        for r in &p.runs {
            if let Some(t) = r.text_source_ordinal {
                assert_eq!(body.styles.nodes[&t].element, NativeTextElement::T);
            }
        }
    }
    let content = &a.nodes[0].text_source.as_ref().unwrap().content;
    assert!(
        matches!(content, ChartAnnotationTextContent::Rich { source_ordinal } if *source_ordinal == body.source_ordinal)
    );
}
#[test]
fn cached_title_keeps_formula_missing_empty_and_unknown_reference_content() {
    let title = r#"<c:title><c:tx><c:strRef future="1"><c:f>Sheet1!$A$1</c:f><c:strCache><c:ptCount val="3"/><c:pt idx="0"><c:v/></c:pt><c:pt idx="1"/><c:pt idx="2"><c:v>标题</c:v></c:pt></c:strCache><c:future/></c:strRef></c:tx></c:title>"#;
    let xml = chart(&series()).replace("<c:plotArea>", &format!("{title}<c:plotArea>"));
    let result = inspect(&xml, Default::default(), Default::default()).unwrap();
    let t = result.charts[0].annotations.nodes[0]
        .text_source
        .as_ref()
        .unwrap();
    let ChartAnnotationTextContent::StringReference { channel } = &t.content else {
        panic!()
    };
    assert_eq!(channel.formula.as_deref(), Some("Sheet1!$A$1"));
    assert_eq!(
        channel.cache.as_ref().unwrap().levels[0]
            .iter()
            .map(|p| p.value.as_deref())
            .collect::<Vec<_>>(),
        [Some(""), None, Some("标题")]
    );
    assert_eq!(t.retained_ordinals.len(), 2);
}
#[test]
fn chart_and_axis_text_defaults_are_bound_without_synthetic_shape_ids() {
    let axis = format!(
        r#"<c:valAx><c:axId val="1"/>{}{}</c:valAx>"#,
        rich_title(),
        tx(1200, "")
    );
    let xml = chart(&series())
        .replace("</c:plotArea>", &format!("{axis}</c:plotArea>"))
        .replace(
            "</c:chartSpace>",
            &format!("{}</c:chartSpace>", tx(1000, "")),
        );
    let r = inspect(&xml, Default::default(), Default::default()).unwrap();
    let p = &r.charts[0];
    assert_eq!(
        p.annotations.nodes[0].parent_ordinal,
        p.axes[0].source_ordinal
    );
    assert_eq!(p.annotations.text_bodies.len(), 4);
    assert_eq!(
        p.annotations.text_bodies[2].parent_ordinal,
        p.axes[0].source_ordinal
    );
    assert_eq!(p.annotations.text_bodies[3].parent_ordinal, 0);
}
#[test]
fn mce_selection_extensions_and_foreign_annotation_names_remain_explicit() {
    let ext = r#"<c:extLst><c:ext uri="own"><c:dLbl><c:idx val="2"/></c:dLbl></c:ext></c:extLst>"#;
    let xml = annotated().replace("<c:showCatName/>", r#"<mc:AlternateContent xmlns:mc="http://schemas.openxmlformats.org/markup-compatibility/2006" xmlns:z="urn:unknown"><mc:Choice Requires="z"><c:showCatName val="0"/></mc:Choice><mc:Fallback><c:showCatName val="1"/></mc:Fallback></mc:AlternateContent>"#)
        .replace("</c:dLbls>",&format!("<a:dLbl/><c:future/><c:showLeaderLines future=\"1\"/>{ext}</c:dLbls>"))
        .replace("<a:defRPr sz=\"1200\">", "<a:defRPr sz=\"1200\" custom=\"1\">")
        .replace("<a:bodyPr/>", &format!("<a:bodyPr>{ext}</a:bodyPr>"));
    let r = inspect(&xml, Default::default(), Default::default()).unwrap();
    let a = &r.charts[0].annotations;
    assert_eq!(a.nodes.len(), 6);
    assert_eq!(
        property(&a.nodes[0], ChartPropertyKind::ShowCategoryName)
            .unwrap()
            .value
            .as_deref(),
        Some("1")
    );
    assert_eq!(a.nodes[0].declarations.unrecognized_children.len(), 2);
    assert_eq!(a.nodes[0].declarations.retained_attribute_ordinals.len(), 1);
    assert!(
        a.text_bodies[0]
            .styles
            .nodes
            .values()
            .filter(|n| !n.retained_ordinals.is_empty())
            .count()
            >= 2
    );
    assert_eq!(r.charts[0].extension_ordinals.len(), 4);
    assert!(r.charts[0].compatibility.selections[0].branches[1].selected);
}
fn negative_cases() -> Vec<(&'static str, String)> {
    let good = annotated();
    vec![
        (
            "duplicate-point",
            good.replace("<c:idx val=\"2\"/>", "<c:idx val=\"2\"/><c:idx val=\"2\"/>"),
        ),
        (
            "duplicate-label",
            good.replace(
                "<c:dLbl>",
                "<c:dLbl><c:idx val=\"2\"/><c:delete/></c:dLbl><c:dLbl>",
            ),
        ),
        (
            "duplicate-legend",
            good.replace("</c:chart>", "<c:legend/></c:chart>"),
        ),
        (
            "duplicate-setting",
            good.replace("<c:showCatName/>", "<c:showCatName/><c:showCatName/>"),
        ),
        (
            "delete-with-format",
            good.replace("<c:showCatName/>", "<c:delete/><c:showCatName/>"),
        ),
        (
            "nested-scalar",
            good.replace(
                "<c:showCatName/>",
                "<c:showCatName><c:v>1</c:v></c:showCatName>",
            ),
        ),
        (
            "extension-in-scalar",
            good.replace(
                "<c:showCatName/>",
                "<c:showCatName><c:extLst/></c:showCatName>",
            ),
        ),
        (
            "separator-child",
            good.replace("<c:separator/>", "<c:separator><a:t>x</a:t></c:separator>"),
        ),
        ("missing-body", good.replace("<a:bodyPr/>", "")),
        (
            "wrong-body-namespace",
            good.replace("<a:bodyPr/>", "<c:bodyPr/>"),
        ),
        (
            "duplicate-body",
            good.replace("<a:bodyPr/>", "<a:bodyPr/><a:bodyPr/>"),
        ),
        (
            "duplicate-manual-coordinate",
            good.replace("<c:y val=\"2e-1\"/>", "<c:y/><c:y/>"),
        ),
        (
            "ambiguous-title",
            good.replace(
                "<c:plotArea>",
                &format!(
                    "{}<c:plotArea>",
                    rich_title().replace("<c:rich>", "<c:strRef><c:f>A1</c:f></c:strRef><c:rich>")
                ),
            ),
        ),
    ]
}
#[test]
fn ambiguous_or_malformed_annotations_fail_without_partial_results() {
    for (name, xml) in negative_cases() {
        assert!(
            inspect(&xml, Default::default(), Default::default()).is_err(),
            "{name}"
        );
    }
}
#[test]
fn text_and_annotation_budgets_accumulate_across_bodies() {
    let xml = annotated();
    for limits in [
        SourceChartLimits {
            max_annotations: 5,
            ..Default::default()
        },
        SourceChartLimits {
            max_text_bodies: 2,
            ..Default::default()
        },
    ] {
        assert!(inspect(&xml, limits, Default::default()).is_err());
    }
    // Each body is individually below ten elements; the three together are not.
    assert!(
        inspect(
            &xml,
            Default::default(),
            SourceLimits {
                max_text_style_elements: 10,
                ..Default::default()
            }
        )
        .is_err()
    );
    let title = rich_title();
    let xml = chart(&series()).replace("<c:plotArea>", &format!("{title}<c:plotArea>"));
    assert!(
        inspect(
            &xml,
            Default::default(),
            SourceLimits {
                max_text_bytes: 4,
                ..Default::default()
            }
        )
        .is_err()
    );
}
#[test]
fn text_budget_is_shared_across_distinct_chart_parts() {
    use mo_opc::{PackageBuilder, PartName, Relationship, RelationshipSource};
    let xml = chart(&series()).replace(
        "</c:chartSpace>",
        "<c:txPr><a:bodyPr/><a:p/></c:txPr></c:chartSpace>",
    );
    let bytes = package(
        &xml,
        &(frame(2) + &frame(3).replace("r:id=\"chart\"", "r:id=\"second\"")),
        CT,
        "chart",
        false,
    );
    let p = Package::open(
        bytes.as_slice(),
        bytes.len() as u64,
        Default::default(),
        &|| false,
    )
    .unwrap();
    let mut b = PackageBuilder::new();
    for (part, info) in p.parts() {
        if !part.as_str().ends_with(".rels") {
            b.add_part(
                part.clone(),
                info.content_type.clone(),
                p.read_part(part, 1 << 24, &|| false).unwrap(),
            )
            .unwrap();
        }
    }
    let second = PartName::new("/ppt/charts/chart2.xml").unwrap();
    b.add_part(second.clone(), CT.into(), xml.as_bytes().to_vec())
        .unwrap();
    let slide = RelationshipSource::Part(PartName::new("/ppt/slides/slide1.xml").unwrap());
    for (owner, rels) in p.relationships() {
        let mut rels = rels.clone();
        if *owner == slide {
            rels.push(
                Relationship::new(
                    owner,
                    "second".into(),
                    format!("{R}/chart"),
                    second.to_string(),
                    false,
                )
                .unwrap(),
            );
        }
        b.set_relationships(owner.clone(), rels.clone()).unwrap();
        if *owner == RelationshipSource::Part(PartName::new("/ppt/charts/chart1.xml").unwrap()) {
            b.set_relationships(RelationshipSource::Part(second.clone()), rels)
                .unwrap();
        }
    }
    let bytes = b.to_bytes(Default::default(), &|| false).unwrap();
    let p = Package::open(
        bytes.as_slice(),
        bytes.len() as u64,
        Default::default(),
        &|| false,
    )
    .unwrap();
    let index = inspect_source(&p, Default::default(), &|| false).unwrap();
    let q = SourceChartQuery {
        expected_source_sha256: p.sha256().clone(),
        surface: "/ppt/slides/slide1.xml".into(),
    };
    let r = query(
        &p,
        &index,
        &q,
        SourceLimits {
            max_text_style_elements: 6,
            ..Default::default()
        },
        Default::default(),
        &|| false,
    )
    .unwrap();
    assert_eq!(r.charts.len(), 2);
    assert!(
        query(
            &p,
            &index,
            &q,
            SourceLimits {
                max_text_style_elements: 5,
                ..Default::default()
            },
            Default::default(),
            &|| false
        )
        .is_err()
    );
    assert!(
        query(
            &p,
            &index,
            &q,
            Default::default(),
            SourceChartLimits {
                max_text_bodies: 1,
                ..Default::default()
            },
            &|| false
        )
        .is_err()
    );
}
#[test]
fn owned_annotation_corpus_exports_public_parity_inputs() {
    let mut cases = vec![
        ("labels-legend", annotated()),
        (
            "rich-title",
            chart(&series()).replace("<c:plotArea>", &format!("{}<c:plotArea>", rich_title())),
        ),
    ];
    let success_count = cases.len();
    cases.extend(negative_cases());
    for (i, (name, xml)) in cases.into_iter().enumerate() {
        let expected = if i < success_count {
            "inspected"
        } else {
            "error"
        };
        assert_eq!(
            inspect(&xml, Default::default(), Default::default()).is_ok(),
            i < success_count,
            "{name}"
        );
        if let Ok(root) = std::env::var("MO_CHART_ANNOTATION_CASE_DIR") {
            let dir = std::path::Path::new(&root).join(name);
            std::fs::create_dir(&dir).unwrap();
            let bytes = package(&xml, &frame(2), CT, "chart", false);
            let p = Package::open(
                bytes.as_slice(),
                bytes.len() as u64,
                Default::default(),
                &|| false,
            )
            .unwrap();
            std::fs::write(dir.join("source.pptx"), &bytes).unwrap();
            std::fs::write(
                dir.join("request.json"),
                serde_json::to_vec(&SourceChartQuery {
                    expected_source_sha256: p.sha256().clone(),
                    surface: "/ppt/slides/slide1.xml".into(),
                })
                .unwrap(),
            )
            .unwrap();
            std::fs::write(dir.join("expected-status.txt"), expected).unwrap();
        }
    }
}
