use super::*;
use mo_common::Emu;
use mo_presentation_source::source::text::{cascade::*, fonts::*, *};
use mo_text::{ShapeFeature, geometry::GeometryStyle};

/// Tab advances have no glyph outline. Until the paint compiler consumes those
/// advances as decoration spans, do not silently drop their underline/strike.
pub(super) fn audit_tab(
    style: &CascadedCharacterStyle,
    paragraph: u32,
    run: u32,
) -> Option<SourceTextIssue> {
    let property = if style.attributes.underline != Some(NativeTextUnderline::None) {
        CharacterProperty::Underline
    } else if style.attributes.strike != Some(NativeTextStrike::NoStrike) {
        CharacterProperty::Strike
    } else {
        return None;
    };
    Some(SourceTextIssue::CharacterProperty {
        paragraph,
        run: Some(run),
        property,
    })
}

pub(super) fn audit(
    style: &CascadedCharacterStyle,
    paragraph: u32,
    run: Option<u32>,
) -> Option<SourceTextIssue> {
    use CharacterProperty::*;
    let a = &style.attributes;
    let property = if a.caps != Some(NativeTextCaps::None) {
        Some(Caps)
    } else if a.kumimoji != Some(false) {
        Some(Kumimoji)
    } else if a.normalize_height != Some(false) {
        Some(NormalizeHeight)
    } else {
        None
    };
    if let Some(property) = property {
        return Some(SourceTextIssue::CharacterProperty {
            paragraph,
            run,
            property,
        });
    }
    if style.declarations.contains_key(&CharacterSlot::RightToLeft) {
        return Some(SourceTextIssue::DirectionOverride { paragraph, run });
    }
    if style.declarations.contains_key(&CharacterSlot::Symbol) {
        return Some(SourceTextIssue::SymbolFont { paragraph, run });
    }
    None
}
pub(super) fn effective(
    style: &CascadedCharacterStyle,
    font: &NativeTypeface,
) -> Result<(ManifestTextStyle, GeometryStyle, mo_geometry::Fixed), SourceTextError> {
    let a = &style.attributes;
    let size = a
        .size
        .ok_or(SourceTextError::Invalid("cascaded character size"))?;
    if !(100..=400_000).contains(&size) {
        return Err(SourceTextError::Invalid("cascaded character size range"));
    }
    let font_style = match (a.bold, a.italic) {
        (Some(false), Some(false)) => FontStyle::Regular,
        (Some(true), Some(false)) => FontStyle::Bold,
        (Some(false), Some(true)) => FontStyle::Italic,
        (Some(true), Some(true)) => FontStyle::BoldItalic,
        _ => return Err(SourceTextError::Invalid("cascaded font style")),
    };
    let language = a.language.as_deref().unwrap_or("und").to_ascii_lowercase();
    // Same explicit ASCII language contract as the shared shaping transport.
    // Do not copy an unbounded arbitrary native lexical value into each span.
    if language.is_empty()
        || language.len() > 255
        || !language
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'-')
    {
        return Err(SourceTextError::Invalid("native shaping language"));
    }
    let font_size = Emu::new(i64::from(size) * 127);
    let baseline_shift = super::number::baseline(
        a.baseline
            .as_ref()
            .ok_or(SourceTextError::Invalid("cascaded baseline"))?,
        font_size,
    )?;
    let (cluster_spacing, conversion_error) = super::number::tracking(
        a.spacing
            .as_ref()
            .ok_or(SourceTextError::Invalid("cascaded character spacing"))?,
    )?;
    let mut features = vec![ShapeFeature {
        tag: "kern".into(),
        value: u32::from(a.kerning.is_some_and(|threshold| size >= threshold)),
        start: 0,
        end: None,
    }];
    // Explicit tracking separates ordinary letters. Preserve required shaping
    // (rlig/ccmp/mark and script joins); suppress only optional common ligatures.
    if cluster_spacing != mo_geometry::Fixed::ZERO || conversion_error != mo_geometry::Fixed::ZERO {
        features.extend(["liga", "clig"].map(|tag| ShapeFeature {
            tag: tag.into(),
            value: 0,
            start: 0,
            end: None,
        }));
    }
    Ok((
        ManifestTextStyle {
            typeface: font.typeface.clone(),
            font_style,
            language,
            features,
            suppress_dotted_circle: false,
            max_glyphs: 262_144,
        },
        GeometryStyle {
            font_size,
            baseline_shift,
            cluster_spacing,
        },
        conversion_error,
    ))
}
pub(super) fn equal(a: &ManifestTextStyle, b: &ManifestTextStyle) -> bool {
    a.typeface == b.typeface
        && a.font_style == b.font_style
        && a.language == b.language
        && a.features == b.features
        && a.suppress_dotted_circle == b.suppress_dotted_circle
        && a.max_glyphs == b.max_glyphs
}
