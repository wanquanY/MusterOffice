mod cartesian;
mod circular;
use super::*;
use ChartMarkupKind as M;
use ChartPropertyKind as P;
use mo_presentation_source::source::charts::paints::SourceChartPaints;
use style::{audit, on, paint, point_paint, text, value};
const ZERO: Point = Point {
    x: Fixed::ZERO,
    y: Fixed::ZERO,
};
#[derive(Clone, Copy)]
struct Rect {
    x: f64,
    y: f64,
    w: f64,
    h: f64,
}
struct Series<'a> {
    source: &'a SourceChartSeries,
    title: String,
    categories: Vec<String>,
    values: Vec<Option<String>>,
}
fn channel(series: &SourceChartSeries, role: ChartChannelRole) -> Option<&SourceChartChannel> {
    series.channels.iter().find(|c| c.role == role)
}
fn values(
    series: &SourceChartSeries,
    role: ChartChannelRole,
    part: &str,
) -> Result<Vec<Option<String>>, SourcePageError> {
    let channel = channel(series, role)
        .ok_or_else(|| invalid(part, series.source_ordinal, "chart data channel missing"))?;
    let cache = channel.cache.as_ref().ok_or_else(|| {
        invalid(
            part,
            channel.source_ordinal,
            "chart cache missing; workbook refresh is not implicit",
        )
    })?;
    if cache.levels.len() != 1 {
        return Err(invalid(
            part,
            cache.source_ordinal,
            "multilevel chart data requires layout",
        ));
    }
    let count = cache.declared_point_count.unwrap_or_else(|| {
        cache.levels[0]
            .iter()
            .map(|p| p.index.saturating_add(1))
            .max()
            .unwrap_or(0)
    }) as usize;
    if count == 0 || count > 4096 {
        return Err(invalid(
            part,
            cache.source_ordinal,
            "chart point count outside page profile",
        ));
    }
    let mut out = vec![None; count];
    let mut seen = BTreeSet::new();
    for p in &cache.levels[0] {
        if p.index as usize >= count || !seen.insert(p.index) {
            return Err(invalid(
                part,
                p.source_ordinal,
                "chart point index conflict",
            ));
        }
        out[p.index as usize] = p.value.clone();
    }
    Ok(out)
}
fn numeric(value: &str, part: &str, ordinal: u32) -> Result<f64, SourcePageError> {
    mo_charts::DecimalNumber::try_from(value.to_owned())
        .map_err(|e| invalid(part, ordinal, e.to_string()))?;
    let v: f64 = value
        .parse()
        .map_err(|_| invalid(part, ordinal, "invalid chart number"))?;
    if !v.is_finite() || v.abs() > 1e12 || (v != 0.0 && v.abs() < 1e-12) {
        return Err(invalid(
            part,
            ordinal,
            "chart value outside bounded Cartesian profile",
        ));
    }
    Ok(v)
}
fn display(
    value: &str,
    code: &str,
    part: &str,
    ordinal: u32,
    check: &dyn Fn() -> bool,
) -> Result<String, SourcePageError> {
    if code == "General" {
        return Ok(value.to_owned());
    }
    let symbols = mo_charts::number_format::NumberSymbols {
        decimal_separator: ".".into(),
        group_separator: ",".into(),
    };
    let mut formatter =
        mo_charts::number_format::Formatter::new(&symbols, Default::default(), check)
            .map_err(|e| errors::format(part, ordinal, e))?;
    let n = mo_charts::DecimalNumber::try_from(value.to_owned())
        .map_err(|e| invalid(part, ordinal, e.to_string()))?;
    let d = formatter
        .decimal(&n, code)
        .map_err(|e| errors::format(part, ordinal, e))?;
    if d.color.is_some() {
        return Err(invalid(
            part,
            ordinal,
            "number format color requires text color cascade",
        ));
    }
    let mut out = String::new();
    for fragment in d.fragments {
        if let mo_charts::number_format::NumberFragment::Text { value } = fragment {
            out.push_str(&value);
        } else {
            return Err(invalid(
                part,
                ordinal,
                "number format requires width-dependent layout",
            ));
        }
    }
    Ok(out)
}
impl Layout {
    fn path(&mut self, ordinal: u32, commands: Vec<C>, paint: style::Paint) {
        if let Some(rgba) = paint.fill {
            self.draws.push(Draw {
                ordinal,
                commands: commands.clone(),
                origin: ZERO,
                rgba,
                stroke: None,
                error: ERROR,
            });
        }
        if let Some((rgba, stroke)) = paint.line {
            self.draws.push(Draw {
                ordinal,
                commands,
                origin: ZERO,
                rgba,
                stroke: Some(stroke),
                error: ERROR,
            });
        }
    }
    fn rect(&mut self, ordinal: u32, r: Rect, paint: style::Paint) -> Result<(), SourcePageError> {
        self.path(
            ordinal,
            vec![
                C::Move {
                    to: point(r.x, r.y)?,
                },
                C::Line {
                    to: point(r.x + r.w, r.y)?,
                },
                C::Line {
                    to: point(r.x + r.w, r.y + r.h)?,
                },
                C::Line {
                    to: point(r.x, r.y + r.h)?,
                },
                C::Close,
            ],
            paint,
        );
        Ok(())
    }
    fn line(
        &mut self,
        ordinal: u32,
        from: (f64, f64),
        to: (f64, f64),
        mut paint: style::Paint,
    ) -> Result<(), SourcePageError> {
        paint.fill = None;
        self.path(
            ordinal,
            vec![
                C::Move {
                    to: point(from.0, from.1)?,
                },
                C::Line {
                    to: point(to.0, to.1)?,
                },
            ],
            paint,
        );
        Ok(())
    }
    fn label(&mut self, ordinal: u32, value: String, style: style::TextStyle, r: Rect, align: f64) {
        self.labels.push(Label {
            ordinal,
            text: value,
            style,
            x: r.x,
            y: r.y + r.h / 2.0,
            width: r.w,
            align,
        });
    }
}
fn estimated_width(text: &str, size: f64) -> f64 {
    text.chars()
        .map(|c| if c.is_ascii() { 0.65 } else { 1.05 })
        .sum::<f64>()
        * size
}
pub(super) fn prepare(
    source: &SourceChartPaints,
    area_ordinal: u32,
    size: mo_presentation_model::Size,
    tolerance: Fixed,
    check: &dyn Fn() -> bool,
) -> Result<Layout, SourcePageError> {
    let chart = &source.chart;
    let part = &chart.part;
    if chart.plots.len() != 1 || !chart.extension_ordinals.is_empty() {
        return Err(invalid(
            part,
            0,
            "combined or extended charts require native layout",
        ));
    }
    let plot = &chart.plots[0];
    if !["barChart", "lineChart", "pieChart", "doughnutChart"].contains(&plot.native_kind.as_str())
    {
        return Err(invalid(
            part,
            plot.source_ordinal,
            format!("unsupported page chart {}", plot.native_kind),
        ));
    }
    audit(
        &plot.layout,
        &[
            P::BarDirection,
            P::Grouping,
            P::VaryColors,
            P::GapWidth,
            P::Overlap,
            P::FirstSliceAngle,
            P::HoleSize,
            P::Smooth,
        ],
        &[M::DataLabels],
        part,
    )?;
    if on(&plot.layout, P::Smooth) {
        return Err(invalid(
            part,
            plot.source_ordinal,
            "smooth line charts require spline layout",
        ));
    }
    if plot.native_kind == "lineChart" && plot.series.iter().any(|s| !s.point_overrides.is_empty())
    {
        return Err(invalid(
            part,
            plot.source_ordinal,
            "line point overrides require marker layout",
        ));
    }
    let mut series = Vec::new();
    for s in &plot.series {
        cancel(check)?;
        audit(
            &s.layout,
            &[P::Smooth, P::InvertIfNegative],
            &[M::ShapeProperties, M::Marker],
            part,
        )?;
        if on(&s.layout, P::Smooth) || on(&s.layout, P::InvertIfNegative) {
            return Err(invalid(
                part,
                s.source_ordinal,
                "series interpolation/negative paint requires mapping",
            ));
        }
        for p in &s.point_overrides {
            audit(&p.layout, &[], &[M::ShapeProperties], part)?;
        }
        let categories = values(s, ChartChannelRole::Categories, part)?
            .into_iter()
            .map(|v| v.unwrap_or_default())
            .collect::<Vec<_>>();
        let values = values(s, ChartChannelRole::Values, part)?;
        if categories.len() != values.len() {
            return Err(invalid(
                part,
                s.source_ordinal,
                "category/value count mismatch",
            ));
        }
        for v in values.iter().flatten() {
            numeric(v, part, s.source_ordinal)?;
        }
        let title = channel(s, ChartChannelRole::Title)
            .and_then(|c| {
                c.literal_text.clone().or_else(|| {
                    c.cache
                        .as_ref()
                        .and_then(|c| c.levels.first())
                        .and_then(|p| p.first())
                        .and_then(|p| p.value.clone())
                })
            })
            .unwrap_or_default();
        series.push(Series {
            source: s,
            title,
            categories,
            values,
        });
    }
    series.sort_by_key(|s| s.source.order);
    if series.is_empty()
        || series.len() > 32
        || series
            .windows(2)
            .any(|s| s[0].source.order == s[1].source.order)
    {
        return Err(invalid(
            part,
            plot.source_ordinal,
            "invalid chart series order/count",
        ));
    }
    for a in &chart.annotations.nodes {
        match a.kind {
            ChartAnnotationKind::DataLabels => {
                audit(
                    &a.declarations,
                    &[
                        P::Delete,
                        P::LabelPosition,
                        P::ShowLegendKey,
                        P::ShowValue,
                        P::ShowCategoryName,
                        P::ShowSeriesName,
                        P::ShowPercent,
                        P::ShowBubbleSize,
                        P::ShowLeaderLines,
                        P::Separator,
                    ],
                    &[M::TextProperties],
                    part,
                )?;
                if a.parent_ordinal != plot.source_ordinal
                    || (on(&a.declarations, P::ShowPercent)
                        && (on(&a.declarations, P::ShowValue)
                            || !matches!(plot.native_kind.as_str(), "pieChart" | "doughnutChart")))
                {
                    return Err(invalid(
                        part,
                        a.source_ordinal,
                        "series or composite percentage labels require layout",
                    ));
                }
                if [
                    P::ShowLegendKey,
                    P::ShowCategoryName,
                    P::ShowSeriesName,
                    P::ShowBubbleSize,
                    P::ShowLeaderLines,
                ]
                .iter()
                .any(|k| on(&a.declarations, *k))
                {
                    return Err(invalid(
                        part,
                        a.source_ordinal,
                        "composite chart labels require layout",
                    ));
                }
            }
            ChartAnnotationKind::Legend => {
                audit(
                    &a.declarations,
                    &[P::LegendPosition, P::Overlay],
                    &[M::TextProperties],
                    part,
                )?;
                if !matches!(value(&a.declarations, P::LegendPosition), None | Some("b"))
                    || on(&a.declarations, P::Overlay)
                {
                    return Err(invalid(
                        part,
                        a.source_ordinal,
                        "legend position/overlay requires layout",
                    ));
                }
            }
            _ => {
                return Err(invalid(
                    part,
                    a.source_ordinal,
                    "chart annotation requires native layout",
                ));
            }
        }
    }
    let outer = Rect {
        x: 0.0,
        y: 0.0,
        w: size.width.get() as f64,
        h: size.height.get() as f64,
    };
    if outer.w <= 0.0 || outer.h <= 0.0 {
        return Err(invalid(part, plot.source_ordinal, "empty chart extent"));
    }
    let mut out = Layout {
        coordinate_tolerance: tolerance,
        draws: vec![],
        labels: vec![],
    };
    out.rect(0, outer, paint(source, 0)?)?;
    let legend = chart
        .annotations
        .nodes
        .iter()
        .find(|a| a.kind == ChartAnnotationKind::Legend);
    let legend_style = legend.map(|a| text(source, a.source_ordinal)).transpose()?;
    let reserve = legend_style.as_ref().map_or(0.0, |s| s.size * 2.8);
    let plot_area = Rect {
        x: 0.0,
        y: 0.0,
        w: outer.w,
        h: outer.h - reserve,
    };
    let is_circular = matches!(plot.native_kind.as_str(), "pieChart" | "doughnutChart");
    if is_circular {
        circular::build(
            source,
            area_ordinal,
            plot,
            &series,
            plot_area,
            &mut out,
            check,
        )?;
    } else {
        cartesian::build(
            source,
            area_ordinal,
            plot,
            &series,
            plot_area,
            &mut out,
            check,
        )?;
    }
    if let (Some(legend), Some(style)) = (legend, legend_style) {
        let entries = if is_circular {
            series[0]
                .categories
                .iter()
                .enumerate()
                .map(|(i, t)| (t.clone(), point_paint(source, series[0].source, i as u32)))
                .collect::<Vec<_>>()
        } else {
            series
                .iter()
                .map(|s| (s.title.clone(), point_paint(source, s.source, 0)))
                .collect()
        };
        let widths: Vec<_> = entries
            .iter()
            .map(|(t, _)| estimated_width(t, style.size) + style.size * 2.8)
            .collect();
        let total: f64 = widths.iter().sum();
        if total > outer.w {
            return Err(invalid(
                part,
                legend.source_ordinal,
                "legend does not fit chart width",
            ));
        }
        let mut x = (outer.w - total) / 2.0;
        let y = outer.h - reserve / 2.0;
        for ((title, paint), w) in entries.into_iter().zip(widths) {
            let p = paint?;
            let rgba = p
                .fill
                .or(p.line.map(|p| p.0))
                .ok_or_else(|| invalid(part, legend.source_ordinal, "legend paint unresolved"))?;
            out.rect(
                legend.source_ordinal,
                Rect {
                    x,
                    y: y - style.size * 0.3,
                    w: style.size * 0.85,
                    h: style.size * 0.6,
                },
                style::Paint {
                    fill: Some(rgba),
                    line: None,
                },
            )?;
            out.label(
                legend.source_ordinal,
                title,
                style.clone(),
                Rect {
                    x: x + style.size * 1.2,
                    y: y - style.size,
                    w: w - style.size * 1.2,
                    h: style.size * 2.0,
                },
                0.0,
            );
            x += w;
        }
    }
    Ok(out)
}
fn labels<'a>(
    source: &'a SourceChartPaints,
    plot: &SourceChartPlot,
) -> Option<&'a SourceChartAnnotation> {
    source.chart.annotations.nodes.iter().find(|a| {
        a.kind == ChartAnnotationKind::DataLabels
            && a.parent_ordinal == plot.source_ordinal
            && !on(&a.declarations, P::Delete)
    })
}
