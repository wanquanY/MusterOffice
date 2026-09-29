//! Logical paragraph preparation, not line fitting or glyph painting.
mod language;
#[cfg(test)]
mod language_tests;
mod scripts;
#[cfg(test)]
mod tests;
mod types;
use crate::{TextError, cancelled};
use mo_unicode::{
    TextBoundary, UnicodeLimits,
    bidi::{self, BidiClass, BidiError, BidiLimits, BidiParagraphRequest},
    script::{self, Script, ScriptSet},
};
pub use types::*;
struct Cluster {
    start: TextBoundary,
    end: TextBoundary,
    first: char,
    style: u32,
    level: u8,
    kind: TextItemKind,
    scripts: Option<ScriptSet>,
    preferred: Option<Script>,
    resolved: Option<Script>,
    ambiguous: bool,
    language_script: Option<Script>,
}
fn kind(c: char) -> TextItemKind {
    use BidiClass::*;
    use TextItemKind::*;
    match bidi::bidi_class(c) {
        B => ParagraphBreak,
        S => {
            if c == '\t' {
                Tab
            } else {
                LineBreak
            }
        }
        Lre | Lro | Rle | Rlo | Pdf | Lri | Rli | Fsi | Pdi => BidiControl,
        _ if matches!(c, '\u{2028}' | '\u{c}') => LineBreak,
        _ => Text,
    }
}
pub fn itemize(
    request: &ItemizationRequest,
    limits: ItemizationLimits,
    check: &dyn Fn() -> bool,
) -> Result<ItemizationResult, TextError> {
    itemize_with_languages(request, &[], limits, check)
}

