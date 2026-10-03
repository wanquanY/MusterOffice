//! Movement is resolved against retained line geometry, including native
//! paragraph translations. Sticky x lives in frame coordinates across paragraphs.
use super::*;
use mo_text::interaction::{Affinity, CaretMove, TextPosition};
use query::charge;

impl FrameInteraction<'_> {
    fn charge_navigation(
        &self,
        paragraph: usize,
        work: &mut usize,
    ) -> Result<(), SourceFrameError> {
        charge(
            work,
            self.maps[paragraph].navigation_work(),
            self.limits.max_query_work,
            "frame interaction query work",
        )
    }
    fn adjacent_paragraph(&self, p: usize, forward: bool) -> Option<usize> {
        if forward {
            p.checked_add(1).filter(|&p| p < self.maps.len())
        } else {
            p.checked_sub(1)
        }
    }
    fn end_position(&self, p: usize, end: bool) -> FrameTextPosition {
        FrameTextPosition {
            paragraph: p as u32,
            position: TextPosition {
                scalar_offset: if end {
                    self.maps[p]
                        .boundaries
                        .last()
                        .expect("map boundary")
                        .scalar_offset
                } else {
                    0
                },
                affinity: if end {
                    Affinity::Upstream
                } else {
                    Affinity::Downstream
                },
            },
        }
    }
    pub(super) fn move_caret(
        &self,
        position: FrameTextPosition,
        movement: CaretMove,
        preferred_x: Option<Fixed>,
        work: &mut usize,
        check: &dyn Fn() -> bool,
    ) -> Result<FrameTextQueryResult, SourceFrameError> {
        cancel(check)?;
        let p = position.paragraph as usize;
        self.charge_navigation(p, work)?;
        let current = self.caret(position)?;
        let mut exhausted = false;
        let mut sticky = None;
        let next = match movement {
            CaretMove::TextStart | CaretMove::TextEnd => {
                let end = movement == CaretMove::TextEnd;
                let target = if end { self.maps.len() - 1 } else { 0 };
                if target != p {
                    self.charge_navigation(target, work)?;
                }
                self.caret(self.end_position(target, end))?
            }
            CaretMove::Up | CaretMove::Down => {
                let forward = movement == CaretMove::Down;
                let x = preferred_x.unwrap_or(current.caret.edge.x);
                sticky = Some(x);
                let line = current.caret.line as usize;
                let adjacent_line = if forward {
                    line.checked_add(1)
                        .filter(|&l| l < self.maps[p].lines.len())
                } else {
                    line.checked_sub(1)
                };
                let target = adjacent_line.map(|l| (p, l)).or_else(|| {
                    self.adjacent_paragraph(p, forward).map(|next| {
                        (
                            next,
                            if forward {
                                0
                            } else {
                                self.maps[next].lines.len() - 1
                            },
                        )
                    })
                });
                if let Some((p2, line)) = target {
                    if p2 != p {
                        self.charge_navigation(p2, work)?;
                    }
                    let offset = self.frame.paragraphs[p2].line_offsets[line];
                    let local = self.maps[p2].nearest_in_line(
                        line as u32,
                        x.checked_sub(offset.x)?,
                        check,
                    )?;
                    self.placed_caret(p2 as u32, local)?
                } else {
                    exhausted = true;
                    current
                }
            }
            _ => {
                let result = self.maps[p].move_caret(position.position, movement, None, check)?;
                if !result.exhausted {
                    self.placed_caret(p as u32, result.caret)?
                } else {
                    let forward = match movement {
                        CaretMove::NextGrapheme => true,
                        CaretMove::PreviousGrapheme => false,
                        CaretMove::Right => self.maps[p].paragraph_level.is_multiple_of(2),
                        CaretMove::Left => self.maps[p].paragraph_level % 2 == 1,
                        _ => unreachable!("only directional moves exhaust"),
                    };
                    if let Some(p2) = self.adjacent_paragraph(p, forward) {
                        self.charge_navigation(p2, work)?;
                        if matches!(movement, CaretMove::Left | CaretMove::Right) {
                            let target = &self.maps[p2];
                            let line = if forward { 0 } else { target.lines.len() - 1 };
                            let right = forward == (target.paragraph_level % 2 == 1);
                            self.placed_caret(
                                p2 as u32,
                                target.line_edge(line as u32, right, check)?,
                            )?
                        } else {
                            self.caret(self.end_position(p2, !forward))?
                        }
                    } else {
                        exhausted = true;
                        current
                    }
                }
            }
        };
        cancel(check)?;
        Ok(FrameTextQueryResult::Moved {
            caret: next,
            preferred_x: sticky,
            exhausted,
        })
    }
}
