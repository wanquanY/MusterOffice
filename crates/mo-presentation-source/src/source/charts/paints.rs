//! Native chart paint declarations and contextual color evaluation. Series,
//! point and automatic chart-style inheritance remain a separate computation.
mod read;
mod types;
use super::*;
use crate::source::{SourceColorMapRef, color, drawingml::SourceColor, fill::*, line::*, theme};
pub use types::*;

fn fill_colors<'a>(fill: &'a SourceFillDefinition, colors: &mut Vec<&'a SourceColor>) {
    match fill {
        SourceFillDefinition::Solid { color } => colors.extend(color),
        SourceFillDefinition::Gradient(g) => gradient_colors(g, colors),
        SourceFillDefinition::Pattern(p) => pattern_colors(p, colors),
        _ => {}
    }
}
fn gradient_colors<'a>(g: &'a SourceGradientFill, colors: &mut Vec<&'a SourceColor>) {
    if let Some(stops) = &g.stops {
        colors.extend(stops.entries.iter().map(|s| &s.color));
    }
}
fn pattern_colors<'a>(p: &'a SourcePatternFill, colors: &mut Vec<&'a SourceColor>) {
    colors.extend(p.foreground.iter().chain(&p.background).map(|c| &c.color));
}
pub fn query(
    package: &dyn PackageRead,
    index: &SourceIndex,
    request: &SourceChartPaintQuery,
    source_limits: SourceLimits,
    limits: ChartPaintLimits,
    check: &dyn Fn() -> bool,
) -> Result<SourceChartPaints, PptxError> {
    let mut source = super::query(
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
    let binding = source
        .bindings
        .iter()
        .find(|b| b.object == request.object)
        .ok_or_else(|| invalid("chart paint object binding missing"))?;
    // The binding query read the surface and each distinct chart once. Charge
    // this stage's chart reread and optional theme against the same byte budget.
    let surface_name = PartName::new(&request.object.part)?;
    let mut bytes_left = (limits.source.max_total_part_bytes as u64)
        .checked_sub(package.parts()[&surface_name].byte_length)
        .ok_or(PptxError::Limit("chart paint total part bytes"))?;
    for item in &source.charts {
        bytes_left = bytes_left
            .checked_sub(item.byte_length.get())
            .ok_or(PptxError::Limit("chart paint total part bytes"))?;
    }
    let chart = source.charts.swap_remove(binding.chart as usize);
    bytes_left = bytes_left
        .checked_sub(chart.byte_length.get())
        .ok_or(PptxError::Limit("chart paint total part bytes"))?;
    let part = PartName::new(&chart.part)?;
    let bytes = package.read_part(&part, limits.source.max_part_bytes as u64, check)?;
    let mut parsed = read::read(&bytes, source_limits, limits.max_declarations, check)?;
    let overlay = super::context::read_theme_override(
        package,
        &chart,
        source_limits,
        limits.source,
        &mut bytes_left,
        check,
    )?;
    let surface = &index.surfaces[&request.object.part];
    let mut session = color::Session::new(
        index,
        surface,
        request.profile,
        &request.context,
        limits.colors,
        check,
    )?;
    let scheme = overlay
        .as_ref()
        .and_then(|(part, t)| t.color_scheme.as_ref().map(|s| (part.as_str(), s)));
    session.overlay(parsed.color_map.as_ref().map(|(_, m)| m), scheme);
    let mut count = 0usize;
    for decl in &mut parsed.declarations {
        cancelled(check)?;
        let mut colors = vec![];
        if let Some(fill) = &decl.fill {
            fill_colors(&fill.definition, &mut colors);
        }
        if let Some(line) = &decl.line {
            match &line.fill {
                Some(SourceLineFill::Solid { color, .. }) => colors.extend(color),
                Some(SourceLineFill::Gradient { gradient, .. }) => {
                    gradient_colors(gradient, &mut colors)
                }
                Some(SourceLineFill::Pattern { pattern, .. }) => {
                    pattern_colors(pattern, &mut colors)
                }
                _ => {}
            }
        }
        for c in colors {
            if count >= limits.colors.max_queries {
                return Err(PptxError::Limit("chart paint color queries"));
            }
            count += 1;
            let outcome = session.expression(
                color::ExpressionRef {
                    value: &c.value,
                    transforms: &c.transforms,
                },
                None,
            )?;
            decl.colors.push(ChartPaintColor {
                source_ordinal: c.source_ordinal,
                outcome: color::ColorSample::from_computed(outcome.color),
                dependencies: outcome.dependencies,
                notices: outcome.notices,
            });
        }
    }
    let mut text_colors = Vec::new();
    for body in &chart.annotations.text_bodies {
        for node in body.styles.nodes.values() {
            cancelled(check)?;
            let mut colors = Vec::new();
            if let crate::source::text::SourceTextValue::Fill { fill } = &node.value {
                fill_colors(&fill.definition, &mut colors);
            }
            for c in colors {
                if count >= limits.colors.max_queries {
                    return Err(PptxError::Limit("chart text color queries"));
                }
                count += 1;
                let outcome = session.expression(
                    color::ExpressionRef {
                        value: &c.value,
                        transforms: &c.transforms,
                    },
                    None,
                )?;
                text_colors.push(ChartPaintColor {
                    source_ordinal: c.source_ordinal,
                    outcome: color::ColorSample::from_computed(outcome.color),
                    dependencies: outcome.dependencies,
                    notices: outcome.notices,
                });
            }
        }
    }
    cancelled(check)?;
    Ok(SourceChartPaints {
        source_sha256: index.source_sha256.clone(),
        object: request.object.clone(),
        profile: request.profile,
        color_mapping: parsed
            .color_map
            .as_ref()
            .map(|(ordinal, _)| SourceColorMapRef {
                part: chart.part.clone(),
                source_ordinal: *ordinal,
            })
            .or_else(|| surface.resolved_color_mapping.clone()),
        color_scheme: scheme
            .map(|(part, s)| theme::SourceThemeSchemeRef {
                part: part.into(),
                source_ordinal: s.source_ordinal,
            })
            .or_else(|| surface.theme_selection.colors.clone()),
        theme_override: overlay.as_ref().map(|(part, t)| ChartThemeOverride {
            part: part.clone(),
            sha256: t.sha256.clone(),
        }),
        chart,
        declarations: parsed.declarations,
        text_colors,
    })
}
