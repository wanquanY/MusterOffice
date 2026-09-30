#[path = "../../mo-presentation-source/tests/support/charts.rs"]
mod support;
use mo_geometry::{Fixed, Point};
use mo_opc::{Package, PackageLimits};
use mo_presentation_compile::{chart_geometry::ChartGeometryError, source_chart::*};
use mo_presentation_source::{
    PptxError,
    source::{SourceIndex, SourceObjectRef, charts::*, inspect_source},
};
use std::cell::Cell;
use support::*;

fn series_data(index: u32, order: u32, values: &str, count: u32) -> String {
    format!(
        r#"<c:ser><c:idx val="{index}"/><c:order val="{order}"/><c:spPr><a:solidFill><a:srgbClr val="2468AC"/></a:solidFill></c:spPr><c:explosion val="0"/><c:dPt><c:idx val="1"/><c:bubble3D val="0"/><c:spPr><a:solidFill><a:srgbClr val="FEDCBA"/></a:solidFill></c:spPr></c:dPt><c:dLbls><c:showVal val="1"/></c:dLbls><c:val><c:numRef><c:f>Sheet1!$B$2:$B$4</c:f><c:numCache><c:formatCode>0.00</c:formatCode><c:ptCount val="{count}"/>{values}</c:numCache></c:numRef></c:val></c:ser>"#
    )
}
fn values() -> &'static str {
    r#"<c:pt idx="2"><c:v>3</c:v></c:pt><c:pt idx="0"><c:v>1</c:v></c:pt><c:pt idx="1"><c:v>2</c:v></c:pt>"#
}
fn owned(series: &str) -> Vec<u8> {
    let xml = format!(
        r#"<c:chartSpace xmlns:c="{C}" xmlns:a="{A}" xmlns:r="{R}"><c:chart><c:plotArea><c:doughnutChart>{series}<c:firstSliceAng val="17"/><c:holeSize val="50"/></c:doughnutChart></c:plotArea></c:chart><c:externalData r:id="data"><c:autoUpdate val="0"/></c:externalData></c:chartSpace>"#
    );
    package(&xml, &frame(2), CT, "chart", false)
}
fn package_index(bytes: &[u8]) -> (Package<&[u8]>, SourceIndex, SourceCircularRequest) {
    let p = Package::open(bytes, bytes.len() as u64, PackageLimits::default(), &|| {
        false
    })
    .unwrap();
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
    let request = SourceCircularRequest {
        expected_source_sha256: p.sha256().clone(),
        object: SourceObjectRef {
            part: "/ppt/slides/slide1.xml".into(),
            native_id: 2,
        },
        plot_source_ordinal: charts.charts[0].plots[0].source_ordinal,
        profile: SourceCircularProfile::DeclaredCircularDraftV1,
        center: Point {
            x: Fixed::ZERO,
            y: Fixed::ZERO,
        },
        outer_radius: Fixed::from_raw(1000 << 32),
        coordinate_tolerance: Fixed::from_raw(1 << 24),
        negative_weights: mo_charts::sectors::NegativeWeights::Reject,
    };
    (p, index, request)
}
fn run(bytes: &[u8]) -> Result<SourceCircularGeometry, SourceCircularError> {
    let (p, i, q) = package_index(bytes);
    compile(&p, &i, &q, Default::default(), Default::default(), &|| {
        false
    })
}

#[test]
fn native_cache_compiles_concentric_rings_in_series_order_and_points_in_index_order() {
    let bytes = owned(&(series_data(9, 1, values(), 3) + &series_data(4, 0, values(), 3)));
    let g = run(&bytes).unwrap();
    assert_eq!(g.series.iter().map(|s| s.index).collect::<Vec<_>>(), [4, 9]);
    assert_eq!(g.series[0].inner_radius.raw(), 500 << 32);
    assert_eq!(g.series[0].outer_radius.raw(), 750 << 32);
    assert_eq!(g.series[1].inner_radius, g.series[0].outer_radius);
    assert_eq!(g.series[1].outer_radius.raw(), 1000 << 32);
    assert_eq!(g.first_slice_degrees, 17);
    assert_eq!(g.hole_percent, 50);
    assert_eq!(g.data_authority, ChartDataAuthority::SourceCacheSnapshot);
    assert!(matches!(
        g.external_data.unwrap().target,
        ChartWorkbookTarget::Embedded { .. }
    ));
    for s in g.series {
        assert_eq!(
            s.points.iter().map(|p| p.index).collect::<Vec<_>>(),
            [0, 1, 2]
        );
        assert_eq!(
            s.geometry
                .layout
                .sectors
                .iter()
                .map(|p| p.point_index)
                .collect::<Vec<_>>(),
            [0, 1, 2]
        );
        assert_eq!(s.formula.as_deref(), Some("Sheet1!$B$2:$B$4"));
        assert!(
            s.points
                .iter()
                .all(|p| p.format_code.as_deref() == Some("0.00"))
        );
        assert!(s.source_geometry_error_bound.raw() > 0);
        assert!(s.coordinate_error_bound.raw() <= 1 << 24);
        assert!(
            s.layout
                .markup
                .iter()
                .any(|m| m.kind == ChartMarkupKind::DataLabels)
        );
        assert_eq!(s.point_overrides[0].index, 1);
    }
}

