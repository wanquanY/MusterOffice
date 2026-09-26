//! Context-aware mixed-font resolution. Never slices and splices probe glyphs.
mod partition;
#[cfg(test)]
mod tests;
mod types;
use crate::{
    cascade::{context::Context, *},
    *,
};
use mo_font::VerifiedFont;
use partition::Partition;
use std::collections::BTreeMap;
pub use types::*;
struct Probe {
    bad: Vec<u32>,
    shaped: Option<ShapedText>,
    attempt: FontAttempt,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Segment {
    start: u32,
    end: u32,
    candidate: Option<usize>,
}
fn subitem(item: &CascadeItem, start: u32, end: u32) -> CascadeItem {
    let mut result = item.clone();
    result.start = start;
    result.end = end;
    result.beginning_of_text &= start == item.start;
    result.end_of_text &= end == item.end;
    result
}
pub fn shape_fallback(
    request: &CascadeRequest,
    bundle: &[u8],
    backend: &mut dyn backend::TextBackend,
    limits: FallbackLimits,
    check: &dyn Fn() -> bool,
) -> Result<FallbackResult, TextError> {
    shape_fallback_validating(
        request,
        resources::ResourceInput::Bundle(bundle),
        backend,
        limits,
        check,
        |_, _| Ok(()),
    )
}
pub(crate) fn shape_fallback_validating(
    request: &CascadeRequest,
    input: resources::ResourceInput<'_, '_>,
    backend: &mut dyn backend::TextBackend,
    limits: FallbackLimits,
    check: &dyn Fn() -> bool,
    validate_styles: impl FnOnce(&[VerifiedFont<'_>], &[usize]) -> Result<(), TextError>,
) -> Result<FallbackResult, TextError> {
    let mut context =
        Context::prepare_using(request, input, limits.cascade, check, validate_styles)?;
    shape_prepared(&mut context, &request.items, backend, limits, check, &[])
}
pub(crate) fn shape_prepared(
    context: &mut Context<'_>,
    request_items: &[CascadeItem],
    backend: &mut dyn backend::TextBackend,
    limits: FallbackLimits,
    check: &dyn Fn() -> bool,
    scopes: &[std::ops::Range<u32>],
) -> Result<FallbackResult, TextError> {
    if !scopes.is_empty() {
        if scopes.len() != request_items.len() {
            return Err(TextError::Invalid("one shaping context per item"));
        }
        for (scope, item) in scopes.iter().zip(request_items) {
            cancelled(check)?;
            if scope.start > item.start
                || scope.end < item.end
                || !context.boundaries.contains(&scope.start)
                || !context.boundaries.contains(&scope.end)
            {
                return Err(TextError::Invalid(
                    "grapheme-aligned shaping context must contain item",
                ));
            }
        }
    }
    let mut items = Vec::new();
    let mut fragments = 0;
    let mut selected_glyphs = 0;
    let mut rejections = 0;
    for (index, item) in request_items.iter().enumerate() {
        if let Some(scope) = scopes.get(index) {
            context.set_scope(scope.clone());
        }
        let result = resolve_item(item, context, backend, limits, check)?;
        fragments += result.fragments.len();
        rejections += result.reshape_rejections.len();
        if fragments > limits.max_fragments || rejections > limits.max_reshape_rejections {
            return Err(TextError::Limit("font fallback fragments or rejections"));
        }
        for fragment in &result.fragments {
            if let FontFragment::Selected { shaped, .. } = fragment {
                selected_glyphs += shaped.runs[0].glyphs.len();
            }
        }
        if selected_glyphs > limits.cascade.max_selected_glyphs {
            return Err(TextError::Limit("font cascade selected glyphs"));
        }
        items.push(result);
    }
    cancelled(check)?;
    Ok(FallbackResult {
        profile: "grapheme-cluster-probe-and-reshape-ucd18-hb14.5-cmap14-v1".into(),
        items,
        verified_faces: context.fonts().len() as u32,
        shaping_runs: context.shaping_runs,
        component_calls: context.component_calls,
        context_scalars: context.context_scalars,
        probed_glyphs: context.probed_glyphs,
    })
}
fn resolve_item(
    item: &CascadeItem,
    context: &mut Context<'_>,
    backend: &mut dyn backend::TextBackend,
    limits: FallbackLimits,
    check: &dyn Fn() -> bool,
) -> Result<FallbackItem, TextError> {
    let mut partition = Partition::new(item.start, item.end, &context.boundaries);
    let mut probes: Vec<Probe> = Vec::new();
    let mut rejections = Vec::new();
    let mut cache = BTreeMap::<(usize, u32, u32), ShapedText>::new();
    loop {
        cancelled(check)?;
        let mut segments = assign(&partition, &probes, &rejections, check)?;
        if (probes.is_empty() || segments.iter().any(|s| s.candidate.is_none()))
            && probes.len() < item.candidates.len()
        {
            let index = probes.len();
            let candidate = &item.candidates[index];
            let shaped = context.shape(candidate, vec![item.run(candidate)], backend, check)?;
            let issues = context.variations(candidate, item, check)?;
            let bad = partition.observe(&shaped.runs[0], &issues, check)?;
            let attempt = FontAttempt {
                candidate: index as u32,
                font: candidate.font,
                missing_glyph_clusters: shaped.runs[0].missing_glyph_clusters.clone(),
                variation_issues: issues,
            };
            probes.push(Probe {
                bad,
                shaped: Some(shaped),
                attempt,
            });
            continue;
        }
        if segments.len() > limits.max_fragments {
            return Err(TextError::Limit("font fallback fragments"));
        }
        // A whole original item needs no boundary reshape. The exact probe may
        // be reused; partial glyph arrays are never sliced out of it.
        if segments.len() == 1
            && let Some(candidate) = segments[0].candidate
            && let Some(shaped) = probes[candidate].shaped.take()
        {
            cache.insert((candidate, item.start, item.end), shaped);
        }
        let before = rejections.len();
        reshape(
            item,
            &segments,
            context,
            backend,
            &mut cache,
            &mut rejections,
            check,
        )?;
        if rejections.len() > limits.max_reshape_rejections {
            return Err(TextError::Limit("font fallback reshape rejections"));
        }
        if rejections.len() != before {
            continue;
        }
        let mut fragments = Vec::new();
        for segment in segments.drain(..) {
            cancelled(check)?;
            fragments.push(match segment.candidate {
                Some(candidate) => FontFragment::Selected {
                    start: segment.start,
                    end: segment.end,
                    font: item.candidates[candidate].font,
                    candidate: candidate as u32,
                    shaped: cache
                        .remove(&(candidate, segment.start, segment.end))
                        .expect("validated final reshape"),
                },
                None => FontFragment::Unresolved {
                    start: segment.start,
                    end: segment.end,
                },
            });
        }
        return Ok(FallbackItem {
            start: item.start,
            end: item.end,
            probes: probes.into_iter().map(|p| p.attempt).collect(),
            protected_boundaries: partition.protected(),
            reshape_rejections: rejections,
            fragments,
        });
    }
}
fn assign(
    partition: &Partition,
    probes: &[Probe],
    rejections: &[ReshapeRejection],
    check: &dyn Fn() -> bool,
) -> Result<Vec<Segment>, TextError> {
    let mut result: Vec<Segment> = Vec::new();
    for (lo, hi) in partition.regions() {
        cancelled(check)?;
        let (start, end) = (partition.points[lo], partition.points[hi]);
        let candidate = probes
            .iter()
            .enumerate()
            .find(|(i, p)| {
                p.bad[lo] == p.bad[hi]
                    && !rejections
                        .iter()
                        .any(|r| r.candidate as usize == *i && r.start < end && start < r.end)
            })
            .map(|(i, _)| i);
        if let Some(last) = result.last_mut()
            && last.candidate == candidate
        {
            last.end = end;
        } else {
            result.push(Segment {
                start,
                end,
                candidate,
            });
        }
    }
    Ok(result)
}
fn reshape(
    item: &CascadeItem,
    segments: &[Segment],
    context: &mut Context<'_>,
    backend: &mut dyn backend::TextBackend,
    cache: &mut BTreeMap<(usize, u32, u32), ShapedText>,
    rejections: &mut Vec<ReshapeRejection>,
    check: &dyn Fn() -> bool,
) -> Result<(), TextError> {
    let mut groups = BTreeMap::<usize, Vec<Segment>>::new();
    for &s in segments {
        cancelled(check)?;
        if let Some(c) = s.candidate
            && !cache.contains_key(&(c, s.start, s.end))
        {
            groups.entry(c).or_default().push(s);
        }
    }
    for (index, segments) in groups {
        let candidate = &item.candidates[index];
        // Both backend transports already support bounded run batches: upload
        // the same font once per batch while retaining each full text context.
        for batch in segments.chunks(256) {
            cancelled(check)?;
            let runs = batch
                .iter()
                .map(|s| subitem(item, s.start, s.end).run(candidate))
                .collect();
            let mut shaped = context.shape(candidate, runs, backend, check)?;
            let results = std::mem::take(&mut shaped.runs);
            for (segment, run) in batch.iter().zip(results) {
                cancelled(check)?;
                let issues = context.variations(
                    candidate,
                    &subitem(item, segment.start, segment.end),
                    check,
                )?;
                if run.missing_glyph_clusters.is_empty() && issues.is_empty() {
                    let mut output = shaped.clone();
                    output.runs = vec![run];
                    cache.insert((index, segment.start, segment.end), output);
                } else {
                    rejections.push(ReshapeRejection {
                        start: segment.start,
                        end: segment.end,
                        candidate: index as u32,
                        missing_glyph_clusters: run.missing_glyph_clusters,
                        variation_issues: issues,
                    });
                }
            }
        }
    }
    Ok(())
}
