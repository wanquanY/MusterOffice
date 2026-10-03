use super::*;
fn scaling(axis: &SourceChartAxis, part: &str) -> Result<SourceChartLayout, SourcePageError> {
    let l = SourceChartLayout {
        properties: axis
            .scaling
            .as_ref()
            .map(|s| s.properties.clone())
            .unwrap_or_default(),
        ..Default::default()
    };
    audit(&l, &[P::Minimum, P::Maximum, P::Orientation], &[], part)?;
    if !matches!(value(&l, P::Orientation), None | Some("minMax" | "maxMin")) {
        return Err(invalid(
            part,
            axis.source_ordinal,
            "invalid axis orientation",
        ));
    }
    Ok(l)
}
fn nice(range: f64) -> f64 {
    let raw = range / 5.0;
    let exponent = 10.0_f64.powf(raw.log10().floor());
    let n = raw / exponent;
    (if n <= 1.0 {
        1.0
    } else if n <= 2.0 {
        2.0
    } else if n <= 5.0 {
        5.0
    } else {
        10.0
    }) * exponent
}
fn lexical(v: f64) -> String {
    let s = format!("{v:.12}");
    let s = s.trim_end_matches('0').trim_end_matches('.');
    if s == "-0" { "0".into() } else { s.into() }
}
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
    let horizontal =
        plot.native_kind == "barChart" && value(&plot.layout, P::BarDirection) == Some("bar");
    if plot.native_kind == "barChart"
        && !matches!(value(&plot.layout, P::BarDirection), Some("bar" | "col"))
    {
        return Err(invalid(
            part,
            plot.source_ordinal,
            "bar direction unresolved",
        ));
    }
    if !matches!(
        value(&plot.layout, P::Grouping),
        None | Some("clustered" | "standard")
    ) || style::number(&plot.layout, P::Overlap, 0.0, part)? != 0.0
        || on(&plot.layout, P::VaryColors)
    {
        return Err(invalid(
            part,
            plot.source_ordinal,
            "stacked/overlapping or automatically varied series require layout",
        ));
    }
    let axes = &source.chart.axes;
    if axes.len() != 2
        || plot.axis_ids.len() != 2
        || axes.iter().any(|a| !plot.axis_ids.contains(&a.id))
    {
        return Err(invalid(
            part,
            plot.source_ordinal,
            "chart axes are not a single bound category/value pair",
        ));
    }
    let cat = axes
        .iter()
        .find(|a| a.kind == ChartAxisKind::Category)
        .ok_or_else(|| invalid(part, plot.source_ordinal, "category axis missing"))?;
    let val = axes
        .iter()
        .find(|a| a.kind == ChartAxisKind::Value)
        .ok_or_else(|| invalid(part, plot.source_ordinal, "value axis missing"))?;
    for a in axes {
        audit(
            &a.layout,
            &[
                P::Delete,
                P::AxisPosition,
                P::MajorTickMark,
                P::MinorTickMark,
                P::TickLabelPosition,
                P::CrossAxis,
                P::Crosses,
                P::CrossesAt,
                P::CrossBetween,
                P::MajorUnit,
                P::LabelOffset,
                P::Auto,
                P::NoMultiLevelLabels,
            ],
            &[M::ShapeProperties, M::TextProperties, M::MajorGridlines],
            part,
        )?;
        if [P::MajorTickMark, P::MinorTickMark]
            .iter()
            .any(|k| !matches!(value(&a.layout, *k), None | Some("none")))
            || !matches!(value(&a.layout, P::Crosses), None | Some("autoZero"))
            || value(&a.layout, P::CrossesAt).is_some()
            || !matches!(
                value(&a.layout, P::TickLabelPosition),
                None | Some("nextTo" | "none")
            )
        {
            return Err(invalid(
                part,
                a.source_ordinal,
                "axis ticks/crossing require layout",
            ));
        }
    }
    if value(&cat.layout, P::CrossAxis) != Some(val.id.to_string().as_str())
        || value(&val.layout, P::CrossAxis) != Some(cat.id.to_string().as_str())
    {
        return Err(invalid(
            part,
            plot.source_ordinal,
            "axis cross references mismatch",
        ));
    }
    let cs = scaling(cat, part)?;
    let vs = scaling(val, part)?;
    if value(&cat.layout, P::AxisPosition) != Some(if horizontal { "l" } else { "b" })
        || value(&val.layout, P::AxisPosition) != Some(if horizontal { "b" } else { "l" })
        || value(&cs, P::Minimum).is_some()
        || value(&cs, P::Maximum).is_some()
        || !matches!(value(&val.layout, P::CrossBetween), None | Some("between"))
        || style::number(&cat.layout, P::LabelOffset, 100.0, part)? != 100.0
    {
        return Err(invalid(
            part,
            plot.source_ordinal,
            "axis position/interval requires layout",
        ));
    }
    let cat_style = text(source, cat.source_ordinal)?;
    let val_style = text(source, val.source_ordinal)?;
    let n = series[0].categories.len();
    if n > 256 || series.iter().any(|s| s.categories != series[0].categories) {
        return Err(invalid(
            part,
            plot.source_ordinal,
            "unaligned category caches or excessive category count",
        ));
    }
    let nums: Vec<Vec<Option<f64>>> = series
        .iter()
        .map(|s| {
            s.values
                .iter()
                .map(|v| {
                    v.as_ref()
                        .map(|v| numeric(v, part, s.source.source_ordinal))
                        .transpose()
                })
                .collect()
        })
        .collect::<Result<_, _>>()?;
    let mut low = 0.0_f64;
    let mut high = 0.0_f64;
    for v in nums.iter().flatten().flatten() {
        low = low.min(*v);
        high = high.max(*v);
    }
    if low == high {
        high = low + 1.0;
    }
    let default_step = nice(high - low);
    let min = style::number(
        &vs,
        P::Minimum,
        (low / default_step).floor() * default_step,
        part,
    )?;
    let max = style::number(
        &vs,
        P::Maximum,
        (high / default_step).ceil() * default_step,
        part,
    )?;
    let step = style::number(&val.layout, P::MajorUnit, nice(max - min), part)?;
    if max <= min
        || step <= 0.0
        || (max - min) / step > 100.0
        || (max - min) < 1e-12
        || (max - min) > 1e12
    {
        return Err(invalid(
            part,
            val.source_ordinal,
            "invalid value axis interval",
        ));
    }
    if nums
        .iter()
        .flatten()
        .flatten()
        .any(|v| *v < min || *v > max)
    {
        return Err(invalid(
            part,
            val.source_ordinal,
            "values outside axis limits require plot clipping",
        ));
    }
    let fmt = val
        .number_format
        .as_ref()
        .and_then(|f| f.format_code.as_deref())
        .unwrap_or("General");
    let mut ticks = Vec::new();
    let count = ((max - min) / step + 1e-10).floor() as usize;
    for i in 0..=count {
        let v = min + step * i as f64;
        ticks.push((
            v,
            display(&lexical(v), fmt, part, val.source_ordinal, check)?,
        ));
    }
    let cat_width = series[0]
        .categories
        .iter()
        .map(|t| estimated_width(t, cat_style.size))
        .fold(0.0_f64, f64::max);
    let val_width = ticks
        .iter()
        .map(|(_, t)| estimated_width(t, val_style.size))
        .fold(0.0_f64, f64::max);
    let left = if horizontal {
        cat_width + cat_style.size * 0.8
    } else {
        val_width + val_style.size * 0.8
    };
    let top = cat_style.size * 1.15;
    let bottom = if horizontal {
        val_style.size * 1.9
    } else {
        cat_style.size * 1.9
    };
    let right = if horizontal {
        val_style.size * 2.5
    } else {
        cat_style.size * 1.5
    };
    let r = Rect {
        x: area.x + left,
        y: area.y + top,
        w: area.w - left - right,
        h: area.h - top - bottom,
    };
    if r.w <= cat_style.size * 2.0 || r.h <= cat_style.size * 2.0 {
        return Err(invalid(
            part,
            plot.source_ordinal,
            "chart axes leave no plot region",
        ));
    }
    out.rect(area_ordinal, r, paint(source, area_ordinal)?)?;
    // Subtraction of close, large decimal limits can magnify conversion error.
    // Admit only intervals whose conservative binary64 budget is below the
    // declared local layout bound; never certify a rounded-away data difference.
    let numeric_bound =
        f64::EPSILON * 32.0 * min.abs().max(max.abs()).max(1.0) / (max - min) * r.w.max(r.h);
    if numeric_bound > ERROR.raw() as f64 / 4294967296.0 {
        return Err(RasterError::Precision.into());
    }
    let reverse_val = value(&vs, P::Orientation) == Some("maxMin");
    let vpos = |v: f64| {
        let f = (v - min) / (max - min);
        let f = if reverse_val { 1.0 - f } else { f };
        if horizontal {
            r.x + f * r.w
        } else {
            r.y + (1.0 - f) * r.h
        }
    };
    let reverse_cat = value(&cs, P::Orientation) == Some("maxMin");
    let spacing = if horizontal { r.h } else { r.w } / n as f64;
    let cpos = |i: usize| {
        let f = (i as f64 + 0.5) / n as f64;
        let f = if reverse_cat { 1.0 - f } else { f };
        if horizontal {
            r.y + (1.0 - f) * r.h
        } else {
            r.x + f * r.w
        }
    };
    for (v, label) in ticks {
        cancel(check)?;
        let p = vpos(v);
        if let Some(grid) = val
            .layout
            .markup
            .iter()
            .find(|m| m.kind == M::MajorGridlines)
        {
            if horizontal {
                out.line(
                    grid.source_ordinal,
                    (p, r.y),
                    (p, r.y + r.h),
                    paint(source, grid.source_ordinal)?,
                )?;
            } else {
                out.line(
                    grid.source_ordinal,
                    (r.x, p),
                    (r.x + r.w, p),
                    paint(source, grid.source_ordinal)?,
                )?;
            }
        }
        if !on(&val.layout, P::Delete) && value(&val.layout, P::TickLabelPosition) != Some("none") {
            let lr = if horizontal {
                Rect {
                    x: p - val_width / 2.0 - val_style.size * 0.3,
                    y: r.y + r.h + val_style.size * 0.2,
                    w: val_width + val_style.size * 0.6,
                    h: val_style.size * 1.5,
                }
            } else {
                Rect {
                    x: area.x,
                    y: p - val_style.size,
                    w: left - val_style.size * 0.4,
                    h: val_style.size * 2.0,
                }
            };
            out.label(
                val.source_ordinal,
                label,
                val_style.clone(),
                lr,
                if horizontal { 0.5 } else { 1.0 },
            );
        }
    }
    if !on(&cat.layout, P::Delete) {
        if horizontal {
            out.line(
                cat.source_ordinal,
                (vpos(0.0_f64.clamp(min, max)), r.y),
                (vpos(0.0_f64.clamp(min, max)), r.y + r.h),
                paint(source, cat.source_ordinal)?,
            )?;
        } else {
            out.line(
                cat.source_ordinal,
                (r.x, vpos(0.0_f64.clamp(min, max))),
                (r.x + r.w, vpos(0.0_f64.clamp(min, max))),
                paint(source, cat.source_ordinal)?,
            )?;
        }
        if value(&cat.layout, P::TickLabelPosition) != Some("none") {
            for (i, label) in series[0].categories.iter().enumerate() {
                let p = cpos(i);
                let lr = if horizontal {
                    Rect {
                        x: area.x,
                        y: p - cat_style.size,
                        w: left - cat_style.size * 0.4,
                        h: cat_style.size * 2.0,
                    }
                } else {
                    Rect {
                        x: p - spacing / 2.0,
                        y: r.y + r.h + cat_style.size * 0.2,
                        w: spacing,
                        h: cat_style.size * 1.5,
                    }
                };
                out.label(
                    cat.source_ordinal,
                    label.clone(),
                    cat_style.clone(),
                    lr,
                    if horizontal { 1.0 } else { 0.5 },
                );
            }
        }
    }
    if !on(&val.layout, P::Delete) {
        if horizontal {
            out.line(
                val.source_ordinal,
                (r.x, r.y + r.h),
                (r.x + r.w, r.y + r.h),
                paint(source, val.source_ordinal)?,
            )?;
        } else {
            out.line(
                val.source_ordinal,
                (r.x, r.y),
                (r.x, r.y + r.h),
                paint(source, val.source_ordinal)?,
            )?;
        }
    }
    let label = labels(source, plot).filter(|a| on(&a.declarations, P::ShowValue));
    let label_style = label.map(|a| text(source, a.source_ordinal)).transpose()?;
    if let Some(a) = label
        && !matches!(
            value(&a.declarations, P::LabelPosition),
            None | Some("outEnd" | "t")
        )
    {
        return Err(invalid(
            part,
            a.source_ordinal,
            "data label placement requires layout",
        ));
    }
    let gap = style::number(&plot.layout, P::GapWidth, 150.0, part)?;
    if !(0.0..=500.0).contains(&gap) {
        return Err(invalid(part, plot.source_ordinal, "bar gap out of range"));
    }
    let width = spacing / (series.len() as f64 + gap / 100.0);
    let baseline = vpos(0.0_f64.clamp(min, max));
    let mut data_regions: Vec<Rect> = vec![];
    for (si, s) in series.iter().enumerate() {
        let mut line = Vec::new();
        let mut connected = false;
        for (i, v) in nums[si].iter().enumerate() {
            cancel(check)?;
            let Some(v) = v else {
                connected = false;
                continue;
            };
            let cp = cpos(i);
            let vp = vpos(*v);
            let offset = (si as f64 - (series.len() - 1) as f64 / 2.0) * width;
            if plot.native_kind == "barChart" {
                let rect = if horizontal {
                    Rect {
                        x: vp.min(baseline),
                        y: cp + offset - width / 2.0,
                        w: (vp - baseline).abs(),
                        h: width,
                    }
                } else {
                    Rect {
                        x: cp + offset - width / 2.0,
                        y: vp.min(baseline),
                        w: width,
                        h: (vp - baseline).abs(),
                    }
                };
                if *v != 0.0 {
                    let p = point_paint(source, s.source, i as u32)?;
                    if p.fill.is_none() {
                        return Err(invalid(
                            part,
                            s.source.source_ordinal,
                            "bar fill unresolved",
                        ));
                    }
                    out.rect(s.source.source_ordinal, rect, p)?;
                }
            } else {
                let p = point(cp, vp)?;
                line.push(if connected {
                    C::Line { to: p }
                } else {
                    C::Move { to: p }
                });
                connected = true;
            }
            if let (Some(a), Some(style)) = (label, &label_style) {
                let code = a
                    .number_format
                    .as_ref()
                    .and_then(|f| f.format_code.as_deref())
                    .or_else(|| {
                        channel(s.source, ChartChannelRole::Values)
                            .and_then(|c| c.cache.as_ref())
                            .and_then(|c| c.format_code.as_deref())
                    })
                    .unwrap_or("General");
                let d = display(
                    s.values[i].as_ref().expect("numeric point"),
                    code,
                    part,
                    a.source_ordinal,
                    check,
                )?;
                let label_w = estimated_width(&d, style.size) + style.size * 0.3;
                let mut at = if horizontal {
                    Rect {
                        x: if *v >= 0.0 {
                            vp + style.size * 0.3
                        } else {
                            vp - label_w - style.size * 0.3
                        },
                        y: cp + offset - style.size,
                        w: label_w,
                        h: style.size * 2.0,
                    }
                } else {
                    Rect {
                        x: cp
                            + if plot.native_kind == "barChart" {
                                offset
                            } else {
                                0.0
                            }
                            - label_w / 2.0,
                        y: vp - if *v >= 0.0 { style.size * 1.85 } else { 0.0 },
                        w: label_w,
                        h: style.size * 1.5,
                    }
                };
                if plot.native_kind == "lineChart" {
                    // Default labels prefer above; resolve local collisions below the
                    // data point. Explicit top placement remains above, stacked.
                    let automatic = value(&a.declarations, P::LabelPosition).is_none();
                    at = place_label(at, vp, style.size, automatic, &data_regions);
                    data_regions.push(at);
                }
                out.label(a.source_ordinal, d, style.clone(), at, 0.5);
            }
        }
        if plot.native_kind == "lineChart" {
            let mut p = point_paint(source, s.source, 0)?;
            p.fill = None;
            if p.line.is_none() {
                return Err(invalid(
                    part,
                    s.source.source_ordinal,
                    "line series paint unresolved",
                ));
            }
            out.path(s.source.source_ordinal, line, p);
        }
    }
    Ok(())
}

