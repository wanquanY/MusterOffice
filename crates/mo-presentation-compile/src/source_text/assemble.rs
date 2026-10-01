use super::*;
use mo_presentation_source::source::{SourceRun, SourceRunKind, text::cascade::CascadedText};
use mo_unicode::bidi::ParagraphDirection;

pub(super) fn paragraph(
    index: &SourceIndex,
    source: &CascadedText,
    paragraph: u32,
    runs: &[SourceRun],
    budget: &mut budget::Budget,
    check: &dyn Fn() -> bool,
) -> Result<Result<SourceParagraphPlan, SourceTextIssue>, SourceTextError> {
    let native_paragraph = source
        .native_paragraph(paragraph)
        .ok_or(SourceTextError::Invalid("source paragraph scope"))?;
    let p = &source.paragraphs[paragraph as usize];
    if p.runs.len() != runs.len() {
        return Err(SourceTextError::Invalid("source run count"));
    }
    let direction = match p.attributes.right_to_left {
        Some(false) => ParagraphDirection::LeftToRight,
        Some(true) => ParagraphDirection::RightToLeft,
        None => return Err(SourceTextError::Invalid("cascaded paragraph direction")),
    };
    let mut segments = vec![];
    let mut sources = vec![];
    let mut count = 0;
    for (i, (r, native)) in p.runs.iter().zip(runs).enumerate() {
        cancel(check)?;
        if r.run as usize != i || r.kind != native.kind {
            return Err(SourceTextError::Invalid("source run binding"));
        }
        if native.kind == SourceRunKind::Field {
            return Ok(Err(SourceTextIssue::Field {
                paragraph: native_paragraph,
                run: i as u32,
                source_ordinal: r.source_ordinal,
            }));
        }
        let text = if native.kind == SourceRunKind::Break {
            "\u{2028}"
        } else {
            native.text.as_str()
        };
        let end = count + text.chars().count() as u32;
        sources.push(SourceScalarRange {
            start: count,
            end,
            run: i as u32,
            source_ordinal: r.source_ordinal,
            kind: native.kind,
        });
        segments.push(computation::Segment {
            text,
            style: &r.style,
        });
        count = end;
    }
    let resolve = |run, slot, script: Option<&str>, limits, check: &dyn Fn() -> bool| {
        mo_presentation_source::source::text::fonts::resolve(
            index, source, paragraph, run, slot, script, limits, check,
        )
    };
    Ok(
        match computation::paragraph(
            native_paragraph,
            direction,
            &segments,
            &p.end_style,
            &resolve,
            budget,
            check,
        )? {
            Ok(computation) => Ok(SourceParagraphPlan {
                profile: PROFILE.into(),
                source_ordinal: p.source_ordinal,
                sources,
                computation,
            }),
            Err(issue) => Err(issue),
        },
    )
}
