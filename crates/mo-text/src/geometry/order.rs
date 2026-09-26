//! Shared whole-fragment visual ordering and high-precision pen bounds.
use super::*;
use crate::{fallback::FallbackResult, itemize::TextItem, lines::ShapedLine};
use mo_unicode::bidi::BidiLine;
pub(crate) struct FragmentOrder {
    pub visible: Vec<(FragmentRef, GeometryStyle)>,
    pub removed: Vec<FragmentRef>,
}
pub(crate) fn order(
    indices: &[u32],
    items: &[TextItem],
    fallback: &FallbackResult,
    line: &ShapedLine,
    bidi: &BidiLine,
    styles: &[GeometryStyle],
    check: &dyn Fn() -> bool,
) -> Result<FragmentOrder, TextError> {
    let mut ranks = vec![None; (line.end.scalar_offset - line.start.scalar_offset) as usize];
    for (rank, &scalar) in bidi.visual_order.iter().enumerate() {
        ranks[(scalar - line.start.scalar_offset) as usize] = Some(rank);
    }
    let mut visible = Vec::new();
    let mut removed = Vec::new();
    for i in line.fallback_start..line.fallback_end {
        let style = styles[items[indices[i as usize] as usize].style as usize];
        for (j, f) in fallback.items[i as usize].fragments.iter().enumerate() {
            cancelled(check)?;
            let FontFragment::Selected { start, end, .. } = f else {
                return Err(TextError::Invalid("unresolved font before geometry"));
            };
            let reference = FragmentRef {
                fallback_item: i,
                fragment: j as u32,
            };
            let rank = ranks[(*start - line.start.scalar_offset) as usize
                ..(*end - line.start.scalar_offset) as usize]
                .iter()
                .flatten()
                .copied()
                .min();
            if let Some(rank) = rank {
                visible.push((rank, reference, style));
            } else {
                removed.push(reference);
            }
        }
    }
    visible.sort_by_key(|v| v.0);
    Ok(FragmentOrder {
        visible: visible.into_iter().map(|v| (v.1, v.2)).collect(),
        removed,
    })
}
pub(crate) fn pen_bounds(
    fallback: &FallbackResult,
    order: &FragmentOrder,
    check: &dyn Fn() -> bool,
) -> Result<(Position, Position), TextError> {
    let mut x = Position::ZERO;
    let mut min = x;
    let mut max = x;
    for &(r, style) in &order.visible {
        let FontFragment::Selected { shaped, .. } =
            &fallback.items[r.fallback_item as usize].fragments[r.fragment as usize]
        else {
            unreachable!()
        };
        let mut cursor =
            FragmentPen::new(&shaped.runs[0].glyphs, style, shaped.position_units_per_em);
        while let Some(g) = cursor.advance()? {
            cancelled(check)?;
            for endpoint in [g.unspaced_end.x, g.after.x] {
                let pen = x.checked_add(endpoint)?;
                min = min.min(pen);
                max = max.max(pen);
            }
        }
        x = x.checked_add(cursor.position().x)?;
    }
    Ok((min, max))
}