#[test]
fn fractional_hole_and_ring_widths_share_boundaries_without_dropping_uncertainty() {
    let series = series_data(7, 8, values(), 3)
        + &series_data(3, 2, values(), 3)
        + &series_data(9, 4, values(), 3);
    let bytes = owned(&series);
    let (p, i, mut q) = package_index(&bytes);
    q.outer_radius = Fixed::from_raw((1001 << 32) + 1);
    let g = compile(&p, &i, &q, Default::default(), Default::default(), &|| {
        false
    })
    .unwrap();
    assert_eq!(
        g.series.iter().map(|s| s.order).collect::<Vec<_>>(),
        [2, 4, 8]
    );
    for adjacent in g.series.windows(2) {
        assert_eq!(adjacent[0].outer_radius, adjacent[1].inner_radius);
    }
    assert_eq!(g.series[2].outer_radius, q.outer_radius);
    for s in g.series {
        assert!(s.coordinate_error_bound <= q.coordinate_tolerance);
        assert!(s.source_geometry_error_bound.raw() > 0);
    }
}

#[test]
fn sparse_blank_missing_and_error_values_are_not_converted_to_zero() {
    let missing = r#"<c:pt idx="0"><c:v>1</c:v></c:pt><c:pt idx="2"><c:v>3</c:v></c:pt>"#;
    let variants = [
        missing.into(),
        values().replace("<c:v>2</c:v>", "<c:v/>"),
        values().replace("<c:v>2</c:v>", ""),
        values().replace("<c:v>2</c:v>", "<c:v>#N/A</c:v>"),
    ];
    for v in variants {
        assert!(matches!(
            run(&owned(&series_data(7, 0, &v, 3))),
            Err(SourceCircularError::Unresolved {
                series_index: Some(7),
                point_index: Some(1),
                ..
            })
        ));
    }
}

#[test]
fn unsupported_geometry_remains_bound_to_its_physical_source() {
    for series in [
        series_data(7, 0, values(), 3).replace("explosion val=\"0\"", "explosion val=\"20\""),
        series_data(7, 0, values(), 3).replace("bubble3D val=\"0\"", "bubble3D val=\"1\""),
        series_data(7, 0, values(), 3).replace(
            "</c:ser>",
            "<c:extLst><c:ext uri=\"owned\"/></c:extLst></c:ser>",
        ),
        series_data(7, 0, values(), 3).replace("</c:ser>", "<c:futureGeometry/></c:ser>"),
    ] {
        assert!(matches!(
            run(&owned(&series)),
            Err(SourceCircularError::Unresolved {
                source_ordinal: 1..,
                ..
            })
        ));
    }
}

#[test]
fn every_ring_consumes_the_same_geometry_and_numeric_budgets() {
    let bytes = owned(&(series_data(1, 0, values(), 3) + &series_data(2, 1, values(), 3)));
    let (p, i, q) = package_index(&bytes);
    let g = compile(&p, &i, &q, Default::default(), Default::default(), &|| {
        false
    })
    .unwrap();
    let base = SourceCircularLimits::default();
    for n in 0..6 {
        let mut limits = base;
        match n {
            0 => limits.geometry.max_paths = 5,
            1 => {
                limits.geometry.max_commands = g
                    .series
                    .iter()
                    .map(|s| s.geometry.work.commands)
                    .sum::<u32>()
                    - 1
            }
            2 => {
                limits.geometry.max_arc_segments = g
                    .series
                    .iter()
                    .map(|s| s.geometry.work.arc_segments)
                    .sum::<u32>()
                    - 1
            }
            3 => {
                limits.geometry.max_steps =
                    g.series.iter().map(|s| s.geometry.work.steps).sum::<u32>() - 1
            }
            4 => limits.geometry.sectors.max_points = 5,
            _ => limits.geometry.sectors.max_number_bytes = 5,
        }
        assert!(matches!(
            compile(&p, &i, &q, Default::default(), limits, &|| false),
            Err(SourceCircularError::Geometry(
                ChartGeometryError::Limit(_)
                    | ChartGeometryError::Sectors(mo_charts::sectors::SectorError::Limit(_))
            ))
        ));
    }
}

#[test]
fn source_pins_selection_and_unrepresentable_ring_precision_fail_without_a_plan() {
    let bytes = owned(&(series_data(1, 0, values(), 3) + &series_data(2, 1, values(), 3)));
    let (p, i, mut q) = package_index(&bytes);
    q.object.native_id = 99;
    assert!(matches!(
        compile(&p, &i, &q, Default::default(), Default::default(), &|| {
            false
        }),
        Err(SourceCircularError::Source(PptxError::SourceConflict(_)))
    ));
    q.object.native_id = 2;
    q.plot_source_ordinal += 1;
    assert!(matches!(
        compile(&p, &i, &q, Default::default(), Default::default(), &|| {
            false
        }),
        Err(SourceCircularError::Source(PptxError::SourceConflict(_)))
    ));
    q.plot_source_ordinal -= 1;
    q.outer_radius = Fixed::from_raw(1);
    assert!(matches!(
        compile(&p, &i, &q, Default::default(), Default::default(), &|| {
            false
        }),
        Err(SourceCircularError::Geometry(ChartGeometryError::Precision))
    ));
}

#[test]
fn selected_cancel_checkpoints_never_return_a_partial_series_list() {
    let bytes = owned(&(series_data(1, 0, values(), 3) + &series_data(2, 1, values(), 3)));
    let (p, i, q) = package_index(&bytes);
    let calls = Cell::new(0usize);
    compile(&p, &i, &q, Default::default(), Default::default(), &|| {
        calls.set(calls.get() + 1);
        false
    })
    .unwrap();
    for stop in (0..calls.get())
        .step_by((calls.get() / 64).max(1))
        .chain(std::iter::once(calls.get() - 1))
    {
        let n = Cell::new(0usize);
        let result = compile(&p, &i, &q, Default::default(), Default::default(), &|| {
            let v = n.get();
            n.set(v + 1);
            v >= stop
        });
        assert!(result.unwrap_err().to_string().contains("cancel"));
    }
}
