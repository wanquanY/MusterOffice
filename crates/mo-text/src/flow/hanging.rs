//! Candidate-local end punctuation. The pinned Unicode line-break properties
//! distinguish closing punctuation from openers, currency, spaces and hyphens.
use super::*;
use mo_unicode::line_break::{LineBreakClass as B, line_break_properties};

pub(super) fn terminal(
    q: &FlowInput<'_>,
    plan: &LinePlan,
    segmentation: &TextSegmentation,
) -> Option<(mo_unicode::TextBoundary, mo_unicode::TextBoundary)> {
    if q.hanging_punctuation == HangingPunctuation::None {
        return None;
    }
    // A hard line break has no glyph. Trailing spaces, on the other hand,
    // remain content and prevent the punctuation from being the visual edge.
    let last = plan
        .items
        .iter()
        .rev()
        .find(|i| i.kind != TextItemKind::LineBreak)?;
    if last.kind != TextItemKind::Text {
        return None;
    }
    let end = segmentation
        .boundaries
        .binary_search_by_key(&last.end.scalar_offset, |b| b.scalar_offset)
        .ok()?;
    let start = end.checked_sub(1)?;
    let from = &segmentation.boundaries[start];
    let to = &segmentation.boundaries[end];
    if from.scalar_offset <= plan.lines[0].start.scalar_offset {
        return None;
    }
    let mut chars = q.paragraph.text[from.utf8_offset as usize..to.utf8_offset as usize].chars();
    let props = line_break_properties(chars.next()?);
    let eligible = !props.initial_punctuation
        && (props.final_punctuation
            || matches!(props.class, B::Cl | B::Cp | B::Ex | B::Is | B::Sy | B::Qu));
    if !eligible || chars.any(|c| !line_break_properties(c).combining_mark) {
        return None;
    }
    Some((from.clone(), to.clone()))
}
