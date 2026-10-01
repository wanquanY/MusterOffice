//! Domain-independent paragraph compilation shared by native shape/table runs
//! and chart rich/generated text. Font declarations are resolved by the owner.
use super::*;
use mo_presentation_source::{
    PptxError,
    source::text::{
        cascade::CascadedCharacterStyle,
        fonts::{NativeFontSlot, TypefaceLimits, TypefaceOutcome},
    },
};
use mo_text::itemize::{self, *};
use mo_unicode::{UnicodeLimits, bidi::ParagraphDirection};
use std::collections::BTreeMap;

pub(crate) struct Segment<'a> {
    pub text: &'a str,
    pub style: &'a CascadedCharacterStyle,
}
struct SegmentRange {
    start: u32,
    end: u32,
    run: u32,
}
pub(crate) type FontResolver<'a> = dyn Fn(
        Option<u32>,
        NativeFontSlot,
        Option<&str>,
        TypefaceLimits,
        &dyn Fn() -> bool,
    ) -> Result<TypefaceOutcome, PptxError>
    + 'a;

pub(crate) fn paragraph(
    paragraph: u32,
    direction: ParagraphDirection,
    segments: &[Segment<'_>],
    end_style: &CascadedCharacterStyle,
    resolve: &FontResolver<'_>,
    budget: &mut budget::Budget,
    check: &dyn Fn() -> bool,
) -> Result<Result<ParagraphComputationPlan, SourceTextIssue>, SourceTextError> {
    budget.charge(2048)?;
    let mut plan = ParagraphComputationPlan {
        text: String::new(),
        direction,
        fonts: vec![],
        font_spans: vec![],
        spans: vec![],
        styles: vec![],
        geometry: vec![],
        baseline_conversion_error: mo_geometry::Fixed::ZERO,
        tracking_conversion_error: mo_geometry::Fixed::ZERO,
        end_style: 0,
    };
    let mut ranges = vec![];
    let mut count = 0;
    for (i, segment) in segments.iter().enumerate() {
        cancel(check)?;
        if let Some(issue) = style::audit(segment.style, paragraph, Some(i as u32)) {
            return Ok(Err(issue));
        }
        let text = segment.text;
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
        ranges.push(SegmentRange {
            start,
            end: count,
            run: i as u32,
        });
    }
    if let Some(issue) = style::audit(end_style, paragraph, None) {
        return Ok(Err(issue));
    }
    let languages: Vec<_> = ranges
        .iter()
        .filter(|range| range.end > range.start)
        .map(|range| LanguageSpan {
            end: range.end,
            language: segments[range.run as usize]
                .style
                .attributes
                .language
                .as_deref()
                .unwrap_or("und"),
        })
        .collect();
    let items = itemize::itemize_with_languages(
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
        &languages,
        ItemizationLimits::default(),
        check,
    )?;
    if let Some(notice) = items.notices.into_iter().next() {
        return Ok(Err(SourceTextIssue::Itemization { paragraph, notice }));
    }
    let mut compiler = Compiler {
        paragraph,
        segments,
        end_style,
        resolve,
        ranges,
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
            while compiler.ranges[run_at].end <= start {
                run_at += 1;
            }
            let source_range = &compiler.ranges[run_at];
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
    // Content segments may cut an EGC only when effective styles coalesce.
    // Domain provenance is retained separately from computation spans.
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
    segments: &'a [Segment<'a>],
    end_style: &'a CascadedCharacterStyle,
    resolve: &'a FontResolver<'a>,
    ranges: Vec<SegmentRange>,
    paragraph: u32,
    budget: &'a mut budget::Budget,
    check: &'a dyn Fn() -> bool,
    cache: BTreeMap<(Option<u32>, String), u32>,
    plan: ParagraphComputationPlan,
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
        let style = match run {
            Some(r) => self.segments[r as usize].style,
            None => self.end_style,
        };
        let Some((slot, theme_script)) = super::script::select(
            script,
            style.attributes.language.as_deref(),
            style.attributes.alternative_language.as_deref(),
        ) else {
            return Ok(Err(SourceTextIssue::Script {
                paragraph: self.paragraph,
                start,
                end,
                script: script.into(),
            }));
        };
        let font = match (self.resolve)(
            run,
            slot,
            theme_script.as_deref(),
            self.budget.limits.typeface,
            self.check,
        )? {
            TypefaceOutcome::Named { font } => *font,
            TypefaceOutcome::Unresolved { reason } => {
                return Ok(Err(SourceTextIssue::Typeface {
                    paragraph: self.paragraph,
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
                paragraph: self.paragraph,
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
