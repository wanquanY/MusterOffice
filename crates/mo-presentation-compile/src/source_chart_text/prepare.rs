use super::*;
use crate::source_text::{
    budget::Budget,
    computation::{self, Segment},
};
use mo_opc::PackageRead;
use mo_presentation_source::source::{
    SourceIndex, SourceLimits,
    charts::*,
    text::{NativeTextElement, fonts::ChartTypefaceContext},
};
use mo_unicode::bidi::ParagraphDirection;
use std::collections::BTreeMap;

pub fn prepare(
    package: &dyn PackageRead,
    index: &SourceIndex,
    request: &SourceChartLabelRequest,
    source_limits: SourceLimits,
    limits: ChartLabelTextLimits,
    check: &dyn Fn() -> bool,
) -> Result<ChartLabelTextPreparation, ChartTextError> {
    cancel(check)?;
    let charts = query(
        package,
        index,
        &SourceChartQuery {
            expected_source_sha256: request.expected_source_sha256.clone(),
            surface: request.object.part.clone(),
        },
        source_limits,
        limits.labels.source,
        check,
    )?;
    let binding = charts
        .bindings
        .iter()
        .find(|b| b.object == request.object)
        .ok_or(ChartTextError::Invalid("chart text object"))?;
    let chart = &charts.charts[binding.chart as usize];
    let bindings =
        crate::source_chart_labels::compute_prepared(chart, request, limits.labels, check)?;
    let surface = mo_opc::PartName::new(&request.object.part)
        .map_err(mo_presentation_source::PptxError::from)?;
    let mut bytes_left = (limits.labels.source.max_total_part_bytes as u64)
        .checked_sub(package.parts()[&surface].byte_length)
        .ok_or(mo_presentation_source::PptxError::Limit(
            "chart text total part bytes",
        ))?;
    for chart in &charts.charts {
        bytes_left = bytes_left.checked_sub(chart.byte_length.get()).ok_or(
            mo_presentation_source::PptxError::Limit("chart text total part bytes"),
        )?;
    }
    let overlay = context::read_theme_override(
        package,
        chart,
        source_limits,
        limits.labels.source,
        &mut bytes_left,
        check,
    )?;
    let font_context = ChartTypefaceContext::new(
        index,
        &request.object,
        chart,
        overlay.as_ref().map(|(p, t)| (p.as_str(), t)),
    )?;
    let mut resolver = ChartTextResolver::new(chart, limits.text.cascade, check)?;
    let bodies: BTreeMap<_, _> = chart
        .annotations
        .text_bodies
        .iter()
        .map(|b| (b.source_ordinal, b))
        .collect();
    let mut budget = Budget::new(limits.text);
    let mut total_bytes = 0usize;
    let mut labels = vec![];
    for label in &bindings.labels {
        cancel(check)?;
        budget.charge(128)?;
        let unresolved = |issue| ChartLabelTextPreparation::Unresolved {
            target: label.target,
            issue,
        };
        if label.settings.deleted.as_ref().is_some_and(|d| d.value) {
            labels.push(ChartLabelText {
                target: label.target,
                paragraphs: vec![],
            });
            continue;
        }
        if label.custom_text_source.is_none() && !label.settings.unresolved_flags.is_empty() {
            return Ok(unresolved(ChartTextIssue::UnresolvedFlags {
                flags: label.settings.unresolved_flags.clone(),
            }));
        }
        if label.custom_text_source.is_none() && label.components.is_empty() {
            labels.push(ChartLabelText {
                target: label.target,
                paragraphs: vec![],
            });
            continue;
        }
        let Some(cascade) = label
            .text_cascade
            .and_then(|i| bindings.text_cascades.get(i as usize))
        else {
            return Ok(unresolved(ChartTextIssue::MissingTextProperties {}));
        };
        let ChartTextOutcome::Cascaded { text } = cascade else {
            let ChartTextOutcome::Unresolved { reason } = cascade else {
                unreachable!()
            };
            return Ok(unresolved(ChartTextIssue::TextCascade {
                reason: reason.clone(),
            }));
        };
        let body = bodies
            .get(&text.body_source_ordinal)
            .ok_or(ChartTextError::Invalid("chart text body"))?;
        let rich = body.styles.nodes[&body.source_ordinal].element == NativeTextElement::Rich;
        if label.custom_text_source.is_some() && !rich {
            return Ok(unresolved(ChartTextIssue::CustomStringReference {}));
        }
        if !rich && text.paragraphs.len() != 1 {
            return Err(ChartTextError::Invalid("chart property paragraph count"));
        }
        let mut paragraphs = vec![];
        for (idx, p) in text.paragraphs.iter().enumerate() {
            cancel(check)?;
            let paragraph = idx as u32;
            let end_style =
                match content::complete(&p.end_style, paragraph, None, &mut resolver, &mut budget)?
                {
                    Ok(style) => style,
                    Err(issue) => return Ok(unresolved(issue)),
                };
            let content = if rich {
                content::native(
                    &body.paragraphs[idx],
                    p,
                    paragraph,
                    &mut resolver,
                    &mut budget,
                    check,
                )?
            } else {
                let style = match resolver.default_character(text, paragraph)? {
                    Ok(style) => style,
                    Err(reason) => return Ok(unresolved(ChartTextIssue::TextCascade { reason })),
                };
                let style = match content::complete(
                    &style,
                    paragraph,
                    Some(0),
                    &mut resolver,
                    &mut budget,
                )? {
                    Ok(style) => style,
                    Err(issue) => return Ok(unresolved(issue)),
                };
                content::generated(label, style, &mut budget, check)?
            };
            let content = match content {
                Ok(c) => c,
                Err(issue) => return Ok(unresolved(issue)),
            };
            let mut sources = vec![];
            let mut segments = vec![];
            let mut count = 0u32;
            for segment in &content.segments {
                cancel(check)?;
                total_bytes = total_bytes
                    .checked_add(segment.text.len().max(1))
                    .filter(|n| *n <= limits.text.max_text_bytes)
                    .ok_or(SourceTextError::Limit("chart text bytes"))?;
                let end = count
                    .checked_add(segment.text.chars().count() as u32)
                    .ok_or(SourceTextError::Limit("chart text scalars"))?;
                sources.push(ChartTextRange {
                    start: count,
                    end,
                    character: segment.character,
                    origin: segment.origin.clone(),
                });
                segments.push(Segment {
                    text: &segment.text,
                    style: &content.characters[segment.character as usize],
                });
                count = end;
            }
            let resolve = |run: Option<u32>,
                           slot,
                           script: Option<&str>,
                           typeface,
                           check: &dyn Fn() -> bool| {
                let style = run.map_or(&end_style, |r| segments[r as usize].style);
                font_context.resolve_style(&resolver, style, slot, script, typeface, check)
            };
            let direction = if p.attributes.right_to_left.unwrap_or(false) {
                ParagraphDirection::RightToLeft
            } else {
                ParagraphDirection::LeftToRight
            };
            let computation = match computation::paragraph(
                paragraph,
                direction,
                &segments,
                &end_style,
                &resolve,
                &mut budget,
                check,
            )? {
                Ok(plan) => plan,
                Err(issue) => return Ok(unresolved(ChartTextIssue::Computation { issue })),
            };
            paragraphs.push(ChartTextParagraph {
                source_ordinal: p.source_ordinal,
                attributes: p.attributes.clone(),
                characters: content.characters,
                end_style,
                sources,
                computation,
            });
        }
        labels.push(ChartLabelText {
            target: label.target,
            paragraphs,
        });
    }
    cancel(check)?;
    Ok(ChartLabelTextPreparation::Prepared {
        text: PreparedChartLabelText {
            bindings,
            labels,
            accounted_plan_bytes: budget.accounted_bytes(),
        },
    })
}
