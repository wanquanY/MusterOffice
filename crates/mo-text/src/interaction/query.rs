use super::*;
use mo_geometry::Rect;
const MAX_QUERY_WORK: usize = 1_048_576;
fn boundary_at(boundaries: &[TextBoundary], offset: u32) -> Result<usize, TextError> {
    boundaries
        .binary_search_by_key(&offset, |b| b.scalar_offset)
        .map_err(|_| TextError::Invalid("interaction offset must be a grapheme boundary"))
}
pub(super) fn preflight(
    queries: &[TextQuery],
    boundaries: &[TextBoundary],
    check: &dyn Fn() -> bool,
) -> Result<(), TextError> {
    if queries.len() > 64 {
        return Err(TextError::Limit("paragraph interaction queries"));
    }
    for query in queries {
        cancelled(check)?;
        match query {
            TextQuery::Caret { position } => {
                boundary_at(boundaries, position.scalar_offset)?;
            }
            TextQuery::Selection { anchor, focus } => {
                boundary_at(boundaries, anchor.scalar_offset)?;
                boundary_at(boundaries, focus.scalar_offset)?;
            }
            TextQuery::Hit { .. } => {}
        }
    }
    Ok(())
}
fn distance(value: Fixed, lo: Fixed, hi: Fixed) -> u128 {
    if value < lo {
        value.raw().abs_diff(lo.raw())
    } else if value > hi {
        value.raw().abs_diff(hi.raw())
    } else {
        0
    }
}
fn resolved(cell: &InteractionCell, leading: bool) -> ResolvedCaret {
    ResolvedCaret {
        position: TextPosition {
            scalar_offset: if leading {
                cell.start.scalar_offset
            } else {
                cell.end.scalar_offset
            },
            affinity: if leading {
                Affinity::Downstream
            } else {
                Affinity::Upstream
            },
        },
        boundary: if leading {
            cell.start.clone()
        } else {
            cell.end.clone()
        },
        line: cell.line,
        edge: if leading { cell.leading } else { cell.trailing },
    }
}
fn empty(line: &InteractionLine, index: usize) -> ResolvedCaret {
    ResolvedCaret {
        position: TextPosition {
            scalar_offset: line.start.scalar_offset,
            affinity: Affinity::Downstream,
        },
        boundary: line.start.clone(),
        line: index as u32,
        edge: line.empty_caret,
    }
}
fn bounds(cell: &InteractionCell) -> Rect {
    Rect {
        min: Point {
            x: cell.leading.x.min(cell.trailing.x),
            y: cell.leading.top.min(cell.trailing.top),
        },
        max: Point {
            x: cell.leading.x.max(cell.trailing.x),
            y: cell.leading.bottom.max(cell.trailing.bottom),
        },
    }
}
fn charge(work: &mut usize, count: usize) -> Result<(), TextError> {
    *work = work
        .checked_add(count)
        .ok_or(TextError::Limit("interaction query work"))?;
    if *work > MAX_QUERY_WORK {
        return Err(TextError::Limit("interaction query work"));
    }
    Ok(())
}
impl InteractionMap {
    /// Logical affinity disambiguates both soft-wrap and bidi split carets.
    pub fn caret(&self, position: TextPosition) -> Result<ResolvedCaret, TextError> {
        let index = boundary_at(&self.boundaries, position.scalar_offset)?;
        let next = self.cells.get(index);
        let previous = index.checked_sub(1).and_then(|i| self.cells.get(i));
        let empty_line = self.lines.iter().enumerate().find(|(_, line)| {
            line.start.scalar_offset == position.scalar_offset && line.start == line.end
        });
        match position.affinity {
            Affinity::Downstream => {
                if let Some((i, line)) = empty_line {
                    return Ok(empty(line, i));
                }
                if let Some(c) = next {
                    return Ok(resolved(c, true));
                }
                if let Some(c) = previous {
                    return Ok(resolved(c, false));
                }
            }
            Affinity::Upstream => {
                if let Some(c) = previous {
                    return Ok(resolved(c, false));
                }
                if let Some(c) = next {
                    return Ok(resolved(c, true));
                }
                if let Some((i, line)) = empty_line {
                    return Ok(empty(line, i));
                }
            }
        }
        Err(TextError::Invalid("empty interaction map"))
    }
    fn hit(
        &self,
        point: Point,
        work: &mut usize,
        check: &dyn Fn() -> bool,
    ) -> Result<TextQueryResult, TextError> {
        charge(work, self.lines.len())?;
        let mut best_line = None;
        for (i, line) in self.lines.iter().enumerate() {
            cancelled(check)?;
            let rank = (distance(point.y, line.top, line.bottom), i);
            if best_line.is_none_or(|(current, _)| rank < current) {
                best_line = Some((rank, i));
            }
        }
        let (_, index) = best_line.ok_or(TextError::Invalid("empty interaction lines"))?;
        let line = &self.lines[index];
        charge(work, line.cells.len() * 2)?;
        let mut best = None;
        let mut inside = false;
        for &i in &line.cells {
            cancelled(check)?;
            let cell = &self.cells[i as usize];
            let b = bounds(cell);
            let contains = cell.placement != CaretPlacement::Invisible
                && point.x >= b.min.x
                && point.x <= b.max.x
                && point.y >= b.min.y
                && point.y <= b.max.y;
            inside |= contains;
            for leading in [true, false] {
                let c = resolved(cell, leading);
                // The containing advance cell owns its interior, including at
                // a bidi split. Shared borders prefer the base direction, then
                // downstream. Selection still retains both affinities.
                let rank = (
                    u8::from(!contains),
                    point.x.raw().abs_diff(c.edge.x.raw()),
                    distance(point.y, c.edge.top, c.edge.bottom),
                    u8::from(cell.placement == CaretPlacement::Invisible),
                    u8::from(cell.level % 2 != self.paragraph_level % 2),
                    u8::from(!leading),
                    c.position.scalar_offset,
                );
                if best.as_ref().is_none_or(|(current, _)| rank < *current) {
                    best = Some((rank, c));
                }
            }
        }
        Ok(TextQueryResult::Hit {
            caret: best.map(|(_, c)| c).unwrap_or_else(|| empty(line, index)),
            inside,
        })
    }
    pub fn query(
        &self,
        queries: &[TextQuery],
        check: &dyn Fn() -> bool,
    ) -> Result<Vec<TextQueryResult>, TextError> {
        preflight(queries, &self.boundaries, check)?;
        let mut work = 0;
        let mut results = Vec::with_capacity(queries.len());
        let mut output_fragments = 0usize;
        for query in queries {
            cancelled(check)?;
            results.push(match *query {
                TextQuery::Caret { position } => {
                    charge(&mut work, self.lines.len())?;
                    TextQueryResult::Caret {
                        caret: self.caret(position)?,
                    }
                }
                TextQuery::Hit { point } => self.hit(point, &mut work, check)?,
                TextQuery::Selection { anchor, focus } => {
                    charge(&mut work, self.lines.len() * 2)?;
                    let a = boundary_at(&self.boundaries, anchor.scalar_offset)?;
                    let b = boundary_at(&self.boundaries, focus.scalar_offset)?;
                    let (lo, hi) = (a.min(b), a.max(b));
                    charge(&mut work, hi - lo)?;
                    output_fragments += hi - lo;
                    if output_fragments > 65536 {
                        return Err(TextError::Limit("interaction selection fragments"));
                    }
                    let mut fragments = Vec::with_capacity(hi - lo);
                    for cell in &self.cells[lo..hi] {
                        cancelled(check)?;
                        fragments.push(SelectionFragment {
                            line: cell.line,
                            start: cell.start.clone(),
                            end: cell.end.clone(),
                            kind: cell.kind,
                            bounds: bounds(cell),
                        });
                    }
                    TextQueryResult::Selection {
                        anchor: self.caret(anchor)?,
                        focus: self.caret(focus)?,
                        fragments,
                    }
                }
            });
        }
        cancelled(check)?;
        Ok(results)
    }
}
