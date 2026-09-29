//! Width-driven horizontal layout. Reshapes each candidate in actual line
//! context; no monotonic-width assumption, implicit fonts or source edits.
mod hanging;
#[cfg(test)]
mod hanging_tests;
#[cfg(test)]
mod tests;
mod types;
#[cfg(test)]
mod wrapping_tests;
use crate::{
    cascade::{context::Context, *},
    fallback::*,
    geometry::{self, number::Position},
    itemize::*,
    lines::{
        self,
        plan::{self, LinePlan},
    },
    *,
};
use mo_unicode::{
    TextSegmentation, UnicodeLimits,
    bidi::{BidiLimits, BidiLine, BidiParagraphResult, ResolvedParagraph},
    line_break::{self, BreakKind},
};
pub use types::*;
const MAX_CANDIDATES: u32 = 4096;
const MAX_CANDIDATE_SCALARS: u32 = 2_097_152;
struct Candidate {
    plan: LinePlan,
    bidi: BidiLine,
    fallback: FallbackResult,
    fits: bool,
    hanging: Option<HangingLineEnd>,
}
struct Search<'a, 'b> {
    q: &'a FlowInput<'a>,
    original: &'a ItemizationResult,
    segmentation: &'a TextSegmentation,
    bidi: &'a ResolvedParagraph,
    context: Context<'b>,
    work: FlowWork,
}
impl Search<'_, '_> {
    fn candidate(
        &mut self,
        start: u32,
        end: u32,
        backend: &mut dyn backend::TextBackend,
        check: &dyn Fn() -> bool,
    ) -> Result<Result<Candidate, Vec<FlowIssue>>, TextError> {
        cancelled(check)?;
        if self.work.evaluated_candidates >= MAX_CANDIDATES
            || u64::from(self.work.candidate_scalars) + u64::from(end - start)
                > u64::from(MAX_CANDIDATE_SCALARS)
        {
            return Err(TextError::Limit("line fitting candidate work"));
        }
        self.work.evaluated_candidates += 1;
        self.work.candidate_scalars += end - start;
        let bidi = self
            .bidi
            .line(start, end, check)
            .map_err(lines::bidi_error)?;
        let plan = plan::plan(
            self.q.paragraph,
            self.original,
            self.segmentation,
            std::slice::from_ref(&bidi),
            self.bidi.paragraph_level(),
            check,
        )?;
        let mut issues: Vec<_> = plan
            .notices
            .iter()
            .filter(|n| {
                n.kind == ItemizationNoticeKind::MixedLevelCluster && n.start < end && start < n.end
            })
            .map(|n| FlowIssue::MixedLevelCluster {
                start: n.start,
                end: n.end,
            })
            .collect();
        let fallback = shape_prepared(
            &mut self.context,
            &plan.cascade_items,
            backend,
            limits(),
            check,
            &plan.scopes,
        )?;
        for f in fallback.items.iter().flat_map(|i| &i.fragments) {
            if let FontFragment::Unresolved { start, end } = f {
                issues.push(FlowIssue::UnresolvedFont {
                    start: *start,
                    end: *end,
                });
            }
        }
        if !issues.is_empty() {
            return Ok(Err(issues));
        }
        let order = geometry::order::order(
            &plan.indices,
            &plan.items,
            &fallback,
            &plan.lines[0],
            &bidi,
            self.q.styles,
            check,
        )?;
        let terminal = hanging::terminal(self.q, &plan, self.segmentation);
        let bounds = geometry::order::pen_bounds(
            &fallback,
            &order,
            terminal
                .as_ref()
                .map(|(from, to)| from.scalar_offset..to.scalar_offset),
            check,
        )?;
        let width = self.q.widths.at(start);
        let fits = bounds.min >= Position::ZERO && bounds.max <= width;
        let hanging = if !fits {
            match (terminal, bounds.body) {
                (Some((start, end)), Some((min, max)))
                    if min >= Position::ZERO && max.checked_sub(min)? <= width =>
                {
                    Some(HangingLineEnd {
                        start,
                        end,
                        body_pen_min: min,
                        body_pen_max: max,
                    })
                }
                _ => None,
            }
        } else {
            None
        };
        Ok(Ok(Candidate {
            plan,
            bidi,
            fallback,
            fits: fits || hanging.is_some(),
            hanging,
        }))
    }
    fn work(&self) -> FlowWork {
        FlowWork {
            verified_faces: self.context.fonts().len() as u32,
            shaping_runs: self.context.shaping_runs,
            component_calls: self.context.component_calls,
            context_scalars: self.context.context_scalars,
            probed_glyphs: self.context.probed_glyphs,
            ..self.work.clone()
        }
    }
}
fn limits() -> FallbackLimits {
    FallbackLimits {
        cascade: CascadeLimits {
            max_items: 4096,
            ..Default::default()
        },
        ..Default::default()
    }
}
fn empty_fallback() -> FallbackResult {
    FallbackResult {
        profile: "grapheme-cluster-probe-and-reshape-ucd18-hb14.5-cmap14-v1".into(),
        items: vec![],
        verified_faces: 0,
        shaping_runs: 0,
        component_calls: 0,
        context_scalars: 0,
        probed_glyphs: 0,
    }
}
struct Selected {
    bidi: Vec<BidiLine>,
    plan: LinePlan,
    fallback: FallbackResult,
    glyphs: usize,
    fragments: usize,
}
impl Selected {
    fn new(notices: Vec<ItemizationNotice>) -> Self {
        Self {
            bidi: vec![],
            plan: LinePlan {
                items: vec![],
                notices,
                lines: vec![],
                cascade_items: vec![],
                indices: vec![],
                scopes: vec![],
            },
            fallback: empty_fallback(),
            glyphs: 0,
            fragments: 0,
        }
    }
    fn push(&mut self, mut c: Candidate, check: &dyn Fn() -> bool) -> Result<(), TextError> {
        cancelled(check)?;
        self.fragments += c
            .fallback
            .items
            .iter()
            .map(|i| i.fragments.len())
            .sum::<usize>();
        self.glyphs += c
            .fallback
            .items
            .iter()
            .flat_map(|i| &i.fragments)
            .map(|f| match f {
                FontFragment::Selected { shaped, .. } => shaped.runs[0].glyphs.len(),
                _ => 0,
            })
            .sum::<usize>();
        if self.plan.items.len() + c.plan.items.len() > 4096
            || self.bidi.len() >= 4096
            || self.fragments > limits().max_fragments
            || self.glyphs > limits().cascade.max_selected_glyphs
        {
            return Err(TextError::Limit("selected paragraph layout size"));
        }
        let items = self.plan.items.len() as u32;
        let fallback = self.fallback.items.len() as u32;
        for mut line in c.plan.lines {
            line.item_start += items;
            line.item_end += items;
            line.fallback_start += fallback;
            line.fallback_end += fallback;
            self.plan.lines.push(line);
        }
        self.plan
            .indices
            .extend(c.plan.indices.into_iter().map(|i| i + items));
        self.plan.items.append(&mut c.plan.items);
        // Original notices are present once; any new mixed-level notice would
        // have prevented this candidate from being selected.
        self.fallback.items.append(&mut c.fallback.items);
        self.bidi.push(c.bidi);
        Ok(())
    }
}
pub fn layout_paragraph(
    q: &ParagraphLayoutRequest,
    bundle: &[u8],
    backend: &mut dyn backend::TextBackend,
    check: &dyn Fn() -> bool,
) -> Result<ParagraphLayoutResult, TextError> {
    layout_with(
        q,
        resources::ResourceInput::Bundle(bundle),
        backend,
        check,
        geometry::evaluate,
    )
}
pub(crate) fn layout_with(
    q: &ParagraphLayoutRequest,
    input: resources::ResourceInput<'_, '_>,
    backend: &mut dyn backend::TextBackend,
    check: &dyn Fn() -> bool,
    finish: impl FnOnce(
        &geometry::LineGeometryRequest,
        lines::LineShapeResult,
        &[mo_font::VerifiedFont<'_>],
        &[usize],
        &mut dyn backend::TextBackend,
        &dyn Fn() -> bool,
    ) -> Result<geometry::LineGeometryResult, TextError>,
) -> Result<ParagraphLayoutResult, TextError> {
    layout_flow(&q.into(), input, backend, check, finish)
}
pub(crate) fn layout_flow(
    q: &FlowInput<'_>,
    input: resources::ResourceInput<'_, '_>,
    backend: &mut dyn backend::TextBackend,
    check: &dyn Fn() -> bool,
    finish: impl FnOnce(
        &geometry::LineGeometryRequest,
        lines::LineShapeResult,
        &[mo_font::VerifiedFont<'_>],
        &[usize],
        &mut dyn backend::TextBackend,
        &dyn Fn() -> bool,
    ) -> Result<geometry::LineGeometryResult, TextError>,
) -> Result<ParagraphLayoutResult, TextError> {
    cancelled(check)?;
    if q.widths.first <= Position::ZERO || q.widths.rest <= Position::ZERO {
        return Err(TextError::Invalid("positive paragraph width"));
    }
    let geometry_request = geometry::LineGeometryRequest {
        shaping: lines::LineShapeRequest {
            paragraph: q.paragraph.clone(),
            line_ends: vec![],
        },
        styles: q.styles.to_vec(),
        strut_style: q.strut_style,
        spacing: q.spacing.clone(),
    };
    geometry::validate(&geometry_request, check)?;
    let original =
        paragraph::prepare_items_with(q.paragraph, input.bundle_length(), check, &|a, b| {
            geometry::styles_equal(q.styles, &q.spacing, a, b)
        })?;
    let segmentation = mo_unicode::segment(&q.paragraph.text, UnicodeLimits::default(), check)
        .map_err(cascade::unicode_error)?;
    let bidi = ResolvedParagraph::prepare(
        &q.paragraph.text,
        q.paragraph.direction,
        BidiLimits::default(),
        check,
    )
    .map_err(lines::bidi_error)?;
    let breaks =
        line_break::analyze_line_breaks(&q.paragraph.text, UnicodeLimits::default(), check)
            .map_err(cascade::unicode_error)?;
    let cascade = CascadeRequest {
        text: q.paragraph.text.clone(),
        fonts: q.paragraph.fonts.clone(),
        items: vec![],
    };
    let context = Context::prepare_using(
        &cascade,
        input,
        limits().cascade,
        check,
        |fonts, bindings| paragraph::validate_styles(q.paragraph, fonts, bindings, check),
    )?;
    let mut search = Search {
        q,
        original: &original,
        segmentation: &segmentation,
        bidi: &bidi,
        context,
        work: FlowWork::default(),
    };
    let mut result = ParagraphLayoutResult {
        profile: "unicode18-egc-farthest-fit-preserve-spaces-q32-v1-draft".into(),
        breaks,
        suppressed_grapheme_breaks: vec![],
        decisions: vec![],
        geometry: None,
        issues: vec![],
        work: FlowWork::default(),
    };
    for (i, c) in q.paragraph.text.chars().enumerate() {
        cancelled(check)?;
        let issue = match c {
            '\t' => Some(FlowIssue::Tab { scalar: i as u32 }),
            '\u{ad}' => Some(FlowIssue::ConditionalHyphen { scalar: i as u32 }),
            '\u{fffc}' => Some(FlowIssue::ContingentObject { scalar: i as u32 }),
            _ => None,
        };
        if let Some(issue) = issue {
            result.issues.push(issue);
        }
    }
    let mut opportunities = Vec::new();
    for b in &result.breaks.opportunities {
        cancelled(check)?;
        if q.wrapping == LineWrapping::NoWrap && b.kind != BreakKind::Mandatory {
            continue;
        }
        if search
            .context
            .boundaries
            .contains(&b.boundary.scalar_offset)
        {
            opportunities.push((b.boundary.scalar_offset, b.kind));
        } else if b.kind == BreakKind::Mandatory {
            return Err(TextError::Invalid("mandatory break splits a grapheme"));
        } else {
            result.suppressed_grapheme_breaks.push(b.boundary.clone());
        }
    }
    if !result.issues.is_empty() {
        result.work = search.work();
        return Ok(result);
    }
    let end = result.breaks.end.scalar_offset;
    if end == 0 {
        opportunities.push((0, BreakKind::Mandatory));
    }
    let mut selected = Selected::new(original.notices.clone());
    let mut start = 0;
    let mut next = 0;
    loop {
        cancelled(check)?;
        let stop = (next..opportunities.len())
            .find(|&i| opportunities[i].1 == BreakKind::Mandatory)
            .expect("LB3 supplies mandatory end");
        let first = opportunities[next].0;
        let mut chosen = None;
        let mut earliest = None;
        // Descending search proves furthest fit without assuming contextual
        // substitution, kerning or signed advances make widths monotonic.
        for i in (next..=stop).rev() {
            let c = match search.candidate(start, opportunities[i].0, backend, check)? {
                Ok(c) => c,
                Err(issues) => {
                    result.issues = issues;
                    result.decisions.clear();
                    result.work = search.work();
                    return Ok(result);
                }
            };
            if c.fits {
                chosen = Some((c, false));
                break;
            }
            if i == next {
                earliest = Some(c);
            }
        }
        if chosen.is_none()
            && q.wrapping == LineWrapping::Wrap
            && matches!(q.overflow, OverflowPolicy::EmergencyGrapheme)
        {
            let lo = segmentation
                .boundaries
                .partition_point(|b| b.scalar_offset <= start);
            let hi = segmentation
                .boundaries
                .partition_point(|b| b.scalar_offset < first);
            for boundary in segmentation.boundaries[lo..hi].iter().rev() {
                let c = match search.candidate(start, boundary.scalar_offset, backend, check)? {
                    Ok(c) => c,
                    Err(issues) => {
                        result.issues = issues;
                        result.decisions.clear();
                        result.work = search.work();
                        return Ok(result);
                    }
                };
                if c.fits {
                    chosen = Some((c, true));
                    break;
                }
                if boundary.scalar_offset == segmentation.boundaries[lo].scalar_offset {
                    chosen = Some((c, true));
                }
            }
        }
        let (candidate, emergency) =
            chosen.unwrap_or_else(|| (earliest.expect("first legal candidate evaluated"), false));
        let to = candidate.bidi.end.clone();
        let overflows = !candidate.fits;
        let hanging = candidate.hanging.clone();
        selected.push(candidate, check)?;
        result.decisions.push(LineDecision {
            end: to.clone(),
            emergency,
            overflows,
            hanging,
        });
        start = to.scalar_offset;
        if start == end {
            break;
        }
        next = opportunities.partition_point(|b| b.0 <= start);
    }
    if end > 0
        && original
            .items
            .last()
            .is_some_and(|i| i.kind == TextItemKind::LineBreak)
    {
        let empty = search
            .candidate(end, end, backend, check)?
            .map_err(|_| TextError::Invalid("empty trailing line prerequisites"))?;
        result.decisions.push(LineDecision {
            end: empty.bidi.end.clone(),
            emergency: false,
            overflows: false,
            hanging: None,
        });
        selected.push(empty, check)?;
    }
    result.work = search.work();
    selected.fallback.verified_faces = result.work.verified_faces;
    selected.fallback.shaping_runs = result.work.shaping_runs;
    selected.fallback.component_calls = result.work.component_calls;
    selected.fallback.context_scalars = result.work.context_scalars;
    selected.fallback.probed_glyphs = result.work.probed_glyphs;
    let paragraph_bidi = BidiParagraphResult {
        profile: original.bidi.profile.clone(),
        paragraph_level: bidi.paragraph_level(),
        resolved_levels: bidi.resolved_levels(),
        lines: selected.bidi,
    };
    let shaped = selected.plan.finish(paragraph_bidi, selected.fallback);
    result.geometry = Some(finish(
        &geometry_request,
        shaped,
        search.context.fonts(),
        search.context.bindings(),
        backend,
        check,
    )?);
    cancelled(check)?;
    Ok(result)
}
