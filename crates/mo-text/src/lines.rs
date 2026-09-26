//! Reshape planned lines without losing paragraph bidi/script resolution.
//! The line plan is computed data, never an edit to the logical author text.
pub(crate) mod plan;
use crate::{cascade::*, fallback::*, itemize::*, paragraph::*, *};
use mo_unicode::{TextBoundary, UnicodeLimits, bidi::*};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct LineShapeRequest {
    pub paragraph: ParagraphShapeRequest,
    /// Strictly increasing, exhaustive EGC-aligned scalar ends; [0] for empty
    /// text. These are an explicit computational plan, not author hard breaks.
    pub line_ends: Vec<u32>,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ShapedLine {
    pub start: TextBoundary,
    pub end: TextBoundary,
    /// Half-open indices into the result's logical items.
    pub item_start: u32,
    pub item_end: u32,
    /// Half-open indices into fallback.items and shapedItemIndices.
    pub fallback_start: u32,
    pub fallback_end: u32,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct LineShapeResult {
    pub profile: String,
    pub bidi: BidiParagraphResult,
    /// Paragraph-resolved script/style and line-L1-adjusted cluster levels.
    pub items: Vec<TextItem>,
    pub notices: Vec<ItemizationNotice>,
    pub lines: Vec<ShapedLine>,
    pub shaped_item_indices: Vec<u32>,
    pub fallback: FallbackResult,
}
pub(crate) fn bidi_error(e: BidiError) -> TextError {
    match e {
        BidiError::Invalid(m) => TextError::Invalid(m),
        BidiError::Limit(m) => TextError::Limit(m),
        BidiError::Cancelled => TextError::Cancelled,
    }
}
pub fn shape_lines(
    request: &LineShapeRequest,
    bundle: &[u8],
    backend: &mut dyn backend::TextBackend,
    check: &dyn Fn() -> bool,
) -> Result<LineShapeResult, TextError> {
    shape_lines_with(
        request,
        bundle,
        backend,
        check,
        &|_, _| true,
        |result, _, _, _| Ok(result),
    )
}
pub(crate) fn shape_lines_with<T>(
    request: &LineShapeRequest,
    bundle: &[u8],
    backend: &mut dyn backend::TextBackend,
    check: &dyn Fn() -> bool,
    equivalent: &dyn Fn(usize, usize) -> bool,
    finish: impl FnOnce(
        LineShapeResult,
        &[mo_font::VerifiedFont<'_>],
        &[usize],
        &mut dyn backend::TextBackend,
    ) -> Result<T, TextError>,
) -> Result<T, TextError> {
    cancelled(check)?;
    if request.line_ends.is_empty() {
        return Err(TextError::Invalid("explicit line plan must contain an end"));
    }
    if request.line_ends.len() > 4096 {
        return Err(TextError::Limit("explicit line plan ends"));
    }
    let q = &request.paragraph;
    let original = paragraph::prepare_items_with(q, bundle.len(), check, equivalent)?;
    // Resolve bidi in paragraph context, then apply L1/L2 per actual line. A
    // new bidi paragraph per line would corrupt neutrals, isolates and brackets.
    let bidi = analyze_paragraph(
        &BidiParagraphRequest {
            text: q.text.clone(),
            direction: q.direction,
            line_ends: request.line_ends.clone(),
        },
        BidiLimits::default(),
        check,
    )
    .map_err(bidi_error)?;
    let segmentation = mo_unicode::segment(&q.text, UnicodeLimits::default(), check)
        .map_err(cascade::unicode_error)?;
    let plan = plan::plan(
        q,
        &original,
        &segmentation,
        &bidi.lines,
        bidi.paragraph_level,
        check,
    )?;
    let cascade = CascadeRequest {
        text: q.text.clone(),
        fonts: q.fonts.clone(),
        items: plan.cascade_items.clone(),
    };
    let limits = FallbackLimits {
        cascade: CascadeLimits {
            max_items: 4096,
            ..Default::default()
        },
        ..Default::default()
    };
    let mut context = cascade::context::Context::prepare(
        &cascade,
        bundle,
        limits.cascade,
        check,
        |fonts, bindings| paragraph::validate_styles(q, fonts, bindings, check),
    )?;
    let fallback = fallback::shape_prepared(
        &mut context,
        &cascade.items,
        backend,
        limits,
        check,
        &plan.scopes,
    )?;
    cancelled(check)?;
    let result = plan.finish(bidi, fallback);
    finish(result, context.fonts(), context.bindings(), backend)
}

#[cfg(test)]
#[path = "lines_tests.rs"]
mod tests;