fn place_label(mut r: Rect, value_y: f64, size: f64, automatic: bool, occupied: &[Rect]) -> Rect {
    let intersects =
        |a: Rect, b: Rect| a.x < b.x + b.w && a.x + a.w > b.x && a.y < b.y + b.h && a.y + a.h > b.y;
    if automatic && occupied.iter().any(|p| intersects(r, *p)) {
        r.y = value_y + size * 0.2;
    }
    for _ in 0..occupied.len() {
        if !occupied.iter().any(|p| intersects(r, *p)) {
            break;
        }
        r.y -= r.h + size * 0.2;
    }
    r
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn automatic_labels_separate_coincident_series() {
        let first = Rect {
            x: 10.0,
            y: 20.0,
            w: 30.0,
            h: 15.0,
        };
        let second = place_label(first, 38.5, 10.0, true, &[first]);
        assert!(second.y >= first.y + first.h);
        let explicit = place_label(first, 38.5, 10.0, false, &[first]);
        assert!(explicit.y + explicit.h <= first.y);
    }
    #[test]
    fn ticks_are_finite_and_do_not_expose_binary_decimal_noise() {
        assert_eq!(nice(1.0), 0.2);
        assert_eq!(nice(50.0), 10.0);
        assert_eq!(lexical(0.2 * 3.0), "0.6");
        assert_eq!(lexical(-0.0), "0");
    }
}
