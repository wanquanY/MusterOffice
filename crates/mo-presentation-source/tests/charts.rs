#[path = "support/charts.rs"]
mod support;
use mo_opc::{Package, PackageLimits};
use mo_presentation_source::{
    PptxError,
    source::{SourceLimits, charts::*, inspect_source},
};
use support::*;
fn inspect(
    bytes: &[u8],
    limits: SourceChartLimits,
    check: &dyn Fn() -> bool,
) -> Result<SourceCharts, PptxError> {
    let package = Package::open(bytes, bytes.len() as u64, PackageLimits::default(), &|| {
        false
    })?;
    let index = inspect_source(&package, SourceLimits::default(), &|| false)?;
    query(
        &package,
        &index,
        &SourceChartQuery {
            expected_source_sha256: package.sha256().clone(),
            surface: "/ppt/slides/slide1.xml".into(),
        },
        SourceLimits::default(),
        limits,
        check,
    )
}
fn fixture(chart: &str) -> Vec<u8> {
    package(chart, &frame(2), CT, "chart", false)
}
#[test]
fn binds_shared_charts_preserves_sparse_levels_empty_missing_errors_and_formula() {
    let bytes = package(
        &chart(&series()),
        &(frame(2) + &frame(3)),
        CT,
        "chart",
        false,
    );
    let result = inspect(&bytes, SourceChartLimits::default(), &|| false).unwrap();
    assert_eq!(result.bindings.len(), 2);
    assert_eq!(result.charts.len(), 1);
    assert_eq!(result.bindings[0].object.native_id, 2);
    assert_eq!(result.bindings[1].chart, 0);
    let part = &result.charts[0];
    assert_eq!(part.data_authority, ChartDataAuthority::SourceCacheSnapshot);
    let plot = &part.plots[0];
    assert_eq!(plot.native_kind, "barChart");
    assert_eq!(plot.axis_ids, [11, 12]);
    let channels = &plot.series[0].channels;
    assert_eq!(channels.len(), 3);
    assert_eq!(channels[1].formula.as_deref(), Some("Sheet1!$A$2:$A$5"));
    let cat = channels[1].cache.as_ref().unwrap();
    assert_eq!(cat.levels.len(), 2);
    assert_eq!(cat.levels[0].len(), 2);
    assert_eq!(cat.levels[0][1].index, 2);
    let values = channels[2].cache.as_ref().unwrap();
    assert_eq!(values.format_code.as_deref(), Some("0.00"));
    assert_eq!(
        values.levels[0]
            .iter()
            .map(|p| p.value.as_deref())
            .collect::<Vec<_>>(),
        [Some("0"), Some(""), None, Some("#N/A")]
    );
    assert_eq!(values.levels[0][3].format_code.as_deref(), Some("General"));
    let external = part.external_data.as_ref().unwrap();
    assert_eq!(external.auto_update, Some(false));
    assert!(
        matches!(&external.target,ChartWorkbookTarget::Embedded{part,content_type,..} if part=="/ppt/embeddings/data.xlsx"&&content_type==WORKBOOK_TYPE)
    );
    let encoded = serde_json::to_string(&result).unwrap();
    let decoded: SourceCharts = serde_json::from_str(&encoded).unwrap();
    assert_eq!(decoded, result);
}
#[test]
fn external_workbook_is_only_a_host_requirement() {
    let bytes = package(&chart(&series()), &frame(2), CT, "chart", true);
    let result = inspect(&bytes, SourceChartLimits::default(), &|| false).unwrap();
    assert!(
        matches!(&result.charts[0].external_data.as_ref().unwrap().target,ChartWorkbookTarget::External{uri} if uri=="https://example.invalid/chart-data.xlsx")
    );
}
#[test]
fn duplicates_and_ambiguous_channels_fail_without_partial_results() {
    let source = series();
    for changed in [
        source.replace("idx=\"3\" formatCode", "idx=\"0\" formatCode"),
        source.replace("<c:val>", "<c:val><c:numLit/>"),
        source.replace(
            "<c:order val=\"0\"/>",
            "<c:order val=\"0\"/><c:order val=\"1\"/>",
        ),
        source.replace("<c:ptCount val=\"4\"/>", "<c:ptCount val=\"1\"/>"),
    ] {
        assert!(
            inspect(
                &fixture(&chart(&changed)),
                SourceChartLimits::default(),
                &|| false
            )
            .is_err()
        );
    }
}
#[test]
fn wrong_content_type_or_relationship_is_not_a_chart() {
    for bytes in [
        package(
            &chart(&series()),
            &frame(2),
            "application/xml",
            "chart",
            false,
        ),
        package(&chart(&series()), &frame(2), CT, "image", false),
    ] {
        assert!(inspect(&bytes, SourceChartLimits::default(), &|| false).is_err());
    }
}
#[test]
fn every_budget_and_cancellation_is_enforced() {
    let bytes = fixture(&chart(&series()));
    for limits in [
        SourceChartLimits {
            max_charts: 0,
            ..Default::default()
        },
        SourceChartLimits {
            max_bindings: 0,
            ..Default::default()
        },
        SourceChartLimits {
            max_series: 0,
            ..Default::default()
        },
        SourceChartLimits {
            max_points: 0,
            ..Default::default()
        },
        SourceChartLimits {
            max_metadata_bytes: 0,
            ..Default::default()
        },
        SourceChartLimits {
            max_elements: 0,
            ..Default::default()
        },
        SourceChartLimits {
            max_part_bytes: 1,
            ..Default::default()
        },
        SourceChartLimits {
            max_total_part_bytes: 1,
            ..Default::default()
        },
        SourceChartLimits {
            max_relationship_steps: 0,
            ..Default::default()
        },
    ] {
        assert!(inspect(&bytes, limits, &|| false).is_err());
    }
    assert!(matches!(
        inspect(&bytes, Default::default(), &|| true),
        Err(PptxError::Cancelled)
    ));
}
#[test]
fn missing_formula_cache_and_large_sparse_counts_are_distinct() {
    let s = r#"<c:ser><c:idx val="0"/><c:order val="0"/><c:tx><c:v>标题</c:v></c:tx><c:cat><c:strRef><c:f>Sheet1!A1:A9</c:f></c:strRef></c:cat><c:val><c:numLit><c:ptCount val="4294967295"/><c:pt idx="4294967294"><c:v>1e20</c:v></c:pt></c:numLit></c:val></c:ser>"#;
    let result = inspect(&fixture(&chart(s)), Default::default(), &|| false).unwrap();
    let c = &result.charts[0].plots[0].series[0].channels;
    assert_eq!(c[0].literal_text.as_deref(), Some("标题"));
    assert!(c[1].cache.is_none());
    assert_eq!(c[2].cache.as_ref().unwrap().levels[0].len(), 1);
    assert!(
        inspect(
            &fixture(&chart(&s.replace("<c:f>Sheet1!A1:A9</c:f>", ""))),
            Default::default(),
            &|| false
        )
        .is_err()
    );
}

