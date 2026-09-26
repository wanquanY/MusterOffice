//! Ordered font choice for caller-itemized, indivisible shaping items.
//! Never converts component failure into fallback or implies visual certification.
pub(crate) mod context;
#[cfg(test)]
mod tests;
mod types;
use crate::*;
use mo_font::{CoverageOutcome, FontCharacter};
use mo_unicode::{UnicodeError, UnicodeLimits};
use std::collections::BTreeSet;
pub use types::*;

pub(crate) fn unicode_error(error: UnicodeError) -> TextError {
    match error {
        UnicodeError::Cancelled => TextError::Cancelled,
        UnicodeError::Limit(message) => TextError::Limit(message),
    }
}
pub fn shape_cascade(
    request: &CascadeRequest,
    bundle: &[u8],
    backend: &mut dyn TextBackend,
    limits: CascadeLimits,
    check: &dyn Fn() -> bool,
) -> Result<CascadeResult, TextError> {
    shape_cascade_validating(request, bundle, backend, limits, check, |_, _| Ok(()))
}
/// Additional higher-level style validation shares the same verified resources
/// and runs before any component call; no duplicated hash/face verification.
pub(crate) fn shape_cascade_validating(
    request: &CascadeRequest,
    bundle: &[u8],
    backend: &mut dyn TextBackend,
    limits: CascadeLimits,
    check: &dyn Fn() -> bool,
    validate_styles: impl FnOnce(&[VerifiedFont<'_>], &[usize]) -> Result<(), TextError>,
) -> Result<CascadeResult, TextError> {
    let mut context = context::Context::prepare(request, bundle, limits, check, validate_styles)?;
    let mut result = CascadeResult {
        profile: "atomic-item-glyph-availability-ucd18-hb14.5-cmap14-v1".into(),
        items: vec![],
        verified_faces: context.fonts().len() as u32,
        shaping_calls: 0,
        context_scalars: 0,
        probed_glyphs: 0,
    };
    let mut selected_glyphs = 0usize;
    for item in &request.items {
        let mut attempts = Vec::new();
        let mut selected = None;
        for (index, candidate) in item.candidates.iter().enumerate() {
            cancelled(check)?;
            let shaped = context.shape(candidate, vec![item.run(candidate)], backend, check)?;
            let run = &shaped.runs[0];
            let variation_issues = context.variations(candidate, item, check)?;
            let available = run.missing_glyph_clusters.is_empty() && variation_issues.is_empty();
            attempts.push(FontAttempt {
                candidate: index as u32,
                font: candidate.font,
                missing_glyph_clusters: run.missing_glyph_clusters.clone(),
                variation_issues,
            });
            if available {
                selected_glyphs += run.glyphs.len();
                if selected_glyphs > limits.max_selected_glyphs {
                    return Err(TextError::Limit("font cascade selected glyphs"));
                }
                selected = Some((candidate.font, index as u32, shaped));
                break;
            }
        }
        result.items.push(match selected {
            Some((font, candidate, shaped)) => ItemSelection::Selected {
                font,
                candidate,
                shaped,
                attempts,
            },
            None => ItemSelection::Unresolved {
                start: item.start,
                end: item.end,
                attempts,
            },
        });
    }
    cancelled(check)?;
    result.shaping_calls = context.shaping_runs;
    result.context_scalars = context.context_scalars;
    result.probed_glyphs = context.probed_glyphs;
    Ok(result)
}
fn variation_issues(
    font: &VerifiedFont<'_>,
    text: &[char],
    item: &CascadeItem,
    check: &dyn Fn() -> bool,
) -> Result<Vec<VariationIssue>, TextError> {
    let mut issues = Vec::new();
    for at in item.start as usize..item.end as usize {
        cancelled(check)?;
        if !mo_unicode::properties(text[at]).variation_selector {
            continue;
        }
        let base = at.checked_sub(1).filter(|&i| {
            i >= item.start as usize && !mo_unicode::properties(text[i]).variation_selector
        });
        let outcome = if let Some(base) = base {
            font.query_one(
                FontCharacter {
                    codepoint: text[base] as u32,
                    variation_selector: Some(text[at] as u32),
                },
                check,
            )?
        } else {
            CoverageOutcome::UnsupportedVariation
        };
        if !matches!(outcome, CoverageOutcome::Mapped { .. }) {
            issues.push(VariationIssue {
                selector_offset: at as u32,
                base_offset: base.map(|v| v as u32),
                outcome,
            });
        }
    }
    Ok(issues)
}
