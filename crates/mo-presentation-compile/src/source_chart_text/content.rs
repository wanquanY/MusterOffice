use super::*;
use crate::source_text::budget::Budget;
use mo_charts::number_format::NumberFragment;
use mo_presentation_source::source::{SourceRunKind, charts::SourceChartTextParagraph};
use std::borrow::Cow;

pub(super) struct ContentSegment<'a> {
    pub text: Cow<'a, str>,
    pub character: u32,
    pub origin: ChartTextOrigin,
}
pub(super) struct Content<'a> {
    pub segments: Vec<ContentSegment<'a>>,
    pub characters: Vec<CascadedCharacterStyle>,
}
pub(super) fn complete(
    style: &CascadedCharacterStyle,
    paragraph: u32,
    segment: Option<u32>,
    resolver: &mut ChartTextResolver<'_>,
    budget: &mut Budget,
) -> Result<Result<CascadedCharacterStyle, ChartTextIssue>, ChartTextError> {
    if style.attributes.size.is_none() {
        return Ok(Err(ChartTextIssue::MissingCharacterSize {
            paragraph,
            segment,
        }));
    }
    budget.character(style)?;
    let mut style = style.clone();
    resolver.complete_label_style(&mut style)?;
    Ok(Ok(style))
}
pub(super) fn native<'a>(
    native: &'a SourceChartTextParagraph,
    source: &CascadedParagraph,
    paragraph: u32,
    resolver: &mut ChartTextResolver<'_>,
    budget: &mut Budget,
    check: &dyn Fn() -> bool,
) -> Result<Result<Content<'a>, ChartTextIssue>, ChartTextError> {
    if native.source_ordinal != source.source_ordinal || native.runs.len() != source.runs.len() {
        return Err(ChartTextError::Invalid("chart rich paragraph"));
    }
    let mut content = Content {
        segments: vec![],
        characters: vec![],
    };
    for (i, (run, cascaded)) in native.runs.iter().zip(&source.runs).enumerate() {
        cancel(check)?;
        if run.source_ordinal != cascaded.source_ordinal
            || run.kind != cascaded.kind
            || cascaded.run as usize != i
        {
            return Err(ChartTextError::Invalid("chart rich run"));
        }
        if run.kind == SourceRunKind::Field {
            return Ok(Err(ChartTextIssue::Computation {
                issue: SourceTextIssue::Field {
                    paragraph,
                    run: i as u32,
                    source_ordinal: run.source_ordinal,
                },
            }));
        }
        let style = match complete(&cascaded.style, paragraph, Some(i as u32), resolver, budget)? {
            Ok(style) => style,
            Err(issue) => return Ok(Err(issue)),
        };
        content.characters.push(style);
        content.segments.push(ContentSegment {
            text: Cow::Borrowed(if run.kind == SourceRunKind::Break {
                "\u{2028}"
            } else {
                &run.text
            }),
            character: i as u32,
            origin: ChartTextOrigin::NativeRun {
                source_ordinal: run.source_ordinal,
                text_source_ordinal: run.text_source_ordinal,
                run: i as u32,
            },
        });
    }
    Ok(Ok(content))
}
pub(super) fn generated<'a>(
    label: &'a ChartLabelPlan,
    style: CascadedCharacterStyle,
    budget: &mut Budget,
    check: &dyn Fn() -> bool,
) -> Result<Result<Content<'a>, ChartTextIssue>, ChartTextError> {
    let Some(displays) = &label.formatted_components else {
        return Ok(Err(ChartTextIssue::MissingNumberDisplay {}));
    };
    if displays.len() != label.components.len() {
        return Err(ChartTextError::Invalid("chart display components"));
    }
    let mut content = Content {
        segments: vec![],
        characters: vec![style],
    };
    // Draft assembly order follows the bound component order, preserving its
    // indices. Office/WPS combined-label order still has a separate visual gate.
    for (idx, display) in displays.iter().enumerate() {
        cancel(check)?;
        let component = idx as u32;
        if idx > 0 {
            let separator: Cow<'_, str> = match &label.settings.separator {
                Some(ChartLabelSeparator::Declared { value, .. }) => {
                    // Only the separator has native newline semantics; cached
                    // text is never globally normalized or rewritten.
                    if value.contains(['\n', '\r']) {
                        budget.charge(value.len() * 4 + 64)?;
                        Cow::Owned(
                            value
                                .replace("\r\n", "\n")
                                .replace(['\n', '\r'], "\u{2028}"),
                        )
                    } else {
                        Cow::Borrowed(value)
                    }
                }
                Some(ChartLabelSeparator::CommaDefault {}) => Cow::Borrowed(", "),
                Some(ChartLabelSeparator::PieCategoryPercentLineBreak {}) => {
                    Cow::Borrowed("\u{2028}")
                }
                None => return Ok(Err(ChartTextIssue::MissingSeparator {})),
            };
            budget.charge(128)?;
            content.segments.push(ContentSegment {
                text: separator,
                character: 0,
                origin: ChartTextOrigin::Separator {
                    before_component: component,
                },
            });
        }
        match display {
            ChartLabelComponentDisplay::Text { value } => {
                budget.charge(128)?;
                content.segments.push(ContentSegment {
                    text: Cow::Borrowed(value),
                    character: 0,
                    origin: ChartTextOrigin::Component {
                        component,
                        fragment: None,
                        color: None,
                    },
                });
            }
            ChartLabelComponentDisplay::Number { display } => {
                for (i, fragment) in display.fragments.iter().enumerate() {
                    cancel(check)?;
                    let NumberFragment::Text { value } = fragment else {
                        return Ok(Err(ChartTextIssue::NumberSpacing {
                            component,
                            fragment: i as u32,
                        }));
                    };
                    budget.charge(128)?;
                    content.segments.push(ContentSegment {
                        text: Cow::Borrowed(value),
                        character: 0,
                        origin: ChartTextOrigin::Component {
                            component,
                            fragment: Some(i as u32),
                            color: display.color,
                        },
                    });
                }
            }
            ChartLabelComponentDisplay::MissingFormat {} => {
                return Ok(Err(ChartTextIssue::MissingNumberFormat { component }));
            }
            ChartLabelComponentDisplay::Unresolved { issue } => {
                return Ok(Err(ChartTextIssue::NumberFormat {
                    component,
                    issue: issue.clone(),
                }));
            }
        }
    }
    Ok(Ok(content))
}
