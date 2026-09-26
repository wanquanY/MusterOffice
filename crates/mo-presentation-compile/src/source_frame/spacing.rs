use super::*;
use crate::source_number::{PercentageError, percentage};
use mo_common::Emu;
use mo_pptx::source::text::{SourceTextValue, cascade};
use mo_text::{geometry::line_style_maximum, lines::LineShapeResult};

pub(super) fn read(
    index: &SourceIndex,
    paragraph: u32,
    reference: &mo_pptx::source::text::cascade::TextStyleDeclaration,
    input: &SourceParagraphPlan,
    check: &dyn Fn() -> bool,
) -> Result<(ParagraphSpacing, Fixed), SourceFrameError> {
    let fail = |declaration| {
        mapping(SourceFrameIssue::ParagraphDeclaration {
            paragraph,
            declaration,
        })
    };
    let node = cascade::declaration(index, reference)?;
    if !node.retained_ordinals.is_empty() || node.children.len() != 1 {
        return Err(fail(reference.clone()));
    }
    let (child_ref, child) = cascade::declaration_child(index, reference, node.children[0])?;
    if !child.retained_ordinals.is_empty() || !child.children.is_empty() {
        return Err(fail(child_ref));
    }
    match &child.value {
        SourceTextValue::Points { value } if *value >= 0 => Ok((
            ParagraphSpacing::Fixed(Fixed::emu(Emu::new(i64::from(*value) * 127))),
            Fixed::ZERO,
        )),
        SourceTextValue::Percentage { value } => {
            let mut heights = Vec::with_capacity(input.geometry.len());
            let mut error = Fixed::ZERO;
            for style in &input.geometry {
                cancel(check)?;
                let scaled = percentage(value, style.font_size).map_err(|e| match e {
                    PercentageError::LexicalLimit => {
                        SourceFrameError::Limit("paragraph spacing lexical bytes")
                    }
                    PercentageError::Range => fail(child_ref.clone()),
                })?;
                if scaled.value < Fixed::ZERO {
                    return Err(fail(child_ref));
                }
                heights.push(scaled.value);
                if scaled.fractional {
                    error = Fixed::from_raw(1);
                }
            }
            Ok((ParagraphSpacing::StyleMaximum { heights }, error))
        }
        _ => Err(fail(child_ref)),
    }
}
pub(super) fn resolve(
    spacing: &ParagraphSpacing,
    shaping: &LineShapeResult,
    line: u32,
    strut: u32,
) -> Result<Fixed, SourceFrameError> {
    match spacing {
        ParagraphSpacing::Fixed(value) => Ok(*value),
        ParagraphSpacing::StyleMaximum { heights } => {
            Ok(line_style_maximum(shaping, line, heights, strut)?)
        }
    }
}
