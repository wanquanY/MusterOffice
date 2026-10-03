//! Visual traversal follows the renderer's L1/L2 grapheme order, including
//! coincident bidi edges. It never sorts distorted or overlapping glyph x's.
use super::*;
use query::{empty, resolved};

impl InteractionMap {
    /// Conservative bounded work charged by container batch queries.
    pub fn navigation_work(&self) -> usize {
        self.cells.len() * 2 + self.lines.len() * 4
    }
    fn line(&self, line: u32) -> Result<&InteractionLine, TextError> {
        self.lines
            .get(line as usize)
            .ok_or(TextError::Invalid("navigation line index"))
    }
    fn visual_edge(&self, cell: u32, right: bool) -> ResolvedCaret {
        let cell = &self.cells[cell as usize];
        resolved(cell, right == (cell.level % 2 == 1))
    }
    /// Physical inline edge, before a frame/container translation.
    pub fn line_edge(
        &self,
        line: u32,
        right: bool,
        check: &dyn Fn() -> bool,
    ) -> Result<ResolvedCaret, TextError> {
        cancelled(check)?;
        let l = self.line(line)?;
        let cell = if right {
            l.visual_cells.last()
        } else {
            l.visual_cells.first()
        };
        Ok(cell
            .map(|&c| self.visual_edge(c, right))
            .unwrap_or_else(|| empty(l, line as usize)))
    }
    /// Vertical movement ranks only actual caret x; glyph height/baseline must
    /// not bias it toward a taller run. Ties prefer paragraph direction then downstream.
    pub fn nearest_in_line(
        &self,
        line: u32,
        x: Fixed,
        check: &dyn Fn() -> bool,
    ) -> Result<ResolvedCaret, TextError> {
        let l = self.line(line)?;
        let mut best = None;
        for &at in &l.visual_cells {
            cancelled(check)?;
            let cell = &self.cells[at as usize];
            for leading in [true, false] {
                let c = resolved(cell, leading);
                let rank = (
                    x.raw().abs_diff(c.edge.x.raw()),
                    u8::from(cell.level % 2 != self.paragraph_level % 2),
                    u8::from(!leading),
                    c.position.scalar_offset,
                );
                if best.as_ref().is_none_or(|(r, _)| rank < *r) {
                    best = Some((rank, c));
                }
            }
        }
        cancelled(check)?;
        Ok(best
            .map(|(_, c)| c)
            .unwrap_or_else(|| empty(l, line as usize)))
    }
    fn slot(&self, caret: &ResolvedCaret, check: &dyn Fn() -> bool) -> Result<usize, TextError> {
        let line = self.line(caret.line)?;
        let mut nearest = None;
        for (rank, &at) in line.visual_cells.iter().enumerate() {
            cancelled(check)?;
            for right in [false, true] {
                let edge = self.visual_edge(at, right);
                let slot = rank + usize::from(right);
                if edge.position == caret.position {
                    return Ok(slot);
                }
                // X9/zero-glyph positions have no slot of their own. Anchor them
                // to the nearest rendered edge, with a stable base-direction tie.
                let key = (
                    edge.edge.x.raw().abs_diff(caret.edge.x.raw()),
                    u8::from(self.cells[at as usize].level % 2 != self.paragraph_level % 2),
                    slot,
                );
                if nearest.is_none_or(|(old, _)| key < old) {
                    nearest = Some((key, slot));
                }
            }
        }
        Ok(nearest.map_or(0, |(_, slot)| slot))
    }
    pub fn move_caret(
        &self,
        position: TextPosition,
        movement: CaretMove,
        preferred_x: Option<Fixed>,
        check: &dyn Fn() -> bool,
    ) -> Result<CaretNavigation, TextError> {
        cancelled(check)?;
        let caret = self.caret(position)?;
        let vertical = matches!(movement, CaretMove::Up | CaretMove::Down);
        if preferred_x.is_some() && !vertical {
            return Err(TextError::Invalid("sticky x requires vertical movement"));
        }
        let x = preferred_x.unwrap_or(caret.edge.x);
        let mut exhausted = false;
        let next = match movement {
            CaretMove::TextStart | CaretMove::TextEnd => self.caret(TextPosition {
                scalar_offset: if movement == CaretMove::TextStart {
                    0
                } else {
                    self.boundaries.last().unwrap().scalar_offset
                },
                affinity: if movement == CaretMove::TextStart {
                    Affinity::Downstream
                } else {
                    Affinity::Upstream
                },
            })?,
            CaretMove::PreviousGrapheme | CaretMove::NextGrapheme => {
                let at = self
                    .boundaries
                    .binary_search_by_key(&position.scalar_offset, |b| b.scalar_offset)
                    .unwrap();
                let next = if movement == CaretMove::PreviousGrapheme {
                    at.checked_sub(1)
                } else {
                    at.checked_add(1).filter(|&n| n < self.boundaries.len())
                };
                if let Some(at) = next {
                    self.caret(TextPosition {
                        scalar_offset: self.boundaries[at].scalar_offset,
                        affinity: if movement == CaretMove::PreviousGrapheme {
                            Affinity::Downstream
                        } else {
                            Affinity::Upstream
                        },
                    })?
                } else {
                    exhausted = true;
                    caret.clone()
                }
            }
            CaretMove::LineStart | CaretMove::LineEnd => self.line_edge(
                caret.line,
                (movement == CaretMove::LineEnd) != (self.paragraph_level % 2 == 1),
                check,
            )?,
            CaretMove::Up | CaretMove::Down => {
                let next = if movement == CaretMove::Up {
                    caret.line.checked_sub(1)
                } else {
                    caret
                        .line
                        .checked_add(1)
                        .filter(|&n| (n as usize) < self.lines.len())
                };
                if let Some(line) = next {
                    self.nearest_in_line(line, x, check)?
                } else {
                    exhausted = true;
                    caret.clone()
                }
            }
            CaretMove::Left | CaretMove::Right => {
                let right = movement == CaretMove::Right;
                let l = self.line(caret.line)?;
                let slot = self.slot(&caret, check)?;
                let cell = if right {
                    l.visual_cells.get(slot)
                } else {
                    slot.checked_sub(1).and_then(|s| l.visual_cells.get(s))
                };
                if let Some(&cell) = cell {
                    self.visual_edge(cell, right)
                } else {
                    let forward = right != (self.paragraph_level % 2 == 1);
                    let line = if forward {
                        caret
                            .line
                            .checked_add(1)
                            .filter(|&n| (n as usize) < self.lines.len())
                    } else {
                        caret.line.checked_sub(1)
                    };
                    if let Some(line) = line {
                        self.line_edge(line, !right, check)?
                    } else {
                        exhausted = true;
                        caret.clone()
                    }
                }
            }
        };
        cancelled(check)?;
        Ok(CaretNavigation {
            caret: next,
            preferred_x: vertical.then_some(x),
            exhausted,
        })
    }
}
