use super::*;
use mo_presentation_source::source::{
    SourceRun, SourceRunKind,
    text::{cascade::*, fonts::*},
};
use mo_text::itemize::{self, *};
use mo_unicode::{UnicodeLimits, bidi::ParagraphDirection};
use std::collections::BTreeMap;

type PreparedParagraph = Result<SourceParagraphPlan, SourceTextIssue>;

pub(super) fn paragraph(
    index: &SourceIndex,
    source: &CascadedText,
    paragraph: u32,
    runs: &[SourceRun],
    budget: &mut budget::Budget,
    check: &dyn Fn() -> bool,
) -> Result<PreparedParagraph, SourceTextError> {
    budget.charge(2048)?;
    let local_paragraph = paragraph;
    let paragraph = source
        .native_paragraph(local_paragraph)
        .ok_or(SourceTextError::Invalid("source paragraph scope"))?;
    let p = &source.paragraphs[local_paragraph as usize];
    if p.runs.len() != runs.len() {
        return Err(SourceTextError::Invalid("source run count"));
    }
    let direction = match p.attributes.right_to_left {
        Some(false) => ParagraphDirection::LeftToRight,
        Some(true) => ParagraphDirection::RightToLeft,
        None => return Err(SourceTextError::Invalid("cascaded paragraph direction")),
    };
    let mut plan = SourceParagraphPlan {
        profile: super::PROFILE.into(),
        source_ordinal: p.source_ordinal,
        text: String::new(),
        direction,
        sources: vec![],
        fonts: vec![],
        font_spans: vec![],
        spans: vec![],
        styles: vec![],
        geometry: vec![],
        baseline_conversion_error: mo_geometry::Fixed::ZERO,
        tracking_conversion_error: mo_geometry::Fixed::ZERO,
        end_style: 0,
    };
    let mut count = 0;
    for (i, (r, native)) in p.runs.iter().zip(runs).enumerate() {
        cancel(check)?;
        if r.run as usize != i || r.kind != native.kind {
            return Err(SourceTextError::Invalid("source run binding"));
        }
        if native.kind == SourceRunKind::Field {
            return Ok(Err(SourceTextIssue::Field {
                paragraph,
                run: i as u32,
                source_ordinal: r.source_ordinal,
            }));
        }
        if let Some(issue) = style::audit(&r.style, paragraph, Some(i as u32)) {
            return Ok(Err(issue));
        }
        let text = if native.kind == SourceRunKind::Break {
            "\u{2028}"
        } else {
            native.text.as_str()
        };
        if plan.text.len().saturating_add(text.len()) > 262_144 {
            return Err(SourceTextError::Limit("source paragraph bytes"));
        }
        if text.contains(['\n', '\r', '\u{2029}']) {
            return Ok(Err(SourceTextIssue::ParagraphControl {
                paragraph,
                run: i as u32,
            }));
        }
        let start = count;
        count += text.chars().count() as u32;
        if count > 65_536 {
            return Err(SourceTextError::Limit("source paragraph scalars"));
        }
        budget.charge(128 + text.len() * 4)?;
        plan.text.push_str(text);
        plan.sources.push(SourceScalarRange {
            start,
            end: count,
            run: i as u32,
            source_ordinal: r.source_ordinal,
            kind: native.kind,
        });
    }
    if let Some(issue) = style::audit(&p.end_style, paragraph, None) {
        return Ok(Err(issue));
    }
    let items = itemize::itemize(
        &ItemizationRequest {
            text: plan.text.clone(),
            direction,
            spans: if count == 0 {
                vec![]
            } else {
                vec![StyleSpan {
                    end: count,
                    style: 0,
                }]
            },
        },
        ItemizationLimits::default(),
        check,
    )?;
    if let Some(notice) = items.notices.into_iter().next() {
        return Ok(Err(SourceTextIssue::Itemization { paragraph, notice }));
    }
    let mut compiler = Compiler {
        index,
        source,
        paragraph: local_paragraph,
        budget,
        check,
        cache: BTreeMap::new(),
        plan,
    };
    let mut run_at = 0;
    for item in items.items {
        cancel(check)?;
        let mut start = item.start.scalar_offset;
        while start < item.end.scalar_offset {
            cancel(check)?;
            while compiler.plan.sources[run_at].end <= start {
                run_at += 1;
            }
            let source_range = &compiler.plan.sources[run_at];
            let end = source_range.end.min(item.end.scalar_offset);
            let binding =
                match compiler.bind(Some(source_range.run), item.script.tag(), start, end)? {
                    Ok(binding) => binding,
                    Err(issue) => return Ok(Err(issue)),
                };
            let style = compiler.plan.fonts[binding as usize].style;
            compiler.budget.charge(128)?;
            compiler.plan.font_spans.push(SourceFontSpan {
                start,
                end,
                binding,
            });
            if compiler.plan.font_spans.len() > 4096 {
                return Err(SourceTextError::Limit("source font spans"));
            }
            if let Some(previous) = compiler.plan.spans.last_mut().filter(|s| s.style == style) {
                previous.end = end;
            } else {
                if compiler.plan.spans.len() >= 256 {
                    return Err(SourceTextError::Limit("source computation spans"));
                }
                compiler.plan.spans.push(StyleSpan { end, style });
            }
            start = end;
        }
    }
    // Source run boundaries may cut an EGC only when effective computation
    // styles coalesce. Their separate XML provenance is never discarded.
    let boundaries = mo_unicode::segment(&compiler.plan.text, UnicodeLimits::default(), check)
        .map_err(|e| match e {
            mo_unicode::UnicodeError::Cancelled => SourceTextError::Cancelled,
            mo_unicode::UnicodeError::Limit(m) => SourceTextError::Limit(m),
        })?
        .boundaries;
    for span in &compiler.plan.spans {
        cancel(check)?;
        if boundaries
            .binary_search_by_key(&span.end, |b| b.scalar_offset)
            .is_err()
        {
            return Ok(Err(SourceTextIssue::GraphemeStyleConflict {
                paragraph,
                boundary: span.end,
            }));
        }
    }
    // The draft's empty-line strut explicitly uses the insertion style's Latin
    // slot. It is independent of preceding content/script and host locale.
    let end = match compiler.bind(None, "Zyyy", count, count)? {
        Ok(binding) => compiler.plan.fonts[binding as usize].style,
        Err(issue) => return Ok(Err(issue)),
    };
    compiler.plan.end_style = end;
    cancel(check)?;
    Ok(Ok(compiler.plan))
}

