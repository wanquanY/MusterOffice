#[path = "support/charts.rs"]
mod support;
use mo_opc::{Package, PackageLimits};
use mo_presentation_source::{
    PptxError,
    source::{charts::*, inspect_source},
};
use support::*;

#[test]
fn series_and_point_geometry_and_style_bindings_are_preserved_without_defaults() {
    let s=series().replace("</c:ser>",r#"<c:spPr/><c:explosion val="12"/><c:dLbls/><c:dPt><c:idx val="2"/><c:explosion/><c:bubble3D val="false"/><c:spPr/></c:dPt><c:futureGeometry val="opaque"/></c:ser>"#);
    let result = inspect(&chart(&s), Default::default()).unwrap();
    let s = &result.charts[0].plots[0].series[0];
    assert_eq!(s.layout.properties[0].kind, ChartPropertyKind::Explosion);
    assert_eq!(s.layout.properties[0].value.as_deref(), Some("12"));
    assert!(
        s.layout
            .markup
            .iter()
            .any(|m| m.kind == ChartMarkupKind::DataLabels)
    );
    assert_eq!(
        s.layout.unrecognized_children[0].local_name,
        "futureGeometry"
    );
    let p = &s.point_overrides[0];
    assert_eq!(p.index, 2);
    assert_eq!(p.layout.properties[0].kind, ChartPropertyKind::Explosion);
    assert_eq!(p.layout.properties[0].value, None);
    assert_eq!(p.layout.properties[1].value.as_deref(), Some("false"));
    assert!(p.layout.markup[0].source_ordinal > p.source_ordinal);
}

#[test]
fn opaque_extensions_duplicate_point_overrides_and_override_budgets_are_explicit() {
    let extension =
        r#"<c:extLst><c:ext uri="owned"><f:future xmlns:f="urn:owned-future"/></c:ext></c:extLst>"#;
    let s = series().replace("</c:ser>", &format!("{extension}</c:ser>"));
    let result = inspect(&chart(&s), Default::default()).unwrap();
    let part = &result.charts[0];
    assert_eq!(part.extension_ordinals.len(), 1);
    let markup = &part.plots[0].series[0].layout.markup[0];
    assert_eq!(markup.kind, ChartMarkupKind::Extensions);
    assert_eq!(markup.source_ordinal, part.extension_ordinals[0]);
    let point = r#"<c:dPt><c:idx val="0"/></c:dPt>"#;
    let duplicate = series().replace("</c:ser>", &format!("{point}{point}</c:ser>"));
    assert!(inspect(&chart(&duplicate), Default::default()).is_err());
    let single = series().replace("</c:ser>", &format!("{point}</c:ser>"));
    assert!(
        inspect(
            &chart(&single),
            SourceChartLimits {
                max_points: 8,
                ..Default::default()
            }
        )
        .is_err()
    );
}

fn inspect(chart: &str, limits: SourceChartLimits) -> Result<SourceCharts, PptxError> {
    let bytes = package(chart, &frame(2), CT, "chart", false);
    let package = Package::open(
        bytes.as_slice(),
        bytes.len() as u64,
        PackageLimits::default(),
        &|| false,
    )?;
    let index = inspect_source(&package, Default::default(), &|| false)?;
    query(
        &package,
        &index,
        &SourceChartQuery {
            expected_source_sha256: package.sha256().clone(),
            surface: "/ppt/slides/slide1.xml".into(),
        },
        Default::default(),
        limits,
        &|| false,
    )
}
fn with_axes(axes: &str) -> String {
    chart(&series()).replace("</c:plotArea>", &format!("{axes}</c:plotArea>"))
}
fn value_axis() -> &'static str {
    r#"<c:valAx><c:axId val="12"/><c:scaling><c:logBase val="1.0000000000000001"/><c:orientation val="maxMin"/><c:max val="1e+99"/><c:min val="-0.00000000000000001"/></c:scaling><c:delete/><c:axPos val="r"/><c:majorGridlines><c:spPr/></c:majorGridlines><c:minorGridlines/><c:title/><c:numFmt formatCode="0.00 &amp; kg" sourceLinked="false"/><c:majorTickMark val="in"/><c:minorTickMark val="none"/><c:tickLblPos val="nextTo"/><c:spPr/><c:txPr/><c:crossAx val="11"/><c:crossesAt val="-0.0"/><c:crossBetween val="between"/><c:majorUnit val="0.5"/><c:minorUnit val="0.1"/><c:dispUnits/></c:valAx>"#
}
#[test]
fn preserves_axis_source_order_exact_values_defaults_and_markup_bindings() {
    let axes = format!(
        r#"<c:catAx><c:axId val="11"/><c:scaling/><c:auto val="1"/><c:crosses val="autoZero"/><c:crossAx val="12"/></c:catAx>{}"#,
        value_axis()
    );
    let result = inspect(&with_axes(&axes), Default::default()).unwrap();
    let axes = &result.charts[0].axes;
    assert_eq!(axes.len(), 2);
    assert_eq!(axes[0].kind, ChartAxisKind::Category);
    assert_eq!(axes[1].kind, ChartAxisKind::Value);
    assert!(axes[0].scaling.as_ref().unwrap().properties.is_empty());
    let axis = &axes[1];
    let scaling = &axis.scaling.as_ref().unwrap().properties;
    assert_eq!(
        scaling
            .iter()
            .map(|p| (p.kind, p.value.as_deref()))
            .collect::<Vec<_>>(),
        [
            (ChartPropertyKind::LogBase, Some("1.0000000000000001")),
            (ChartPropertyKind::Orientation, Some("maxMin")),
            (ChartPropertyKind::Maximum, Some("1e+99")),
            (ChartPropertyKind::Minimum, Some("-0.00000000000000001")),
        ]
    );
    assert!(
        scaling
            .windows(2)
            .all(|p| p[0].source_ordinal < p[1].source_ordinal)
    );
    assert_eq!(axis.layout.properties[0].kind, ChartPropertyKind::Delete);
    assert_eq!(axis.layout.properties[0].value, None);
    assert!(
        axis.layout
            .properties
            .iter()
            .any(|p| p.kind == ChartPropertyKind::CrossesAt && p.value.as_deref() == Some("-0.0"))
    );
    assert!(
        !axis
            .layout
            .properties
            .iter()
            .any(|p| p.kind == ChartPropertyKind::Crosses)
    );
    let format = axis.number_format.as_ref().unwrap();
    assert_eq!(format.format_code.as_deref(), Some("0.00 & kg"));
    assert_eq!(format.source_linked.as_deref(), Some("false"));
    assert_eq!(
        axis.layout
            .markup
            .iter()
            .map(|p| p.kind)
            .collect::<Vec<_>>(),
        [
            ChartMarkupKind::MajorGridlines,
            ChartMarkupKind::MinorGridlines,
            ChartMarkupKind::Title,
            ChartMarkupKind::ShapeProperties,
            ChartMarkupKind::TextProperties,
            ChartMarkupKind::DisplayUnits,
        ]
    );
    assert_eq!(
        serde_json::from_str::<SourceCharts>(&serde_json::to_string(&result).unwrap()).unwrap(),
        result
    );
}
#[test]
fn plot_layout_keeps_lexical_declarations_and_does_not_supply_missing_defaults() {
    let xml = chart(&series()).replace("</c:barChart>", r#"<c:gapWidth/><c:overlap val="-100"/><c:varyColors val="0"/><c:serLines/><c:serLines/></c:barChart><c:doughnutChart><c:firstSliceAng val="270"/><c:holeSize val="50"/><c:dLbls/></c:doughnutChart>"#);
    let result = inspect(&xml, Default::default()).unwrap();
    let plots = &result.charts[0].plots;
    assert_eq!(
        plots[0]
            .layout
            .properties
            .iter()
            .map(|p| (p.kind, p.value.as_deref()))
            .collect::<Vec<_>>(),
        [
            (ChartPropertyKind::BarDirection, Some("col")),
            (ChartPropertyKind::Grouping, Some("clustered")),
            (ChartPropertyKind::GapWidth, None),
            (ChartPropertyKind::Overlap, Some("-100")),
            (ChartPropertyKind::VaryColors, Some("0"))
        ]
    );
    assert_eq!(plots[0].layout.markup.len(), 2);
    assert_eq!(
        plots[1]
            .layout
            .properties
            .iter()
            .map(|p| (p.kind, p.value.as_deref()))
            .collect::<Vec<_>>(),
        [
            (ChartPropertyKind::FirstSliceAngle, Some("270")),
            (ChartPropertyKind::HoleSize, Some("50"))
        ]
    );
    assert_eq!(plots[1].layout.markup[0].kind, ChartMarkupKind::DataLabels);
    // This API reads declarations; it must not pretend to validate every scalar's domain.
    let xml = xml.replace("val=\"50\"", "val=\"not-a-number\"");
    assert_eq!(
        inspect(&xml, Default::default()).unwrap().charts[0].plots[1]
            .layout
            .properties[1]
            .value
            .as_deref(),
        Some("not-a-number")
    );
}
#[test]
fn axes_and_cross_references_are_chart_local_without_inventing_unresolved_axes() {
    let axes = r#"<c:dateAx><c:axId val="11"/><c:baseTimeUnit val="days"/><c:majorTimeUnit val="months"/><c:minorTimeUnit val="days"/><c:lblOffset val="0"/></c:dateAx><c:serAx><c:axId val="12"/><c:tickLblSkip val="3"/><c:tickMarkSkip val="2"/><c:crossAx val="999"/></c:serAx>"#;
    let result = inspect(&with_axes(axes), Default::default()).unwrap();
    assert_eq!(
        result.charts[0]
            .axes
            .iter()
            .map(|a| a.kind)
            .collect::<Vec<_>>(),
        [ChartAxisKind::Date, ChartAxisKind::Series]
    );
    assert_eq!(
        result.charts[0].axes[1].layout.properties[2]
            .value
            .as_deref(),
        Some("999")
    );
    assert_eq!(result.charts[0].axes.len(), 2);
    assert!(matches!(
        inspect(
            &with_axes(axes),
            SourceChartLimits {
                max_axes: 1,
                ..Default::default()
            }
        ),
        Err(PptxError::Limit("chart axes"))
    ));
    assert!(
        inspect(
            &with_axes(&axes.replace("val=\"12\"", "val=\"11\"")),
            Default::default()
        )
        .is_err()
    );
}
#[test]
fn duplicate_layout_scalars_complex_nodes_and_scalar_children_are_rejected() {
    for xml in [
        with_axes(&format!("{}{}", value_axis(), value_axis())),
        with_axes(&value_axis().replace("<c:axPos val=\"r\"/>", "<c:axPos val=\"r\"/><c:axPos/>")),
        with_axes(&value_axis().replace("<c:scaling>", "<c:scaling/><c:scaling>")),
        with_axes(&value_axis().replace("<c:delete/>", "<c:delete>bad</c:delete>")),
        with_axes(&value_axis().replace("<c:delete/>", "<c:delete><c:v>1</c:v></c:delete>")),
        with_axes(&value_axis().replace("<c:spPr/>", "<c:spPr/><c:spPr/>")),
        chart(&series()).replace("<c:barDir val=\"col\"/>", "<c:barDir/><c:barDir/>"),
    ] {
        assert!(inspect(&xml, Default::default()).is_err(), "{xml}");
    }
}
#[test]
fn mce_fallback_nested_extensions_and_foreign_names_do_not_invent_properties() {
    let xml = with_axes(value_axis()).replace("<c:delete/>", r#"<mc:AlternateContent xmlns:mc="http://schemas.openxmlformats.org/markup-compatibility/2006" xmlns:z="urn:unknown"><mc:Choice Requires="z"><c:delete val="1"/></mc:Choice><mc:Fallback><c:delete val="0"/></mc:Fallback></mc:AlternateContent><c:extLst><c:ext uri="owned"><c:delete val="1"/><c:valAx><c:axId val="12"/></c:valAx></c:ext></c:extLst><a:delete val="1"/>"#);
    let result = inspect(&xml, Default::default()).unwrap();
    assert_eq!(result.charts[0].axes.len(), 1);
    assert_eq!(
        result.charts[0].axes[0].layout.properties[0]
            .value
            .as_deref(),
        Some("0")
    );
    assert!(result.charts[0].compatibility.selections[0].branches[1].selected);
}

#[test]
fn owned_layout_cases_exercise_public_parity_inputs() {
    let axes = format!(
        r#"<c:catAx><c:axId val="11"/><c:crossAx val="12"/></c:catAx>{}<c:dateAx><c:axId val="13"/><c:baseTimeUnit val="days"/></c:dateAx><c:serAx><c:axId val="14"/><c:tickLblSkip val="2"/></c:serAx>"#,
        value_axis()
    );
    let good = with_axes(&axes);
    let cases = [
        ("four-axes", good.clone(), true),
        ("doughnut", chart(&series()).replace("</c:barChart>", r#"</c:barChart><c:doughnutChart><c:holeSize val="50"/><c:firstSliceAng val="270"/></c:doughnutChart>"#), true),
        ("omitted-values", with_axes(r#"<c:catAx><c:axId val="11"/><c:delete/><c:scaling/><c:numFmt/></c:catAx>"#), true),
        ("unknown-enum-lexical", good.replace("val=\"maxMin\"", "val=\"future-enum\""), true),
        ("duplicate-axis", good.replace("<c:axId val=\"14\"/>", "<c:axId val=\"12\"/>"), false),
        ("duplicate-scaling", good.replace("<c:scaling>", "<c:scaling/><c:scaling>"), false),
        ("scalar-content", good.replace("<c:delete/>", "<c:delete><c:v>1</c:v></c:delete>"), false),
        ("duplicate-plot-property", good.replace("<c:barDir val=\"col\"/>", "<c:barDir/><c:barDir/>"), false),
    ];
    for (name, xml, expected) in cases {
        assert_eq!(
            inspect(&xml, Default::default()).is_ok(),
            expected,
            "{name}"
        );
        if let Ok(root) = std::env::var("MUSTER_OFFICE_CHART_LAYOUT_FIXTURES") {
            let directory = std::path::Path::new(&root).join(name);
            std::fs::create_dir(&directory).unwrap();
            let bytes = package(&xml, &frame(2), CT, "chart", false);
            let package = Package::open(
                bytes.as_slice(),
                bytes.len() as u64,
                Default::default(),
                &|| false,
            )
            .unwrap();
            let request = SourceChartQuery {
                expected_source_sha256: package.sha256().clone(),
                surface: "/ppt/slides/slide1.xml".into(),
            };
            std::fs::write(directory.join("source.pptx"), bytes).unwrap();
            std::fs::write(
                directory.join("request.json"),
                serde_json::to_vec(&request).unwrap(),
            )
            .unwrap();
            std::fs::write(
                directory.join("expected-status.txt"),
                if expected { "inspected" } else { "error" },
            )
            .unwrap();
        }
    }
}
