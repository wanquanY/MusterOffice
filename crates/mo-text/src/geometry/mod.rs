//! Horizontal glyph placement for explicit lines and explicit metric/spacing
//! policy. Core geometry only: no implicit fonts, line breaking or rendering.
mod metrics;
pub(crate) mod number;
pub(crate) mod order;
mod pen;
mod spacing;
#[cfg(test)]
pub(crate) mod test_support;
#[cfg(test)]
mod tests;
mod types;
use crate::{
    fallback::FontFragment,
    itemize::{ItemizationNoticeKind, TextItemKind},
    *,
};
use number::Position;
pub use pen::{FragmentPen, GlyphPen};
pub use spacing::line_style_maximum;
pub use types::*;
pub fn layout_lines(
    q: &LineGeometryRequest,
    bundle: &[u8],
    backend: &mut dyn backend::TextBackend,
    check: &dyn Fn() -> bool,
) -> Result<LineGeometryResult, TextError> {
    validate(q, check)?;
    lines::shape_lines_with(
        &q.shaping,
        bundle,
        backend,
        check,
        &|a, b| styles_equal(&q.styles, &q.spacing, a, b),
        |shaping, fonts, bindings, backend| evaluate(q, shaping, fonts, bindings, backend, check),
    )
}
pub(crate) fn styles_equal(
    styles: &[GeometryStyle],
    spacing: &LineSpacing,
    a: usize,
    b: usize,
) -> bool {
    styles[a] == styles[b]
        && match spacing {
            LineSpacing::StyleMaximum { heights } => heights[a] == heights[b],
            _ => true,
        }
}
pub(crate) fn validate(q: &LineGeometryRequest, check: &dyn Fn() -> bool) -> Result<(), TextError> {
    cancelled(check)?;
    if q.styles.len() != q.shaping.paragraph.styles.len()
        || q.strut_style as usize >= q.styles.len()
    {
        return Err(TextError::Invalid(
            "one geometry style per shaping style and explicit strut",
        ));
    }
    if q.styles.iter().any(|s| s.font_size.get() <= 0) {
        return Err(TextError::Invalid("positive font size"));
    }
    if matches!(q.spacing,LineSpacing::Exact{height}|LineSpacing::AtLeast{height} if height.get()<=0)
    {
        return Err(TextError::Invalid("positive line spacing"));
    }
    if let LineSpacing::StyleMaximum { heights } = &q.spacing
        && (heights.len() != q.styles.len() || heights.iter().any(|h| *h < Position::ZERO))
    {
        return Err(TextError::Invalid("one nonnegative line height per style"));
    }
    Ok(())
}
pub(crate) fn evaluate(
    q: &LineGeometryRequest,
    shaping: lines::LineShapeResult,
    fonts: &[mo_font::VerifiedFont<'_>],
    bindings: &[usize],
    backend: &mut dyn backend::TextBackend,
    check: &dyn Fn() -> bool,
) -> Result<LineGeometryResult, TextError> {
    evaluate_impl(q, shaping, fonts, bindings, backend, check, false).map(|(result, _)| result)
}
pub(crate) struct PreciseGlyph {
    pub line: u32,
    pub source: FragmentRef,
    pub glyph: u32,
    pub origin: mo_geometry::Point,
}
#[derive(Default)]
pub(crate) struct PrecisePlacements {
    pub glyphs: Vec<PreciseGlyph>,
    pub layout: Option<PreciseGeometryLayout>,
}
pub(crate) fn evaluate_precise(
    q: &LineGeometryRequest,
    shaping: lines::LineShapeResult,
    fonts: &[mo_font::VerifiedFont<'_>],
    bindings: &[usize],
    backend: &mut dyn backend::TextBackend,
    check: &dyn Fn() -> bool,
) -> Result<(LineGeometryResult, PrecisePlacements), TextError> {
    evaluate_impl(q, shaping, fonts, bindings, backend, check, true)
}
fn evaluate_impl(
    q: &LineGeometryRequest,
    shaping: lines::LineShapeResult,
    fonts: &[mo_font::VerifiedFont<'_>],
    bindings: &[usize],
    backend: &mut dyn backend::TextBackend,
    check: &dyn Fn() -> bool,
    retain: bool,
) -> Result<(LineGeometryResult, PrecisePlacements), TextError> {
    let mut issues = Vec::new();
    for item in &shaping.items {
        if item.kind == TextItemKind::Tab {
            issues.push(GeometryIssue::Tab {
                start: item.start.scalar_offset,
                end: item.end.scalar_offset,
            });
        }
    }
    for notice in &shaping.notices {
        if notice.kind == ItemizationNoticeKind::MixedLevelCluster {
            issues.push(GeometryIssue::MixedLevelCluster {
                start: notice.start,
                end: notice.end,
            });
        }
    }
    let measurements = metrics::measure(q, &shaping, fonts, bindings, backend, &mut issues, check)?;
    let (layout, precise) = if issues.is_empty() {
        place(q, &shaping, &measurements, &mut issues, check, retain)?
    } else {
        (None, PrecisePlacements::default())
    };
    cancelled(check)?;
    Ok((
        LineGeometryResult {
            profile: "horizontal-hb-metrics-strut-q32-emu-v1-draft".into(),
            shaping,
            metric_instances: measurements.instances,
            layout,
            issues,
        },
        precise,
    ))
}

fn extents(
    m: &MetricInstance,
    s: GeometryStyle,
) -> Result<(Position, Position, Position), TextError> {
    let scale = |i: usize| {
        Position::scale(
            i64::from(m.measured.values[i].position.expect("metrics validated")),
            s.font_size,
            m.position_units_per_em,
        )
    };
    let shift = s.baseline_shift.position();
    Ok((
        scale(0)?.checked_add(shift)?,
        Position::ZERO.checked_sub(scale(1)?)?.checked_sub(shift)?,
        scale(2)?,
    ))
}
fn place(
    q: &LineGeometryRequest,
    shaped: &lines::LineShapeResult,
    measured: &metrics::Measurements,
    issues: &mut Vec<GeometryIssue>,
    check: &dyn Fn() -> bool,
    retain: bool,
) -> Result<(Option<GeometryLayout>, PrecisePlacements), TextError> {
    let mut precise = PrecisePlacements::default();
    let mut precise_lines = Vec::new();
    let mut y = Position::ZERO;
    let mut lines = Vec::new();
    let strut = extents(
        &measured.instances[measured.strut],
        q.styles[q.strut_style as usize],
    )?;
    for (line_index, line) in shaped.lines.iter().enumerate() {
        cancelled(check)?;
        let (mut ascent, mut descent, mut gap) = strut;
        let ordered = order::order(
            &shaped.shaped_item_indices,
            &shaped.items,
            &shaped.fallback,
            line,
            &shaped.bidi.lines[line_index],
            &q.styles,
            check,
        )?;
        for i in line.fallback_start..line.fallback_end {
            let style = q.styles
                [shaped.items[shaped.shaped_item_indices[i as usize] as usize].style as usize];
            for (j, _) in shaped.fallback.items[i as usize]
                .fragments
                .iter()
                .enumerate()
            {
                cancelled(check)?;
                let m = &measured.instances[measured.fragments[i as usize][j].unwrap()];
                let (a, d, g) = extents(m, style)?;
                ascent = ascent.max(a);
                descent = descent.max(d);
                gap = gap.max(g);
            }
        }
        let content = ascent.checked_add(descent)?;
        let natural = content.checked_add(gap)?;
        let height = match &q.spacing {
            LineSpacing::Natural => natural,
            LineSpacing::Exact { height } => Position::emu(*height),
            LineSpacing::AtLeast { height } => natural.max(Position::emu(*height)),
            LineSpacing::StyleMaximum { heights } => {
                line_style_maximum(shaped, line_index as u32, heights, q.strut_style)?
            }
        };
        if height < Position::ZERO
            || (height == Position::ZERO && !matches!(q.spacing, LineSpacing::StyleMaximum { .. }))
        {
            issues.push(GeometryIssue::NonPositiveNaturalHeight {
                line: line_index as u32,
            });
            return Ok((None, PrecisePlacements::default()));
        }
        let baseline = y
            .checked_add(ascent)?
            .checked_add(height.checked_sub(content)?.half()?)?;
        let bottom = y.checked_add(height)?;
        let mut x = Position::ZERO;
        let mut pen_y = Position::ZERO;
        let mut min = Position::ZERO;
        let mut max = Position::ZERO;
        let mut glyphs = Vec::new();
        for &(reference, style) in &ordered.visible {
            cancelled(check)?;
            let FontFragment::Selected { shaped, .. } = &shaped.fallback.items
                [reference.fallback_item as usize]
                .fragments[reference.fragment as usize]
            else {
                unreachable!()
            };
            let mut pen =
                FragmentPen::new(&shaped.runs[0].glyphs, style, shaped.position_units_per_em);
            while let Some(g) = pen.advance()? {
                cancelled(check)?;
                // Prefix sums stay in exact design units until each queried
                // origin; never add rounded per-glyph EMU advances.
                let gx = x.checked_add(g.origin.x)?;
                let gy = baseline
                    .checked_sub(style.baseline_shift.position())?
                    .checked_sub(pen_y)?
                    .checked_sub(g.origin.y)?;
                if retain {
                    precise.glyphs.push(PreciseGlyph {
                        line: line_index as u32,
                        source: reference,
                        glyph: g.glyph,
                        origin: mo_geometry::Point { x: gx, y: gy },
                    });
                }
                glyphs.push(PositionedGlyph {
                    source: reference,
                    glyph: g.glyph,
                    x: gx.wire()?,
                    y: gy.wire()?,
                });
                for endpoint in [g.unspaced_end.x, g.after.x] {
                    let endpoint = x.checked_add(endpoint)?;
                    min = min.min(endpoint);
                    max = max.max(endpoint);
                }
            }
            x = x.checked_add(pen.position().x)?;
            pen_y = pen_y.checked_add(pen.position().y)?;
        }
        if retain {
            precise_lines.push(PreciseLineGeometry {
                top: y,
                baseline,
                bottom,
                height,
                advance: x,
                advance_y: Position::ZERO.checked_sub(pen_y)?,
                pen_min: min,
                pen_max: max,
            });
        }
        lines.push(GeometryLine {
            top: y.wire()?,
            baseline: baseline.wire()?,
            bottom: bottom.wire()?,
            height: height.wire()?,
            advance: x.wire()?,
            advance_y: Position::ZERO.checked_sub(pen_y)?.wire()?,
            pen_min: min.wire()?,
            pen_max: max.wire()?,
            visual_fragments: ordered.visible.into_iter().map(|v| v.0).collect(),
            removed_by_x9: ordered.removed,
            glyphs,
        });
        y = bottom;
    }
    if retain {
        precise.layout = Some(PreciseGeometryLayout {
            height: y,
            lines: precise_lines,
        });
    }
    Ok((
        Some(GeometryLayout {
            height: y.wire()?,
            lines,
        }),
        precise,
    ))
}
