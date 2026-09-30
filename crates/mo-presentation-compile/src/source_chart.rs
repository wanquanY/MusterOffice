//! Native chart/cache bindings to circular plot geometry. Plot-area layout,
//! styles, labels, workbook recalculation and full-source admission stay separate.
mod data;
mod types;
use crate::{chart_geometry, interval::Interval as I, trig};
use mo_charts::sectors::{SectorDirection, SectorRequest};
use mo_geometry::Fixed;
use mo_opc::PackageRead;
use mo_presentation_source::{
    PptxError,
    source::{SourceIndex, SourceLimits, charts::*},
};
use num_bigint::BigInt;
pub use types::*;

fn unresolved(
    ordinal: u32,
    series: Option<u32>,
    point: Option<u32>,
    reason: &'static str,
) -> SourceCircularError {
    SourceCircularError::Unresolved {
        source_ordinal: ordinal,
        series_index: series,
        point_index: point,
        reason,
    }
}
fn cancel(check: &dyn Fn() -> bool) -> Result<(), SourceCircularError> {
    if check() {
        Err(PptxError::Cancelled.into())
    } else {
        Ok(())
    }
}
fn property(
    plot: &SourceChartPlot,
    kind: ChartPropertyKind,
    max: u16,
    percent: bool,
) -> Result<u16, SourceCircularError> {
    let p = plot
        .layout
        .properties
        .iter()
        .find(|p| p.kind == kind)
        .ok_or_else(|| {
            unresolved(
                plot.source_ordinal,
                None,
                None,
                "explicit source angle or hole size required",
            )
        })?;
    let value = p
        .value
        .as_deref()
        .ok_or_else(|| {
            unresolved(
                p.source_ordinal,
                None,
                None,
                "layout attribute value missing",
            )
        })?
        .trim_matches([' ', '\t', '\r', '\n']);
    let value = if percent {
        value.strip_suffix('%').unwrap_or(value)
    } else {
        value
    };
    let number = value
        .parse::<u16>()
        .ok()
        .filter(|n| *n <= max)
        .ok_or_else(|| unresolved(p.source_ordinal, None, None, "invalid angle or hole size"))?;
    Ok(number)
}
fn radius(
    value: Fixed,
    numerator: u32,
    denominator: u32,
) -> Result<(Fixed, Fixed), SourceCircularError> {
    I::fixed(value)
        .mul(&I::integer(i64::from(numerator)))
        .divide(i64::from(denominator))
        .q32()
        .map_err(|_| chart_geometry::ChartGeometryError::Range.into())
}
fn consume(remaining: &mut chart_geometry::ChartGeometryLimits, g: &chart_geometry::ChartGeometry) {
    // Internal computation has already admitted every counter against these
    // remaining budgets. Nothing is reset between series or concentric rings.
    remaining.max_paths -= g.work.paths;
    remaining.max_commands -= g.work.commands;
    remaining.max_arc_segments -= g.work.arc_segments;
    remaining.max_steps -= g.work.steps;
    remaining.sectors.max_points -= g.layout.work.points as usize;
    remaining.sectors.max_number_bytes -= g.layout.work.number_bytes as usize;
    remaining.sectors.max_scaled_decimal_digits -= g.layout.work.scaled_decimal_digits as usize;
    remaining.sectors.max_boundary_decimal_digits -= g.layout.work.boundary_decimal_digits as usize;
}

