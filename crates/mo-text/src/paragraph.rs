//! Automatic logical itemization connected to explicit font resources and the
//! existing isolated shaper. Planned line contexts are handled by `lines`.
use crate::{cascade::*, itemize::*, *};
use mo_unicode::bidi::ParagraphDirection;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ParagraphTextStyle {
    pub language: String,
    pub features: Vec<ShapeFeature>,
    pub candidates: Vec<FontCandidate>,
    pub suppress_dotted_circle: bool,
    pub max_glyphs: u32,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ParagraphShapeRequest {
    pub text: String,
    pub direction: ParagraphDirection,
    pub spans: Vec<StyleSpan>,
    pub styles: Vec<ParagraphTextStyle>,
    pub fonts: Vec<CascadeFont>,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ParagraphShapeResult {
    pub profile: String,
    pub itemization: ItemizationResult,
    /// One index per cascade result. Layout controls are present only in the
    /// itemization; their source text remains in every shaping context.
    pub shaped_item_indices: Vec<u32>,
    pub fallback: fallback::FallbackResult,
}
pub(crate) fn item(
    start: u32,
    end: u32,
    direction: Direction,
    script: String,
    style: &ParagraphTextStyle,
    count: u32,
) -> CascadeItem {
    CascadeItem {
        start,
        end,
        direction,
        script,
        language: style.language.clone(),
        features: style.features.clone(),
        beginning_of_text: start == 0,
        end_of_text: end == count,
        suppress_dotted_circle: style.suppress_dotted_circle,
        max_glyphs: style.max_glyphs,
        candidates: style.candidates.clone(),
    }
}
pub(crate) fn prepare_items(
    request: &ParagraphShapeRequest,
    bundle_length: usize,
    check: &dyn Fn() -> bool,
) -> Result<ItemizationResult, TextError> {
    prepare_items_with(request, bundle_length, check, &|_, _| true)
}
pub(crate) fn prepare_items_with(
    request: &ParagraphShapeRequest,
    bundle_length: usize,
    check: &dyn Fn() -> bool,
    equivalent: &dyn Fn(usize, usize) -> bool,
) -> Result<ItemizationResult, TextError> {
    cancelled(check)?;
    if request.styles.len() > 256 || request.spans.len() > 256 {
        return Err(TextError::Limit("paragraph styles or spans"));
    }
    if request.text.len() > 262144 || request.fonts.len() > 32 || bundle_length > 128 * 1024 * 1024
    {
        return Err(TextError::Limit("paragraph text or resources"));
    }
    for style in &request.styles {
        cancelled(check)?;
        if style.language.len() > 255
            || style.features.len() > 1024
            || style.candidates.len() > 32
            || style.candidates.iter().any(|c| c.variations.len() > 64)
        {
            return Err(TextError::Limit("paragraph style size"));
        }
    }
    // Normalize computation keys only; author style records stay unchanged.
    // BCP47 case, independent axis order and alias bindings are not shaping
    // differences and must not accidentally cut a cross-run ligature.
    let font_keys: Vec<_> = request
        .fonts
        .iter()
        .enumerate()
        .map(|(i, font)| {
            request.fonts[..i]
                .iter()
                .position(|other| {
                    other.expected_sha256 == font.expected_sha256
                        && other.face_index == font.face_index
                })
                .unwrap_or(i) as u32
        })
        .collect();
    let mut normalized = request.styles.clone();
    for style in &mut normalized {
        cancelled(check)?;
        style.language.make_ascii_lowercase();
        for candidate in &mut style.candidates {
            cancelled(check)?;
            if let Some(&key) = font_keys.get(candidate.font as usize) {
                candidate.font = key;
            }
            candidate.variations.sort_by(|a, b| a.tag.cmp(&b.tag));
        }
    }
    let mut canonical = Vec::new();
    for (i, style) in normalized.iter().enumerate() {
        cancelled(check)?;
        canonical.push(
            normalized[..i]
                .iter()
                .enumerate()
                .position(|(j, other)| other == style && equivalent(i, j))
                .unwrap_or(i),
        );
    }
    let mut spans = Vec::new();
    for span in &request.spans {
        cancelled(check)?;
        let Some(&style) = canonical.get(span.style as usize) else {
            return Err(TextError::Invalid("paragraph style reference"));
        };
        spans.push(StyleSpan {
            end: span.end,
            style: style as u32,
        });
    }
    let languages: Vec<_> = request
        .spans
        .iter()
        .map(|span| LanguageSpan {
            end: span.end,
            language: &request.styles[span.style as usize].language,
        })
        .collect();
    itemize_with_languages(
        &ItemizationRequest {
            text: request.text.clone(),
            direction: request.direction,
            spans,
        },
        &languages,
        ItemizationLimits::default(),
        check,
    )
}
pub fn shape_paragraph(
    request: &ParagraphShapeRequest,
    bundle: &[u8],
    backend: &mut dyn backend::TextBackend,
    check: &dyn Fn() -> bool,
) -> Result<ParagraphShapeResult, TextError> {
    shape_paragraph_using(
        request,
        resources::ResourceInput::Bundle(bundle),
        backend,
        check,
    )
}
/// Reuses already-verified immutable resources while validating this paragraph's
/// own styles before the first component call.
pub(crate) fn shape_paragraph_using(
    request: &ParagraphShapeRequest,
    input: resources::ResourceInput<'_, '_>,
    backend: &mut dyn backend::TextBackend,
    check: &dyn Fn() -> bool,
) -> Result<ParagraphShapeResult, TextError> {
    let itemization = prepare_items(request, input.bundle_length(), check)?;
    let count = request.text.chars().count() as u32;
    let mut items = Vec::new();
    let mut indices = Vec::new();
    for (index, part) in itemization.items.iter().enumerate() {
        cancelled(check)?;
        if part.kind != TextItemKind::Text {
            continue;
        }
        if items.len() >= CascadeLimits::default().max_items {
            return Err(TextError::Limit("paragraph shaping items"));
        }
        let direction = if part.level % 2 == 0 {
            Direction::LeftToRight
        } else {
            Direction::RightToLeft
        };
        items.push(item(
            part.start.scalar_offset,
            part.end.scalar_offset,
            direction,
            part.script.tag().into(),
            &request.styles[part.style as usize],
            count,
        ));
        indices.push(index as u32);
    }
    let fallback = fallback::shape_fallback_validating(
        &CascadeRequest {
            text: request.text.clone(),
            fonts: request.fonts.clone(),
            items,
        },
        input,
        backend,
        fallback::FallbackLimits::default(),
        check,
        |fonts, bindings| validate_styles(request, fonts, bindings, check),
    )?;
    Ok(ParagraphShapeResult {
        profile: "unicode18-auto-items-hb14.5-language-reshape-v3".into(),
        itemization,
        shaped_item_indices: indices,
        fallback,
    })
}

pub(crate) fn validate_styles(
    request: &ParagraphShapeRequest,
    fonts: &[mo_font::VerifiedFont<'_>],
    bindings: &[usize],
    check: &dyn Fn() -> bool,
) -> Result<(), TextError> {
    let count = request.text.chars().count() as u32;
    // Even unused/control-only styles are checked, before the first shaping
    // call. Reuse this batch's verified font objects for axis validation.
    for style in &request.styles {
        cancelled(check)?;
        if style.candidates.is_empty() {
            return Err(TextError::Invalid("paragraph font candidates"));
        }
        if style.candidates.len() > 32 {
            return Err(TextError::Limit("paragraph font candidates"));
        }
        let candidate_item = item(0, 0, Direction::LeftToRight, "Zyyy".into(), style, count);
        for candidate in &style.candidates {
            cancelled(check)?;
            let Some(&font) = bindings.get(candidate.font as usize) else {
                return Err(TextError::Invalid("paragraph candidate font"));
            };
            prepare::validate_run(
                &candidate_item.run(candidate),
                fonts[font].metadata(),
                count as usize,
                TextLimits::default(),
            )?;
        }
    }
    Ok(())
}

#[cfg(test)]
#[path = "paragraph_tests.rs"]
mod tests;
