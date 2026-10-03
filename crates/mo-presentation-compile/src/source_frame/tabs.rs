//! DrawingML declarations to explicit left-tab layout policy.
use super::*;
use mo_presentation_source::source::text::{cascade::*, *};
use mo_text::geometry::LeftTabStops;

pub(super) fn read(
    index: &SourceIndex,
    ordinal: u32,
    p: &CascadedParagraph,
    indent: Fixed,
    has_tabs: bool,
    check: &dyn Fn() -> bool,
) -> Result<Option<LeftTabStops>, SourceFrameError> {
    let fail_property = |property| {
        mapping(SourceFrameIssue::ParagraphProperty {
            paragraph: ordinal,
            property,
        })
    };
    let declaration = p.declarations.get(&ParagraphSlot::Tabs);
    // Declarations still undergo the same capability audit even when unused.
    if !has_tabs && declaration.is_none() {
        return Ok(None);
    }
    if has_tabs && p.attributes.right_to_left != Some(false) {
        return Err(fail_property(ParagraphProperty::RightToLeft));
    }
    if has_tabs && p.attributes.alignment != Some(NativeTextAlign::L) {
        return Err(fail_property(ParagraphProperty::Alignment));
    }
    let value = p
        .attributes
        .default_tab_size
        .as_ref()
        .ok_or_else(|| fail_property(ParagraphProperty::DefaultTabSize))?;
    let (interval, uncertainty) = number::coordinate(value)?.q32()?;
    // Native Coordinate32 EMUs are exact. Preserve a diagnostic for unsupported
    // fractional universal measures instead of rounding a repeatedly used grid.
    if interval <= Fixed::ZERO || uncertainty != Fixed::ZERO {
        return Err(fail_property(ParagraphProperty::DefaultTabSize));
    }
    let mut stops = vec![];
    if let Some(reference) = declaration {
        let fail = |declaration| {
            mapping(SourceFrameIssue::ParagraphDeclaration {
                paragraph: ordinal,
                declaration,
            })
        };
        let node = cascade::declaration(index, reference)?;
        if !node.retained_ordinals.is_empty() || node.children.len() > 32 {
            return Err(fail(reference.clone()));
        }
        for &child in &node.children {
            cancel(check)?;
            let (child_ref, child) = cascade::declaration_child(index, reference, child)?;
            if !child.retained_ordinals.is_empty() || !child.children.is_empty() {
                return Err(fail(child_ref));
            }
            let SourceTextValue::Tab {
                position: Some(position),
                alignment: Some(NativeTextTabAlign::L),
            } = &child.value
            else {
                return Err(fail(child_ref));
            };
            let (position, uncertainty) = number::coordinate(position)?.q32()?;
            if uncertainty != Fixed::ZERO || stops.last().is_some_and(|&last| last >= position) {
                return Err(fail(child_ref));
            }
            stops.push(position);
        }
    }
    Ok(has_tabs.then_some(LeftTabStops {
        interval,
        stops,
        first_line_offset: indent,
        continuation_offset: Fixed::ZERO,
    }))
}
