use super::*;
use crate::source_frame::interaction::{
    FrameCaret, FrameInteraction, FrameSelectionFragment, FrameTextQuery, FrameTextQueryResult,
};
use mo_geometry::{Point, Rect};
use mo_text::interaction::CaretEdge;
use transform::Transform;
fn cancel(check: &dyn Fn() -> bool) -> Result<(), SourcePageError> {
    if check() {
        Err(mo_raster::RasterError::Cancelled.into())
    } else {
        Ok(())
    }
}
fn edges(c: CaretEdge) -> [Point; 2] {
    [
        Point { x: c.x, y: c.top },
        Point {
            x: c.x,
            y: c.bottom,
        },
    ]
}
fn corners(r: Rect) -> [Point; 4] {
    [
        r.min,
        Point {
            x: r.max.x,
            y: r.min.y,
        },
        r.max,
        Point {
            x: r.min.x,
            y: r.max.y,
        },
    ]
}
fn caret(
    t: &Transform<'_>,
    local: FrameCaret,
    check: &dyn Fn() -> bool,
) -> Result<PageCaret, SourcePageError> {
    let (edge, mut error) = t.points(edges(local.caret.edge), check)?;
    let visible = local
        .visible
        .map(|c| t.points(edges(c), check))
        .transpose()?
        .map(|(v, e)| {
            error = error.max(e);
            v
        });
    Ok(PageCaret {
        local,
        edge,
        visible,
        coordinate_error_bound: error,
    })
}
fn selection(
    t: &Transform<'_>,
    local: FrameSelectionFragment,
    check: &dyn Fn() -> bool,
) -> Result<PageSelectionFragment, SourcePageError> {
    let (quad, mut error) = t.points(corners(local.fragment.bounds), check)?;
    let visible = local
        .visible
        .map(|r| t.points(corners(r), check))
        .transpose()?
        .map(|(v, e)| {
            error = error.max(e);
            v
        });
    Ok(PageSelectionFragment {
        local,
        quad,
        visible,
        coordinate_error_bound: error,
    })
}
impl SourceEditorPage {
    /// Query the immutable rendered page. Hit input is page EMU; all output
    /// corners/segments include native object/group transforms and axis clips.
    pub fn query(
        &self,
        queries: &[PageTextQuery],
        check: &dyn Fn() -> bool,
    ) -> Result<Vec<PageTextQueryResult>, SourcePageError> {
        cancel(check)?;
        if queries.len() > 64 {
            return Err(mo_raster::RasterError::Limit("page text queries").into());
        }
        let texts = &self.page.text.as_ref().expect("editor text context").texts;
        for q in queries {
            cancel(check)?;
            if q.frame as usize >= texts.len() {
                return Err(SourcePageError::Invalid("page text frame index"));
            }
        }
        let mut work = 0usize;
        let mut selected = 0usize;
        let mut results = Vec::with_capacity(queries.len());
        for q in queries {
            cancel(check)?;
            let text = &texts[q.frame as usize];
            let object = &self.page.page.bindings[text.binding as usize];
            let transform = Transform {
                object,
                viewport: &self.page.page.raster.viewport,
                uncertainty: text.local_coordinate_error_bound,
            };
            let action = match q.action {
                PageTextAction::Move {
                    position,
                    movement,
                    preferred_x,
                } => FrameTextQuery::Move {
                    position,
                    movement,
                    preferred_x,
                },
                PageTextAction::Caret { position } => FrameTextQuery::Caret { position },
                PageTextAction::Selection { anchor, focus } => {
                    FrameTextQuery::Selection { anchor, focus }
                }
                PageTextAction::Hit { point } => {
                    let Some(point) = transform.inverse(point)? else {
                        results.push(PageTextQueryResult::Hit {
                            frame: q.frame,
                            caret: None,
                            inside: false,
                        });
                        continue;
                    };
                    FrameTextQuery::Hit { point }
                }
            };
            let view = FrameInteraction {
                frame: &text.frame,
                maps: &self.interaction.maps[q.frame as usize],
                limits: self.interaction.limits,
            };
            let mut queried = view.query_using(&[action], &mut work, &mut selected, check)?;
            let answer = queried.remove(0);
            let points = match &answer {
                FrameTextQueryResult::Selection { fragments, .. } => 8 + fragments.len() * 8,
                _ => 4,
            };
            work = work
                .checked_add(points)
                .filter(|&n| n <= self.interaction.limits.max_query_work)
                .ok_or(mo_raster::RasterError::Limit("page text query work"))?;
            results.push(match answer {
                FrameTextQueryResult::Moved {
                    caret: c,
                    preferred_x,
                    exhausted,
                } => PageTextQueryResult::Moved {
                    frame: q.frame,
                    caret: Box::new(caret(&transform, c, check)?),
                    preferred_x,
                    exhausted,
                },
                FrameTextQueryResult::Caret { caret: c } => PageTextQueryResult::Caret {
                    frame: q.frame,
                    caret: Box::new(caret(&transform, c, check)?),
                },
                FrameTextQueryResult::Hit { caret: c, inside } => PageTextQueryResult::Hit {
                    frame: q.frame,
                    caret: Some(Box::new(caret(&transform, c, check)?)),
                    inside,
                },
                FrameTextQueryResult::Selection {
                    anchor,
                    focus,
                    fragments,
                    paragraph_breaks,
                } => PageTextQueryResult::Selection {
                    frame: q.frame,
                    anchor: Box::new(caret(&transform, *anchor, check)?),
                    focus: Box::new(caret(&transform, *focus, check)?),
                    fragments: fragments
                        .into_iter()
                        .map(|f| selection(&transform, f, check))
                        .collect::<Result<_, _>>()?,
                    paragraph_breaks,
                },
            });
        }
        cancel(check)?;
        Ok(results)
    }
}
