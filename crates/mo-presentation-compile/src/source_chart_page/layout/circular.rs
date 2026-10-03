use super::*;
use crate::source_chart::{self, SourceCircularProfile, SourceCircularRequest};
use mo_charts::sectors::{NegativeWeights, SectorWeight};
pub(super) fn build(
    source: &SourceChartPaints,
    area_ordinal: u32,
    plot: &SourceChartPlot,
    series: &[Series<'_>],
    area: Rect,
    out: &mut Layout,
    check: &dyn Fn() -> bool,
) -> Result<(), SourcePageError> {
    let part = &source.chart.part;
    if !source.chart.axes.is_empty() || series.len() != 1 {
        return Err(invalid(
            part,
            plot.source_ordinal,
            "multi-ring/axis-bound circular chart requires layout",
        ));
    }
    out.rect(area_ordinal, area, paint(source, area_ordinal)?)?;
    let s = &series[0];
    let label = labels(source, plot)
        .filter(|a| on(&a.declarations, P::ShowValue) || on(&a.declarations, P::ShowPercent));
    let label_style = label.map(|a| text(source, a.source_ordinal)).transpose()?;
    if let Some(a) = label
        && !matches!(
            value(&a.declarations, P::LabelPosition),
            None | Some("bestFit" | "ctr" | "inEnd")
        )
    {
        return Err(invalid(
            part,
            a.source_ordinal,
            "circular outside labels require leader-line layout",
        ));
    }
    let margin = label_style.as_ref().map_or(95250.0, |s| s.size * 0.8);
    let radius = area.w.min(area.h) / 2.0 - margin;
    if radius <= 0.0 {
        return Err(invalid(
            part,
            plot.source_ordinal,
            "circular chart has no plot region",
        ));
    }
    let cx = area.x + area.w / 2.0;
    let cy = area.y + area.h / 2.0;
    // Layout's own Q32 conversion error is charged separately below. Allocate
    // the remaining curve budget from the actual device scale and placement.
    let curve_tolerance = out.coordinate_tolerance.checked_sub(ERROR)?;
    if curve_tolerance.raw() <= 0 {
        return Err(RasterError::Precision.into());
    }
    let geometry = source_chart::compile_prepared(
        &source.chart,
        &SourceCircularRequest {
            expected_source_sha256: source.source_sha256.clone(),
            object: source.object.clone(),
            plot_source_ordinal: plot.source_ordinal,
            profile: SourceCircularProfile::DeclaredCircularDraftV1,
            center: point(cx, cy)?,
            outer_radius: fixed(radius)?,
            coordinate_tolerance: curve_tolerance,
            negative_weights: NegativeWeights::Reject,
        },
        Default::default(),
        check,
    )
    .map_err(|e| errors::circular(part, plot.source_ordinal, e))?;
    let series_geometry = &geometry.series[0];
    let weights = s
        .values
        .iter()
        .enumerate()
        .map(|(i, v)| {
            let v = v
                .as_ref()
                .ok_or_else(|| invalid(part, s.source.source_ordinal, "missing circular weight"))?;
            Ok(SectorWeight {
                point_index: i as u32,
                value: mo_charts::DecimalNumber::try_from(v.clone())
                    .map_err(|e| invalid(part, s.source.source_ordinal, e.to_string()))?,
            })
        })
        .collect::<Result<Vec<_>, SourcePageError>>()?;
    let ratios =
        mo_charts::sectors::ratios(&weights, NegativeWeights::Reject, Default::default(), check)
            .map_err(|e| errors::sector(part, plot.source_ordinal, e))?;
    for path in &series_geometry.geometry.paths {
        cancel(check)?;
        if path.commands.is_empty() {
            continue;
        }
        let p = point_paint(source, s.source, path.point_index)?;
        if p.fill.is_none() {
            return Err(invalid(
                part,
                s.source.source_ordinal,
                "circular point paint unresolved",
            ));
        }
        let start = out.draws.len();
        out.path(s.source.source_ordinal, path.commands.clone(), p);
        for draw in &mut out.draws[start..] {
            draw.error = series_geometry.coordinate_error_bound.checked_add(ERROR)?;
        }
    }
    if let (Some(a), Some(style)) = (label, label_style) {
        let symbols = mo_charts::number_format::NumberSymbols {
            decimal_separator: ".".into(),
            group_separator: ",".into(),
        };
        let mut formatter =
            mo_charts::number_format::Formatter::new(&symbols, Default::default(), check)
                .map_err(|e| errors::format(part, a.source_ordinal, e))?;
        let inner = series_geometry.inner_radius.raw() as f64 / 4294967296.0;
        let r = if inner == 0.0 {
            radius * 0.65
        } else {
            (radius + inner) / 2.0
        };
        for sector in &series_geometry.geometry.layout.sectors {
            if sector.zero_weight || ratios.zero_total {
                continue;
            }
            let index = sector.point_index as usize;
            let text = if on(&a.declarations, P::ShowPercent) {
                let code = a
                    .number_format
                    .as_ref()
                    .and_then(|f| f.format_code.as_deref())
                    .unwrap_or("0%");
                let display = formatter
                    .ratio(
                        &ratios.numerators[index].numerator,
                        &ratios.denominator,
                        code,
                    )
                    .map_err(|e| errors::format(part, a.source_ordinal, e))?;
                if display.color.is_some() {
                    return Err(invalid(
                        part,
                        a.source_ordinal,
                        "number format color requires text color cascade",
                    ));
                }
                let mut text = String::new();
                for f in display.fragments {
                    if let mo_charts::number_format::NumberFragment::Text { value } = f {
                        text.push_str(&value);
                    } else {
                        return Err(invalid(
                            part,
                            a.source_ordinal,
                            "circular number format requires width layout",
                        ));
                    }
                }
                text
            } else {
                display(
                    s.values[index].as_ref().expect("validated weight"),
                    a.number_format
                        .as_ref()
                        .and_then(|f| f.format_code.as_deref())
                        .unwrap_or("General"),
                    part,
                    a.source_ordinal,
                    check,
                )?
            };
            let angle = (sector.start_turn.raw() + sector.end_turn.raw()) as f64 / 8589934592.0
                * std::f64::consts::TAU;
            let width = estimated_width(&text, style.size) + style.size * 0.4;
            out.label(
                a.source_ordinal,
                text,
                style.clone(),
                Rect {
                    x: cx + r * angle.sin() - width / 2.0,
                    y: cy - r * angle.cos() - style.size,
                    w: width,
                    h: style.size * 2.0,
                },
                0.5,
            );
        }
    }
    Ok(())
}
