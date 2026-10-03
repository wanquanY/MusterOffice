use super::*;
use mo_text::interaction::{
    Affinity, CaretEdge, ResolvedCaret, TextPosition, TextQuery, TextQueryResult,
};

fn invalid(label: &'static str) -> SourceFrameError {
    mo_text::TextError::Invalid(label).into()
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
fn charge(
    total: &mut usize,
    count: usize,
    max: usize,
    label: &'static str,
) -> Result<(), SourceFrameError> {
    *total = total
        .checked_add(count)
        .filter(|&n| n <= max)
        .ok_or(SourceFrameError::Limit(label))?;
    Ok(())
}
impl FrameInteraction<'_> {
    fn boundary(&self, p: FrameTextPosition) -> Result<usize, SourceFrameError> {
        let map = self
            .maps
            .get(p.paragraph as usize)
            .ok_or_else(|| invalid("frame paragraph index"))?;
        map.boundaries
            .binary_search_by_key(&p.position.scalar_offset, |b| b.scalar_offset)
            .map_err(|_| invalid("frame position must be a grapheme boundary"))
    }
    fn clip(&self, mut rect: Rect) -> Option<Rect> {
        if let Some(clip) = self.frame.clip {
            if clip.horizontal {
                rect.min.x = rect.min.x.max(clip.bounds.min.x);
                rect.max.x = rect.max.x.min(clip.bounds.max.x);
            }
            if clip.vertical {
                rect.min.y = rect.min.y.max(clip.bounds.min.y);
                rect.max.y = rect.max.y.min(clip.bounds.max.y);
            }
        }
        (rect.min.x <= rect.max.x && rect.min.y <= rect.max.y).then_some(rect)
    }
    fn placed_caret(
        &self,
        paragraph: u32,
        mut caret: ResolvedCaret,
    ) -> Result<FrameCaret, SourceFrameError> {
        let offset = self.frame.paragraphs[paragraph as usize].line_offsets[caret.line as usize];
        caret.edge.x = caret.edge.x.checked_add(offset.x)?;
        caret.edge.top = caret.edge.top.checked_add(offset.y)?;
        caret.edge.bottom = caret.edge.bottom.checked_add(offset.y)?;
        let visible = self
            .clip(Rect {
                min: Point {
                    x: caret.edge.x,
                    y: caret.edge.top,
                },
                max: Point {
                    x: caret.edge.x,
                    y: caret.edge.bottom,
                },
            })
            .map(|rect| CaretEdge {
                x: rect.min.x,
                top: rect.min.y,
                bottom: rect.max.y,
            });
        Ok(FrameCaret {
            paragraph,
            caret,
            visible,
        })
    }
    fn caret(&self, position: FrameTextPosition) -> Result<FrameCaret, SourceFrameError> {
        self.placed_caret(
            position.paragraph,
            self.maps[position.paragraph as usize].caret(position.position)?,
        )
    }
    fn hit(
        &self,
        point: Point,
        work: &mut usize,
        check: &dyn Fn() -> bool,
    ) -> Result<FrameTextQueryResult, SourceFrameError> {
        let mut best = None;
        for (p, map) in self.maps.iter().enumerate() {
            charge(
                work,
                map.lines.len(),
                self.limits.max_query_work,
                "frame interaction query work",
            )?;
            let frame = &self.frame.paragraphs[p];
            let geometry = frame
                .computed
                .geometry
                .precise
                .as_ref()
                .expect("complete frame");
            for (l, line) in map.lines.iter().enumerate() {
                cancel(check)?;
                let by = frame.line_offsets[l];
                let rank = (
                    distance(
                        point.y,
                        line.top.checked_add(by.y)?,
                        line.bottom.checked_add(by.y)?,
                    ),
                    distance(
                        point.x,
                        geometry.lines[l].pen_min.checked_add(by.x)?,
                        geometry.lines[l].pen_max.checked_add(by.x)?,
                    ),
                    p,
                    l,
                );
                if best.as_ref().is_none_or(|(current, _)| rank < *current) {
                    best = Some((rank, (p, l)));
                }
            }
        }
        let (_, (p, l)) = best.ok_or_else(|| invalid("empty frame interaction"))?;
        let map = &self.maps[p];
        charge(
            work,
            map.lines[l].cells.len() * 2,
            self.limits.max_query_work,
            "frame interaction query work",
        )?;
        let by = self.frame.paragraphs[p].line_offsets[l];
        let local = Point {
            x: point.x.checked_sub(by.x)?,
            y: point.y.checked_sub(by.y)?,
        };
        let TextQueryResult::Hit { caret, inside } = map.hit_in_line(l as u32, local, check)?
        else {
            unreachable!()
        };
        Ok(FrameTextQueryResult::Hit {
            caret: self.placed_caret(p as u32, caret)?,
            inside: inside
                && self
                    .clip(Rect {
                        min: point,
                        max: point,
                    })
                    .is_some(),
        })
    }
    /// All requests are validated before querying. The owner is immutable and
    /// cannot be reconstructed from client-supplied maps or line translations.
    pub fn query(
        &self,
        queries: &[FrameTextQuery],
        check: &dyn Fn() -> bool,
    ) -> Result<Vec<FrameTextQueryResult>, SourceFrameError> {
        self.query_using(queries, &mut 0, &mut 0, check)
    }
    pub(crate) fn query_using(
        &self,
        queries: &[FrameTextQuery],
        work: &mut usize,
        selected: &mut usize,
        check: &dyn Fn() -> bool,
    ) -> Result<Vec<FrameTextQueryResult>, SourceFrameError> {
        cancel(check)?;
        if queries.len() > 64 {
            return Err(SourceFrameError::Limit("frame interaction queries"));
        }
        for query in queries {
            cancel(check)?;
            match *query {
                FrameTextQuery::Caret { position } => {
                    self.boundary(position)?;
                }
                FrameTextQuery::Selection { anchor, focus } => {
                    self.boundary(anchor)?;
                    self.boundary(focus)?;
                }
                FrameTextQuery::Hit { .. } => {}
            }
        }
        let mut result = Vec::with_capacity(queries.len());
        for query in queries {
            cancel(check)?;
            result.push(match *query {
                FrameTextQuery::Caret { position } => {
                    charge(
                        work,
                        self.maps[position.paragraph as usize].lines.len(),
                        self.limits.max_query_work,
                        "frame interaction query work",
                    )?;
                    FrameTextQueryResult::Caret {
                        caret: self.caret(position)?,
                    }
                }
                FrameTextQuery::Hit { point } => self.hit(point, work, check)?,
                FrameTextQuery::Selection { anchor, focus } => {
                    let key = |p: FrameTextPosition| (p.paragraph, p.position.scalar_offset);
                    let (lo, hi) = if key(anchor) <= key(focus) {
                        (anchor, focus)
                    } else {
                        (focus, anchor)
                    };
                    let mut fragments = Vec::new();
                    let paragraph_breaks: Vec<_> = (lo.paragraph..hi.paragraph).collect();
                    charge(
                        selected,
                        paragraph_breaks.len(),
                        self.limits.max_selection_fragments,
                        "frame selection fragments",
                    )?;
                    for p in lo.paragraph..=hi.paragraph {
                        cancel(check)?;
                        let map = &self.maps[p as usize];
                        let a = if p == lo.paragraph {
                            lo.position
                        } else {
                            TextPosition {
                                scalar_offset: 0,
                                affinity: Affinity::Downstream,
                            }
                        };
                        let b = if p == hi.paragraph {
                            hi.position
                        } else {
                            TextPosition {
                                scalar_offset: map
                                    .boundaries
                                    .last()
                                    .expect("map boundary")
                                    .scalar_offset,
                                affinity: Affinity::Upstream,
                            }
                        };
                        let start = self.boundary(FrameTextPosition {
                            paragraph: p,
                            position: a,
                        })?;
                        let end = self.boundary(FrameTextPosition {
                            paragraph: p,
                            position: b,
                        })?;
                        charge(
                            selected,
                            end - start,
                            self.limits.max_selection_fragments,
                            "frame selection fragments",
                        )?;
                        charge(
                            work,
                            end - start + map.lines.len() * 2,
                            self.limits.max_query_work,
                            "frame interaction query work",
                        )?;
                        let mut r = map.query(
                            &[TextQuery::Selection {
                                anchor: a,
                                focus: b,
                            }],
                            check,
                        )?;
                        let TextQueryResult::Selection {
                            fragments: local, ..
                        } = r.remove(0)
                        else {
                            unreachable!()
                        };
                        for mut fragment in local {
                            cancel(check)?;
                            fragment.bounds = fragment.bounds.translate(
                                self.frame.paragraphs[p as usize].line_offsets
                                    [fragment.line as usize],
                            )?;
                            fragments.push(FrameSelectionFragment {
                                paragraph: p,
                                visible: self.clip(fragment.bounds),
                                fragment,
                            });
                        }
                    }
                    // Anchor/focus retain the user's direction, even for a backward selection.
                    charge(
                        work,
                        self.maps[anchor.paragraph as usize].lines.len()
                            + self.maps[focus.paragraph as usize].lines.len(),
                        self.limits.max_query_work,
                        "frame interaction query work",
                    )?;
                    FrameTextQueryResult::Selection {
                        anchor: Box::new(self.caret(anchor)?),
                        focus: Box::new(self.caret(focus)?),
                        fragments,
                        paragraph_breaks,
                    }
                }
            });
        }
        cancel(check)?;
        Ok(result)
    }
}