#[test]
fn chart_mce_selection_and_extensions_do_not_invent_series() {
    let s = series();
    let chosen = format!(
        r#"<mc:AlternateContent xmlns:mc="http://schemas.openxmlformats.org/markup-compatibility/2006" xmlns:z="urn:unsupported"><mc:Choice Requires="z"><c:ser><c:idx val="0"/><c:order val="0"/></c:ser></mc:Choice><mc:Fallback>{s}</mc:Fallback></mc:AlternateContent><c:extLst><c:ext uri="owned"><c:ser><c:idx val="0"/><c:order val="0"/></c:ser></c:ext></c:extLst>"#
    );
    let result = inspect(&fixture(&chart(&chosen)), Default::default(), &|| false).unwrap();
    assert_eq!(result.charts[0].plots[0].series.len(), 1);
    assert_eq!(result.charts[0].plots[0].series[0].channels.len(), 3);
    assert_eq!(result.charts[0].compatibility.selections.len(), 1);
    assert!(result.charts[0].compatibility.selections[0].branches[1].selected);
}
#[test]
fn source_digest_surface_and_mid_scan_cancellation_are_checked() {
    let bytes = fixture(&chart(&series()));
    let package = Package::open(
        bytes.as_slice(),
        bytes.len() as u64,
        PackageLimits::default(),
        &|| false,
    )
    .unwrap();
    let index = inspect_source(&package, Default::default(), &|| false).unwrap();
    let request = SourceChartQuery {
        expected_source_sha256: package.sha256().clone(),
        surface: "/ppt/slides/slide1.xml".into(),
    };
    let mut foreign = request.clone();
    foreign.expected_source_sha256 = mo_common::Digest::from_sha256([0; 32]);
    assert!(
        query(
            &package,
            &index,
            &foreign,
            Default::default(),
            Default::default(),
            &|| false
        )
        .is_err()
    );
    foreign = request.clone();
    foreign.surface = "/ppt/slides/foreign.xml".into();
    assert!(
        query(
            &package,
            &index,
            &foreign,
            Default::default(),
            Default::default(),
            &|| false
        )
        .is_err()
    );
    let counter = std::cell::Cell::new(0);
    let result = query(
        &package,
        &index,
        &request,
        Default::default(),
        Default::default(),
        &|| {
            counter.set(counter.get() + 1);
            counter.get() > 5
        },
    );
    assert!(matches!(
        result,
        Err(PptxError::Cancelled)
            | Err(PptxError::Xml(mo_xml::XmlError::Cancelled))
            | Err(PptxError::Opc(mo_opc::OpcError::Cancelled))
    ));
}
#[test]
fn unsupported_surface_mce_branch_is_not_counted_as_a_visible_chart() {
    let frames = format!(
        r#"<mc:AlternateContent xmlns:mc="http://schemas.openxmlformats.org/markup-compatibility/2006" xmlns:z="urn:unsupported"><mc:Choice Requires="z">{}</mc:Choice><mc:Fallback>{}</mc:Fallback></mc:AlternateContent>"#,
        frame(2),
        frame(3)
    );
    let bytes = package(&chart(&series()), &frames, CT, "chart", false);
    let result = inspect(&bytes, Default::default(), &|| false).unwrap();
    assert_eq!(result.bindings.len(), 1);
    assert_eq!(result.bindings[0].object.native_id, 3);
}