pub fn compile(
    package: &dyn PackageRead,
    index: &SourceIndex,
    request: &SourceCircularRequest,
    source_limits: SourceLimits,
    limits: SourceCircularLimits,
    check: &dyn Fn() -> bool,
) -> Result<SourceCircularGeometry, SourceCircularError> {
    cancel(check)?;
    if request.outer_radius.raw() <= 0
        || request.outer_radius.raw() > (27_273_042_316_900i128 << 32)
        || request.coordinate_tolerance.raw() <= 0
    {
        return Err(chart_geometry::ChartGeometryError::Invalid(
            "source chart radius or tolerance",
        )
        .into());
    }
    let sources = query(
        package,
        index,
        &SourceChartQuery {
            expected_source_sha256: request.expected_source_sha256.clone(),
            surface: request.object.part.clone(),
        },
        source_limits,
        limits.source,
        check,
    )?;
    let binding = sources
        .bindings
        .iter()
        .find(|b| b.object == request.object)
        .ok_or_else(|| PptxError::SourceConflict("chart object binding missing".into()))?;
    let chart = &sources.charts[binding.chart as usize];
    if let Some(ordinal) = chart.extension_ordinals.first() {
        return Err(unresolved(
            *ordinal,
            None,
            None,
            "opaque chart extensions require semantic resolution",
        ));
    }
    let plot = chart
        .plots
        .iter()
        .find(|p| p.source_ordinal == request.plot_source_ordinal)
        .ok_or_else(|| PptxError::SourceConflict("chart plot binding missing".into()))?;
    let pie = match plot.native_kind.as_str() {
        "pieChart" => true,
        "doughnutChart" => false,
        _ => {
            return Err(unresolved(
                plot.source_ordinal,
                None,
                None,
                "circular geometry requires a native 2D pie or doughnut plot",
            ));
        }
    };
    if let Some(node) = plot.layout.unrecognized_children.first() {
        return Err(unresolved(
            node.source_ordinal,
            None,
            None,
            "unrecognized native plot geometry",
        ));
    }
    if !plot.axis_ids.is_empty() {
        return Err(unresolved(
            plot.source_ordinal,
            None,
            None,
            "circular plot has axis references",
        ));
    }
    if let Some(extension) = plot
        .layout
        .markup
        .iter()
        .find(|p| p.kind == ChartMarkupKind::Extensions)
    {
        return Err(unresolved(
            extension.source_ordinal,
            None,
            None,
            "chart plot extension geometry unresolved",
        ));
    }
    if plot.series.is_empty() || (pie && plot.series.len() != 1) {
        return Err(unresolved(
            plot.source_ordinal,
            None,
            None,
            "pie requires one series; doughnut requires nonempty series",
        ));
    }
    if plot.series.len() > limits.geometry.max_paths as usize {
        return Err(chart_geometry::ChartGeometryError::Limit("source series").into());
    }
    let first_slice_degrees = property(plot, ChartPropertyKind::FirstSliceAngle, 360, false)?;
    let hole_percent = if pie {
        0
    } else {
        property(plot, ChartPropertyKind::HoleSize, 90, true)? as u8
    };
    if !pie && hole_percent < 10 {
        return Err(unresolved(
            plot.source_ordinal,
            None,
            None,
            "doughnut hole must be 10..90 percent",
        ));
    }
    let (start, start_error) = I::ratio(i64::from(first_slice_degrees % 360), 360)
        .q32()
        .map_err(|_| chart_geometry::ChartGeometryError::Range)?;
    let angle_error = I::fixed(start_error)
        .mul(&I::raw(
            BigInt::from(trig::PI_LO),
            BigInt::from(trig::PI_LO + 1),
        ))
        .mul(&I::integer(2))
        .mul(&I::fixed(request.outer_radius))
        .upper_q32()
        .map_err(|_| chart_geometry::ChartGeometryError::Range)?;
    let mut order: Vec<_> = plot.series.iter().collect();
    order.sort_by_key(|series| series.order);
    let count = u32::try_from(order.len())
        .map_err(|_| chart_geometry::ChartGeometryError::Limit("source series"))?;
    let denominator = 100u32
        .checked_mul(count)
        .ok_or(chart_geometry::ChartGeometryError::Range)?;
    let mut remaining = limits.geometry;
    let mut output = Vec::with_capacity(order.len());
    for (position, series) in order.into_iter().enumerate() {
        cancel(check)?;
        data::geometry_modifiers(series)?;
        let (weights, points, channel) = data::weights(
            series,
            remaining
                .sectors
                .max_points
                .min(remaining.max_paths as usize),
            check,
        )?;
        let boundary = |p: u32| {
            radius(
                request.outer_radius,
                u32::from(hole_percent) * count + (100 - u32::from(hole_percent)) * p,
                denominator,
            )
        };
        let (inner, ie) = boundary(position as u32)?;
        let (outer, oe) = boundary(position as u32 + 1)?;
        if inner >= outer {
            return Err(chart_geometry::ChartGeometryError::Precision.into());
        }
        let source_error = angle_error
            .raw()
            .checked_add(ie.max(oe).raw())
            .ok_or(chart_geometry::ChartGeometryError::Range)?;
        let tolerance = request
            .coordinate_tolerance
            .raw()
            .checked_sub(source_error)
            .filter(|v| *v > 0)
            .ok_or(chart_geometry::ChartGeometryError::Precision)?;
        let geometry = chart_geometry::compile(
            &chart_geometry::ChartGeometryRequest {
                sectors: SectorRequest {
                    start_turn: start,
                    direction: SectorDirection::Clockwise,
                    negative_weights: request.negative_weights,
                    weights,
                },
                center: request.center,
                outer_radius: outer,
                inner_radius: inner,
                coordinate_tolerance: Fixed::from_raw(tolerance),
            },
            remaining,
            check,
        )?;
        let bound = geometry
            .paths
            .iter()
            .map(|p| p.coordinate_error_bound.raw())
            .max()
            .unwrap_or(0)
            .checked_add(source_error)
            .ok_or(chart_geometry::ChartGeometryError::Range)?;
        if bound > request.coordinate_tolerance.raw() {
            return Err(chart_geometry::ChartGeometryError::Precision.into());
        }
        consume(&mut remaining, &geometry);
        output.push(SourceCircularSeries {
            index: series.index,
            order: series.order,
            source_ordinal: series.source_ordinal,
            values_source_ordinal: channel.source_ordinal,
            formula: channel.formula.clone(),
            points,
            layout: series.layout.clone(),
            point_overrides: series.point_overrides.clone(),
            inner_radius: inner,
            outer_radius: outer,
            source_geometry_error_bound: Fixed::from_raw(source_error),
            coordinate_error_bound: Fixed::from_raw(bound),
            geometry,
        });
    }
    cancel(check)?;
    Ok(SourceCircularGeometry {
        profile: request.profile,
        source_sha256: sources.source_sha256,
        object: request.object.clone(),
        chart_part: chart.part.clone(),
        chart_sha256: chart.sha256.clone(),
        plot_source_ordinal: plot.source_ordinal,
        native_kind: plot.native_kind.clone(),
        first_slice_degrees,
        hole_percent,
        data_authority: ChartDataAuthority::SourceCacheSnapshot,
        external_data: chart.external_data.clone(),
        series: output,
    })
}