/// Optional authored language runs refine otherwise ambiguous Script_Extensions.
/// They never override a script established by text, split a grapheme, or use a
/// system locale. Empty `languages` retains the original language-free policy.
pub fn itemize_with_languages(
    request: &ItemizationRequest,
    languages: &[LanguageSpan<'_>],
    limits: ItemizationLimits,
    check: &dyn Fn() -> bool,
) -> Result<ItemizationResult, TextError> {
    cancelled(check)?;
    if request.spans.len() > limits.max_styles {
        return Err(TextError::Limit("itemization style spans"));
    }
    let segmentation = mo_unicode::segment(&request.text, UnicodeLimits::default(), check)
        .map_err(|e| match e {
            mo_unicode::UnicodeError::Cancelled => TextError::Cancelled,
            mo_unicode::UnicodeError::Limit(m) => TextError::Limit(m),
        })?;
    let count = segmentation.boundaries.last().unwrap().scalar_offset;
    let language_runs = language::prepare(languages, count, limits.max_items, check)?;
    let mut previous = 0;
    for span in &request.spans {
        cancelled(check)?;
        if span.end <= previous
            || segmentation
                .boundaries
                .binary_search_by_key(&span.end, |b| b.scalar_offset)
                .is_err()
        {
            return Err(TextError::Invalid(
                "contiguous grapheme-aligned style spans",
            ));
        }
        previous = span.end;
    }
    if previous != count || (count == 0 && !request.spans.is_empty()) {
        return Err(TextError::Invalid("style spans must cover paragraph"));
    }
    let bidi = bidi::analyze_paragraph(
        &BidiParagraphRequest {
            text: request.text.clone(),
            direction: request.direction,
            line_ends: vec![],
        },
        BidiLimits::default(),
        check,
    )
    .map_err(|e| match e {
        BidiError::Cancelled => TextError::Cancelled,
        BidiError::Invalid(m) => TextError::Invalid(m),
        BidiError::Limit(m) => TextError::Limit(m),
    })?;
    let mut clusters = Vec::new();
    let mut notices = Vec::new();
    let mut span = 0;
    let mut language_at = 0;
    let mut scopes = vec![Vec::new()];
    let mut scope_stack = vec![0];
    for bounds in segmentation.boundaries.windows(2) {
        cancelled(check)?;
        let (start, end) = (&bounds[0], &bounds[1]);
        while start.scalar_offset >= request.spans[span].end {
            span += 1;
        }
        let text = &request.text[start.utf8_offset as usize..end.utf8_offset as usize];
        let first = text.chars().next().unwrap();
        let class = bidi::bidi_class(first);
        let kind = kind(first);
        let levels =
            &bidi.resolved_levels[start.scalar_offset as usize..end.scalar_offset as usize];
        let level = levels
            .iter()
            .copied()
            .flatten()
            .next()
            .unwrap_or(bidi.paragraph_level);
        if levels.iter().flatten().any(|&other| other != level) {
            notices.push(ItemizationNotice {
                start: start.scalar_offset,
                end: end.scalar_offset,
                kind: ItemizationNoticeKind::MixedLevelCluster,
            });
        }
        // The first explicit script in an EGC governs all marks. SCX is then
        // used as a contextual constraint; disjoint marks remain attached.
        let mut anchor: Option<(Script, ScriptSet)> = None;
        let mut weak = None;
        for c in text.chars() {
            cancelled(check)?;
            let primary = script::script(c);
            let extensions = script::script_extensions(c);
            if !primary.is_contextual() {
                if let Some((_, set)) = anchor {
                    if set.intersection(extensions).is_empty() {
                        notices.push(ItemizationNotice {
                            start: start.scalar_offset,
                            end: end.scalar_offset,
                            kind: ItemizationNoticeKind::MixedScriptCluster,
                        });
                        break;
                    }
                } else {
                    anchor = Some((primary, extensions));
                }
            }
            if weak.is_none() && !extensions.contextual() {
                weak = Some(extensions);
            }
        }
        let (preferred, scripts) = match anchor {
            Some((primary, set)) => (Some(primary), Some(set)),
            None => (None, weak),
        };
        let index = clusters.len();
        clusters.push(Cluster {
            start: start.clone(),
            end: end.clone(),
            first,
            style: request.spans[span].style,
            level,
            kind,
            scripts,
            preferred,
            resolved: None,
            ambiguous: false,
            language_script: language::for_cluster(
                &language_runs,
                &mut language_at,
                start.scalar_offset,
                end.scalar_offset,
                check,
            )?,
        });
        if class == BidiClass::Pdi && scope_stack.len() > 1 {
            scope_stack.pop();
        }
        scopes[*scope_stack.last().unwrap()].push(index);
        if matches!(class, BidiClass::Lri | BidiClass::Rli | BidiClass::Fsi) {
            scope_stack.push(scopes.len());
            scopes.push(Vec::new());
        }
    }
    scripts::resolve(&mut clusters, &scopes, check)?;
    let mut items: Vec<TextItem> = Vec::new();
    for c in clusters {
        cancelled(check)?;
        if c.ambiguous {
            notices.push(ItemizationNotice {
                start: c.start.scalar_offset,
                end: c.end.scalar_offset,
                kind: ItemizationNoticeKind::AmbiguousScript,
            });
        }
        if notices.len() > limits.max_notices {
            return Err(TextError::Limit("itemization notices"));
        }
        let script = c.resolved.unwrap_or_else(Script::common);
        if let Some(last) = items.last_mut()
            && last.kind == TextItemKind::Text
            && c.kind == TextItemKind::Text
            && last.style == c.style
            && last.level == c.level
            && last.script == script
        {
            last.end = c.end;
            continue;
        }
        if items.len() >= limits.max_items {
            return Err(TextError::Limit("itemization output items"));
        }
        items.push(TextItem {
            start: c.start,
            end: c.end,
            style: c.style,
            script,
            level: c.level,
            kind: c.kind,
        });
    }
    Ok(ItemizationResult {
        profile: if languages.is_empty() {
            "unicode18-script-bidi-grapheme-items-v1"
        } else {
            "unicode18-script-bidi-grapheme-language-items-v2"
        }
        .into(),
        bidi,
        items,
        notices,
    })
}
