//! Shared whole-fragment visual ordering and high-precision pen bounds.
use super::*;
use crate::{fallback::FallbackResult, itemize::TextItem, lines::ShapedLine};
use mo_unicode::bidi::BidiLine;
pub(crate) struct FragmentOrder {
    pub visible: Vec<VisualItem>,
    pub removed: Vec<FragmentRef>,
}
#[derive(Clone, Copy)]
pub(crate) enum VisualItem {
    Fragment {
        source: FragmentRef,
        style: GeometryStyle,
    },
    Tab {
        item: u32,
    },
}
pub(crate) enum PlacedItem {
    Fragment {
        source: FragmentRef,
        style: GeometryStyle,
        origin: mo_geometry::Point,
    },
    Tab {
        item: u32,
        before: mo_geometry::Point,
        after: mo_geometry::Point,
    },
}
pub(crate) struct LinePlacement {
    pub items: Vec<PlacedItem>,
    pub advance: mo_geometry::Point,
    pub bounds: PenBounds,
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
                visible.push((
                    rank,
                    VisualItem::Fragment {
                        source: reference,
                        style,
                    },
                ));
            } else {
                removed.push(reference);
            }
        }
    }
    for i in line.item_start..line.item_end {
        cancelled(check)?;
        let item = &items[i as usize];
        if item.kind == TextItemKind::Tab {
            let rank = ranks[(item.start.scalar_offset - line.start.scalar_offset) as usize]
                .ok_or(TextError::Invalid("tab missing from visual order"))?;
            visible.push((rank, VisualItem::Tab { item: i }));
        }
    }
    visible.sort_by_key(|v| v.0);
    Ok(FragmentOrder {
        visible: visible.into_iter().map(|v| v.1).collect(),
        removed,
    })
}
pub(crate) fn place_line(
    fallback: &FallbackResult,
    order: &FragmentOrder,
    hanging: Option<std::ops::Range<u32>>,
    tabs: Option<&LeftTabStops>,
    line_start: u32,
    retain: bool,
    check: &dyn Fn() -> bool,
) -> Result<LinePlacement, TextError> {
    let mut x = Position::ZERO;
    let mut y = Position::ZERO;
    // Candidate fitting needs only bounds, not a second retained placement list.
    let mut items = Vec::with_capacity(if retain { order.visible.len() } else { 0 });
    let mut min = x;
    let mut max = x;
    let mut body: Option<(Position, Position)> = None;
    let mut before = false;
    let mut during = false;
    let mut after = false;
    let mut disjoint = false;
    for &item in &order.visible {
        cancelled(check)?;
        let (r, style) = match item {
            VisualItem::Fragment { source, style } => (source, style),
            VisualItem::Tab { item } => {
                let next = tabs
                    .ok_or(TextError::Invalid("tab policy required"))?
                    .next(x, line_start)?;
                if retain {
                    items.push(PlacedItem::Tab {
                        item,
                        before: mo_geometry::Point { x, y },
                        after: mo_geometry::Point { x: next, y },
                    });
                }
                min = min.min(next);
                max = max.max(next);
                if hanging.is_some() {
                    if during {
                        after = true;
                    } else {
                        before = true;
                    }
                    body = Some(body.map_or((x, next), |(lo, hi)| (lo.min(x), hi.max(next))));
                }
                x = next;
                continue;
            }
        };
        if retain {
            items.push(PlacedItem::Fragment {
                source: r,
                style,
                origin: mo_geometry::Point { x, y },
            });
        }
        let FontFragment::Selected { shaped, .. } =
            &fallback.items[r.fallback_item as usize].fragments[r.fragment as usize]
        else {
            unreachable!()
        };
        let mut cursor =
            FragmentPen::new(&shaped.runs[0].glyphs, style, shaped.position_units_per_em);
        while let Some(g) = cursor.advance()? {
            cancelled(check)?;
            if let Some(range) = &hanging {
                let excluded = range.contains(&shaped.runs[0].glyphs[g.glyph as usize].cluster);
                if excluded {
                    disjoint |= after;
                    during = true;
                } else {
                    if during {
                        after = true;
                    } else {
                        before = true;
                    }
                    for endpoint in [g.before.x, g.unspaced_end.x, g.after.x] {
                        let pen = x.checked_add(endpoint)?;
                        body = Some(body.map_or((pen, pen), |(lo, hi)| (lo.min(pen), hi.max(pen))));
                    }
                }
            }
            for endpoint in [g.unspaced_end.x, g.after.x] {
                let pen = x.checked_add(endpoint)?;
                min = min.min(pen);
                max = max.max(pen);
            }
        }
        x = x.checked_add(cursor.position().x)?;
        y = y.checked_add(cursor.position().y)?;
    }
    Ok(LinePlacement {
        items,
        advance: mo_geometry::Point { x, y },
        bounds: PenBounds {
            min,
            max,
            // Bidi may put logical-final punctuation at either visual edge. Never
            // subtract an interior cluster or a ligature containing preceding text.
            body: if during && !disjoint && !(before && after) {
                body
            } else {
                None
            },
        },
    })
}
pub(crate) struct PenBounds {
    pub min: Position,
    pub max: Position,
    pub body: Option<(Position, Position)>,
}
