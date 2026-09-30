//! Source circular data paths + declared point/series paint into shared Draw IR.
//! This is a plot primitive: labels, legends and automatic chart layout are not
//! silently treated as part of the compiled scene.
mod types;
use crate::{
    source_chart::{self, SourceCircularError},
    source_stroke,
};
use mo_geometry::Fixed;
use mo_opc::PackageRead;
use mo_presentation_source::{
    PptxError,
    source::{
        SourceIndex, SourceLimits,
        charts::{paints, styles::*},
        color::{ColorProfile, ColorSample},
        fill::resolve::{EffectiveFill, FillOrigin},
    },
};
use mo_raster::{Brush, FillPath};
use mo_render::{DrawScene, PathInstance};
pub use types::*;

fn unresolved(ordinal: u32, series: u32, point: u32, reason: &'static str) -> SourceCircularError {
    SourceCircularError::Unresolved {
        source_ordinal: ordinal,
        series_index: Some(series),
        point_index: Some(point),
        reason,
    }
}
fn ordinal(origin: &FillOrigin) -> Result<u32, &'static str> {
    match origin {
        FillOrigin::Chart { source_ordinal, .. } => Ok(*source_ordinal),
        _ => Err("chart paint cannot use an implicit shape color"),
    }
}
fn paint(
    fill: &EffectiveFill,
    prepared: &PreparedChartStyles<'_>,
) -> Result<ChartPlotPaint, &'static str> {
    match fill {
        EffectiveFill::None { declared_by } => Ok(ChartPlotPaint::None {
            declaration: ordinal(declared_by)?,
        }),
        EffectiveFill::Solid { declared_by, color } => {
            if color.context_owner.is_some() {
                return Err("unexpected chart color owner");
            }
            let source_ordinal = ordinal(&color.color.declared_by)?;
            let evaluated = prepared
                .color(source_ordinal)
                .ok_or("chart color binding missing")?;
            let ColorSample::Resolved { rgba8, .. } = evaluated.outcome else {
                return Err("chart color context unresolved");
            };
            Ok(ChartPlotPaint::Solid {
                declaration: ordinal(declared_by)?,
                color: source_ordinal,
                rgba8,
            })
        }
        _ => Err("chart gradient, image or pattern paint requires native shader layout"),
    }
}
pub fn compile(
    package: &dyn PackageRead,
    index: &SourceIndex,
    request: &SourceChartPlotRequest,
    source_limits: SourceLimits,
    limits: SourceChartPlotLimits,
    check: &dyn Fn() -> bool,
) -> Result<SourceChartPlot, SourceCircularError> {
    let paints = paints::query(
        package,
        index,
        &paints::SourceChartPaintQuery {
            expected_source_sha256: request.geometry.expected_source_sha256.clone(),
            object: request.geometry.object.clone(),
            profile: ColorProfile::Ecma3762016DraftV1,
            context: request.color_context.clone(),
        },
        source_limits,
        limits.paints,
        check,
    )?;
    let geometry =
        source_chart::compile_prepared(&paints.chart, &request.geometry, limits.geometry, check)?;
    let plot = paints
        .chart
        .plots
        .iter()
        .find(|p| p.source_ordinal == geometry.plot_source_ordinal)
        .expect("geometry selected an existing plot");
    if let Some(node) = plot
        .layout
        .markup
        .iter()
        .find(|m| m.kind != mo_presentation_source::source::charts::ChartMarkupKind::DataLabels)
    {
        return Err(SourceCircularError::Unresolved {
            source_ordinal: node.source_ordinal,
            series_index: None,
            point_index: None,
            reason: "chart plot markup requires semantic resolution",
        });
    }
    let mut prepared =
        PreparedChartStyles::new(&paints, limits.styles, check).map_err(|e| match e {
            ChartStyleError::Source(e) => SourceCircularError::Source(e),
            ChartStyleError::Unresolved {
                source_ordinal,
                reason,
            } => SourceCircularError::Unresolved {
                source_ordinal,
                series_index: None,
                point_index: None,
                reason,
            },
        })?;
    let mut scene = DrawScene {
        opacity_groups: vec![],
        paths: vec![],
        transforms: vec![],
        clips: vec![],
        instances: vec![],
    };
    let mut series_output = vec![];
    let mut point_output = vec![];
    let mut bound = Fixed::ZERO;
    for series in geometry.series {
        for (point, curve) in series.points.iter().zip(series.geometry.paths) {
            let fail = |ordinal, reason| unresolved(ordinal, series.index, point.index, reason);
            let style = prepared
                .resolve(geometry.plot_source_ordinal, series.index, point.index)
                .map_err(|e| match e {
                    ChartStyleError::Source(e) => SourceCircularError::Source(e),
                    ChartStyleError::Unresolved {
                        source_ordinal,
                        reason,
                    } => fail(source_ordinal, reason),
                })?;
            let fill = paint(&style.fill, &prepared).map_err(|e| fail(series.source_ordinal, e))?;
            let line =
                paint(&style.line_fill, &prepared).map_err(|e| fail(series.source_ordinal, e))?;
            let effects = style.effects.ok_or_else(|| {
                fail(
                    series.source_ordinal,
                    "automatic chart effects require style resolution",
                )
            })?;
            if !effects.is_explicitly_empty_list() {
                return Err(fail(
                    effects.source_ordinal,
                    "chart effects require native effect composition",
                ));
            }
            let stroke = if matches!(line, ChartPlotPaint::Solid { .. }) {
                Some(
                    source_stroke::stroke((&style.line_geometry).into())
                        .map_err(|e| fail(series.source_ordinal, e))?,
                )
            } else {
                None
            };
            // Preserve zero-valued point identity and style; never stroke a
            // fabricated degenerate contour.
            let path = if curve.commands.is_empty() {
                None
            } else {
                let path = u32::try_from(scene.paths.len())
                    .map_err(|_| PptxError::Limit("chart plot paths"))?;
                scene.paths.push(FillPath {
                    fill_rule: series.geometry.fill_rule,
                    commands: curve.commands,
                });
                for (paint, stroke) in [(&fill, None), (&line, stroke)] {
                    if let ChartPlotPaint::Solid { rgba8, .. } = paint {
                        if scene.instances.len() >= limits.max_draws {
                            return Err(PptxError::Limit("chart plot draws").into());
                        }
                        scene.instances.push(PathInstance {
                            blend: Default::default(),
                            clip: None,
                            path,
                            transform: None,
                            brush: Brush::Solid { rgba: *rgba8 },
                            stroke,
                        });
                    }
                }
                Some(path)
            };
            point_output.push(ChartPlotPoint {
                series_index: series.index,
                point_index: point.index,
                cache_source_ordinal: point.source_ordinal,
                path,
                fill,
                line,
                line_geometry: style.line_geometry,
                effects_source_ordinal: effects.source_ordinal,
            });
        }
        bound = bound.max(series.coordinate_error_bound);
        series_output.push(ChartPlotSeries {
            index: series.index,
            order: series.order,
            source_ordinal: series.source_ordinal,
            values_source_ordinal: series.values_source_ordinal,
            formula: series.formula,
            inner_radius: series.inner_radius,
            outer_radius: series.outer_radius,
            coordinate_error_bound: series.coordinate_error_bound,
        });
    }
    if check() {
        return Err(PptxError::Cancelled.into());
    }
    Ok(SourceChartPlot {
        profile: request.profile,
        source_sha256: geometry.source_sha256,
        object: geometry.object,
        chart_part: geometry.chart_part,
        chart_sha256: geometry.chart_sha256,
        plot_source_ordinal: geometry.plot_source_ordinal,
        native_kind: geometry.native_kind,
        data_authority: geometry.data_authority,
        external_data: geometry.external_data,
        color_mapping: paints.color_mapping.clone(),
        color_scheme: paints.color_scheme.clone(),
        theme_override: paints.theme_override.clone(),
        coordinate_error_bound: bound,
        scene,
        series: series_output,
        points: point_output,
    })
}
