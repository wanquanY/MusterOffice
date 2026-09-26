use super::*;
use mo_pptx::source::text::{body::*, cascade::*, *};
use mo_text::{
    flow::{LineWidths, OverflowPolicy},
    geometry::LineSpacing,
};
use mo_unicode::line_break::{LineBreakClass as B, line_break_properties};

pub(super) fn body(body: &EffectiveTextBody) -> Result<(), SourceFrameError> {
    use TextBodyProperty::*;
    let a = &body.attributes;
    let checks = [
        (Rotation, a.rotation == Some(0)),
        (Vertical, a.vertical == Some(NativeTextVertical::Horz)),
        (Wrap, a.wrap == Some(NativeTextWrap::Square)),
        (Columns, a.columns == Some(1)),
        (FromWordArt, a.from_word_art == Some(false)),
        (CenterAnchor, a.center_anchor == Some(false)),
        (Upright, a.upright == Some(false)),
        (
            CompatibleLineSpacing,
            a.compatible_line_spacing == Some(false),
        ),
        (
            VerticalOverflow,
            a.vertical_overflow == Some(NativeTextVerticalOverflow::Overflow),
        ),
        (
            HorizontalOverflow,
            a.horizontal_overflow == Some(NativeTextHorizontalOverflow::Overflow),
        ),
        (
            Anchor,
            matches!(
                a.anchor,
                Some(NativeTextAnchor::T | NativeTextAnchor::Ctr | NativeTextAnchor::B)
            ),
        ),
    ];
    for (property, supported) in checks {
        if !supported {
            return Err(mapping(SourceFrameIssue::BodyProperty { property }));
        }
    }
    if !matches!(body.autofit, EffectiveTextAutofit::None { .. }) {
        return Err(mapping(SourceFrameIssue::Autofit));
    }
    Ok(())
}
fn declaration_error(paragraph: u32, declaration: &TextStyleDeclaration) -> SourceFrameError {
    mapping(SourceFrameIssue::ParagraphDeclaration {
        paragraph,
        declaration: declaration.clone(),
    })
}
pub(super) fn paragraph(
    index: &SourceIndex,
    ordinal: u32,
    p: &CascadedParagraph,
    input: &SourceParagraphPlan,
    region: &SourceFrameRegion,
    check: &dyn Fn() -> bool,
) -> Result<FrameParagraphSpec, SourceFrameError> {
    let a = &p.attributes;
    let fail = |property| {
        mapping(SourceFrameIssue::ParagraphProperty {
            paragraph: ordinal,
            property,
        })
    };
    if !matches!(
        a.alignment,
        Some(NativeTextAlign::L | NativeTextAlign::Ctr | NativeTextAlign::R)
    ) {
        return Err(fail(ParagraphProperty::Alignment));
    }
    if a.font_alignment != Some(NativeTextFontAlign::Base) {
        return Err(fail(ParagraphProperty::FontAlignment));
    }
    for ch in input.text.chars() {
        cancel(check)?;
        let props = line_break_properties(ch);
        if a.east_asian_line_break == Some(false) && props.east_asian {
            return Err(fail(ParagraphProperty::EastAsianLineBreak));
        }
        // Do not silently render hanging punctuation with ordinary line bounds.
        if a.hanging_punctuation != Some(false)
            && (props.initial_punctuation
                || props.final_punctuation
                || matches!(
                    props.class,
                    B::Cl
                        | B::Cp
                        | B::Ex
                        | B::Is
                        | B::Sy
                        | B::Qu
                        | B::Op
                        | B::Po
                        | B::Pr
                        | B::Hy
                        | B::Ba
                        | B::Bb
                        | B::B2
                        | B::In
                        | B::Ns
                        | B::Cj
                ))
        {
            return Err(fail(ParagraphProperty::HangingPunctuation));
        }
    }
    let emu = |v: Option<i32>| Fixed::emu(mo_common::Emu::new(i64::from(v.unwrap_or(0))));
    let left = emu(a.left_margin);
    let right = emu(a.right_margin);
    let indent = emu(a.indent);
    let rest = region
        .inner
        .max
        .x
        .checked_sub(region.inner.min.x)?
        .checked_sub(left)?
        .checked_sub(right)?;
    let first = rest.checked_sub(indent)?;
    if rest <= Fixed::ZERO || first <= Fixed::ZERO {
        return Err(mapping(SourceFrameIssue::InvalidRegion));
    }
    let mut result = FrameParagraphSpec {
        source_ordinal: p.source_ordinal,
        widths: LineWidths { first, rest },
        left,
        indent,
        indent_from_right: a.right_to_left == Some(true),
        spacing: LineSpacing::Natural,
        before: ParagraphSpacing::Fixed(Fixed::ZERO),
        after: ParagraphSpacing::Fixed(Fixed::ZERO),
        spacing_conversion_error: Fixed::ZERO,
        alignment: a.alignment.expect("validated alignment"),
        overflow: if a.latin_line_break == Some(true) {
            OverflowPolicy::EmergencyGrapheme
        } else {
            OverflowPolicy::KeepUnbreakable
        },
    };
    for (slot, reference) in &p.declarations {
        cancel(check)?;
        match slot {
            ParagraphSlot::LineSpacing | ParagraphSlot::SpaceBefore | ParagraphSlot::SpaceAfter => {
                let (value, error) = spacing::read(index, ordinal, reference, input, check)?;
                result.spacing_conversion_error = result.spacing_conversion_error.max(error);
                match slot {
                    ParagraphSlot::LineSpacing => {
                        result.spacing = match value {
                            ParagraphSpacing::Fixed(height) if height > Fixed::ZERO => {
                                LineSpacing::Exact {
                                    height: height.wire()?,
                                }
                            }
                            ParagraphSpacing::Fixed(height) => LineSpacing::StyleMaximum {
                                heights: vec![height; input.geometry.len()],
                            },
                            ParagraphSpacing::StyleMaximum { heights } => {
                                LineSpacing::StyleMaximum { heights }
                            }
                        };
                    }
                    ParagraphSlot::SpaceBefore => result.before = value,
                    ParagraphSlot::SpaceAfter => result.after = value,
                    _ => unreachable!(),
                }
            }
            ParagraphSlot::Bullet if reference.element == NativeTextElement::BuNone => {
                let n = cascade::declaration(index, reference)?;
                if !n.retained_ordinals.is_empty() || !n.children.is_empty() {
                    return Err(declaration_error(ordinal, reference));
                }
            }
            ParagraphSlot::BulletColor | ParagraphSlot::BulletSize | ParagraphSlot::BulletFont
                if p.declarations
                    .get(&ParagraphSlot::Bullet)
                    .is_some_and(|r| r.element == NativeTextElement::BuNone) => {}
            _ => return Err(declaration_error(ordinal, reference)),
        }
    }
    Ok(result)
}