struct Compiler<'a> {
    index: &'a SourceIndex,
    source: &'a CascadedText,
    paragraph: u32,
    budget: &'a mut budget::Budget,
    check: &'a dyn Fn() -> bool,
    cache: BTreeMap<(Option<u32>, String), u32>,
    plan: SourceParagraphPlan,
}
impl Compiler<'_> {
    fn bind(
        &mut self,
        run: Option<u32>,
        script: &str,
        start: u32,
        end: u32,
    ) -> Result<Result<u32, SourceTextIssue>, SourceTextError> {
        cancel(self.check)?;
        let key = (run, script.to_owned());
        if let Some(&id) = self.cache.get(&key) {
            return Ok(Ok(id));
        }
        let p = &self.source.paragraphs[self.paragraph as usize];
        let style = match run {
            Some(r) => &p.runs[r as usize].style,
            None => &p.end_style,
        };
        let Some((slot, theme_script)) = super::script::select(
            script,
            style.attributes.language.as_deref(),
            style.attributes.alternative_language.as_deref(),
        ) else {
            return Ok(Err(SourceTextIssue::Script {
                paragraph: self.source.paragraph_start + self.paragraph,
                start,
                end,
                script: script.into(),
            }));
        };
        let font = match mo_presentation_source::source::text::fonts::resolve(
            self.index,
            self.source,
            self.paragraph,
            run,
            slot,
            theme_script.as_deref(),
            self.budget.limits.typeface,
            self.check,
        )? {
            TypefaceOutcome::Named { font } => *font,
            TypefaceOutcome::Unresolved { reason } => {
                return Ok(Err(SourceTextIssue::Typeface {
                    paragraph: self.source.paragraph_start + self.paragraph,
                    run,
                    slot,
                    reason,
                }));
            }
        };
        self.budget.font(&font)?;
        if [font.authored_font.as_ref(), font.theme_font.as_ref()]
            .into_iter()
            .flatten()
            .any(|f| f.charset == Some(2))
        {
            return Ok(Err(SourceTextIssue::SymbolFont {
                paragraph: self.source.paragraph_start + self.paragraph,
                run,
            }));
        }
        let (effective, geometry, tracking_error) = style::effective(style, &font)?;
        self.plan.tracking_conversion_error =
            self.plan.tracking_conversion_error.max(tracking_error);
        if matches!(
            geometry.baseline_shift,
            mo_text::geometry::BaselineShift::Q32 { .. }
        ) {
            self.plan.baseline_conversion_error = mo_geometry::Fixed::from_raw(1);
        }
        let style = if let Some(i) = self
            .plan
            .styles
            .iter()
            .zip(&self.plan.geometry)
            .position(|(s, g)| style::equal(s, &effective) && *g == geometry)
        {
            i as u32
        } else {
            if self.plan.styles.len() >= 256 {
                return Err(SourceTextError::Limit("source effective styles"));
            }
            self.plan.styles.push(effective);
            self.plan.geometry.push(geometry);
            (self.plan.styles.len() - 1) as u32
        };
        if self.plan.fonts.len() >= 4096 {
            return Err(SourceTextError::Limit("source font bindings"));
        }
        let binding = self.plan.fonts.len() as u32;
        self.plan.fonts.push(SourceFontBinding {
            run,
            script: script.into(),
            slot,
            theme_script,
            font,
            style,
        });
        self.cache.insert(key, binding);
        Ok(Ok(binding))
    }
}
