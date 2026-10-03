//! Bind the rendered frame's original native paragraph range to the validated
//! semantic model. Never use XML discovery ordinals as paragraph identities.
use super::*;
use mo_common::{CellId, ObjectId, RunId};
use mo_pptx::source::SourceRunKind;
use mo_pptx::source::table::SourceCellAddress;
use mo_presentation_compile::source_text::{NATIVE_SOFT_BREAK, SourceScalarRange};
use mo_presentation_model::{
    Document, InlineContent, ObjectContent, Paragraph, RetainedParagraph, RetainedRunKind,
    SourceBindingProfile,
};

pub(super) enum TextIdentity<'a> {
    Unprojected,
    Authored {
        paragraphs: &'a [Paragraph],
        cell: Option<&'a CellId>,
    },
    EmptyCell(&'a CellId),
    Retained(&'a [RetainedParagraph]),
}
fn invalid() -> PptxResourcePageFailure {
    failure(
        PptxPageFailureCode::InputInvalid,
        "editor paragraph model identity",
    )
}
pub(super) fn bind<'a>(
    document: Option<&'a Document>,
    object: Option<&ObjectId>,
    cell: Option<SourceCellAddress>,
) -> Result<TextIdentity<'a>, PptxResourcePageFailure> {
    let Some(document) = document else {
        return Ok(TextIdentity::Unprojected);
    };
    let content = &document
        .objects
        .get(object.ok_or_else(invalid)?)
        .ok_or_else(invalid)?
        .content;
    match content {
        ObjectContent::Shape {
            text: Some(body), ..
        } if cell.is_none() => Ok(TextIdentity::Authored {
            paragraphs: &body.paragraphs,
            cell: None,
        }),
        ObjectContent::Table { table } => {
            let at = cell.ok_or_else(invalid)?;
            let cell = table
                .rows
                .get(at.row as usize)
                .and_then(|r| r.cells.get(at.column as usize))
                .ok_or_else(invalid)?;
            Ok(match &cell.text {
                Some(body) => TextIdentity::Authored {
                    paragraphs: &body.paragraphs,
                    cell: Some(&cell.id),
                },
                None => TextIdentity::EmptyCell(&cell.id),
            })
        }
        ObjectContent::RetainedSource { paragraphs, .. } => {
            let profile = document
                .source_bindings
                .as_ref()
                .ok_or_else(invalid)?
                .profile;
            // Legacy profiles intentionally do not project table text into the
            // model. Rendering it never invents editable IDs or upgrades it.
            if cell.is_some()
                && matches!(
                    profile,
                    SourceBindingProfile::PresentationmlRetainedFieldsV1
                        | SourceBindingProfile::PresentationmlRetainedFieldsV2
                )
            {
                if !paragraphs.is_empty() {
                    return Err(invalid());
                }
                Ok(TextIdentity::Unprojected)
            } else {
                Ok(TextIdentity::Retained(paragraphs))
            }
        }
        _ => Err(invalid()),
    }
}
impl TextIdentity<'_> {
    pub fn cell_id(&self) -> Option<&CellId> {
        match self {
            Self::Authored { cell, .. } => *cell,
            Self::EmptyCell(cell) => Some(cell),
            _ => None,
        }
    }
    pub fn paragraph(
        &self,
        local: usize,
        native: usize,
        text: &str,
        sources: &[SourceScalarRange],
        check: &dyn Fn() -> bool,
    ) -> Result<Option<EditorParagraphIdentity>, PptxResourcePageFailure> {
        cancelled(check)?;
        let mut offset = 0usize;
        let mut runs = vec![];
        // Compare exact UTF-8 slices as well as scalar extents. The public
        // display string and model IDs must refer to the same original text.
        let mut push = |id: &RunId,
                        content: &str,
                        kind: SourceRunKind|
         -> Result<(), PptxResourcePageFailure> {
            cancelled(check)?;
            let end = offset.checked_add(content.len()).ok_or_else(invalid)?;
            if text.get(offset..end) != Some(content) {
                return Err(invalid());
            }
            let start = runs
                .last()
                .map_or(0u32, |r: &EditorTextRunIdentity| r.scalar_end);
            let length = u32::try_from(content.chars().count()).map_err(|_| invalid())?;
            let source = sources.get(runs.len()).ok_or_else(invalid)?;
            if source.run as usize != runs.len()
                || source.kind != kind
                || source.start != start
                || source.end != start.checked_add(length).ok_or_else(invalid)?
            {
                return Err(invalid());
            }
            runs.push(EditorTextRunIdentity {
                id: id.clone(),
                scalar_start: source.start,
                scalar_end: source.end,
            });
            offset = end;
            Ok(())
        };
        let id = match self {
            Self::Unprojected => return Ok(None),
            Self::EmptyCell(_) => {
                if local != 0 || !text.is_empty() || !sources.is_empty() {
                    return Err(invalid());
                }
                return Ok(None);
            }
            Self::Authored { paragraphs, .. } => {
                let p = paragraphs.get(local).ok_or_else(invalid)?;
                for r in &p.runs {
                    push(
                        &r.id,
                        match &r.content {
                            InlineContent::Text { text } => text,
                            InlineContent::Break => NATIVE_SOFT_BREAK,
                            InlineContent::Tab => "\t",
                        },
                        if matches!(r.content, InlineContent::Break) {
                            SourceRunKind::Break
                        } else {
                            SourceRunKind::Text
                        },
                    )?;
                }
                p.id.clone()
            }
            Self::Retained(paragraphs) => {
                let p = paragraphs.get(native).ok_or_else(invalid)?;
                for r in &p.runs {
                    push(
                        &r.id,
                        if r.kind == RetainedRunKind::Break {
                            NATIVE_SOFT_BREAK
                        } else {
                            &r.text
                        },
                        match r.kind {
                            RetainedRunKind::Text => SourceRunKind::Text,
                            RetainedRunKind::Break => SourceRunKind::Break,
                            RetainedRunKind::Field => SourceRunKind::Field,
                        },
                    )?;
                }
                p.id.clone()
            }
        };
        if offset != text.len() || runs.len() != sources.len() {
            return Err(invalid());
        }
        Ok(Some(EditorParagraphIdentity { id, runs }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mo_common::ParagraphId;
    use mo_presentation_model::{RetainedTextRun, TextRun};
    fn sources(ranges: &[(u32, u32, SourceRunKind)]) -> Vec<SourceScalarRange> {
        ranges
            .iter()
            .enumerate()
            .map(|(i, (start, end, kind))| SourceScalarRange {
                start: *start,
                end: *end,
                kind: *kind,
                run: i as u32,
                source_ordinal: 100 + i as u32,
            })
            .collect()
    }
    #[test]
    fn scalar_spans_keep_empty_runs_and_do_not_confuse_utf16_or_graphemes() {
        let contents = [
            InlineContent::Text {
                text: "😀A".into()
            },
            InlineContent::Text {
                text: "\u{301}".into(),
            },
            InlineContent::Text {
                text: String::new(),
            },
            InlineContent::Break,
            InlineContent::Tab,
        ];
        let paragraphs = [Paragraph {
            id: ParagraphId::new("p").unwrap(),
            style: Default::default(),
            default_run_style: Default::default(),
            runs: contents
                .into_iter()
                .enumerate()
                .map(|(i, content)| TextRun {
                    id: RunId::new(format!("r:{i}")).unwrap(),
                    style: Default::default(),
                    content,
                })
                .collect(),
        }];
        let binding = TextIdentity::Authored {
            paragraphs: &paragraphs,
            cell: None,
        };
        let source = sources(&[
            (0, 2, SourceRunKind::Text),
            (2, 3, SourceRunKind::Text),
            (3, 3, SourceRunKind::Text),
            (3, 4, SourceRunKind::Break),
            (4, 5, SourceRunKind::Text),
        ]);
        let p = binding
            .paragraph(0, 30, "😀A\u{301}\u{2028}\t", &source, &|| false)
            .unwrap()
            .unwrap();
        assert_eq!(p.id, paragraphs[0].id);
        assert_eq!(
            p.runs
                .iter()
                .map(|r| (r.scalar_start, r.scalar_end))
                .collect::<Vec<_>>(),
            [(0, 2), (2, 3), (3, 3), (3, 4), (4, 5)]
        );
        assert!(
            binding
                .paragraph(0, 0, "😀A\u{301}\u{2028}", &source, &|| false)
                .is_err()
        );
        assert!(
            binding
                .paragraph(0, 0, "😀B\u{301}\u{2028}\t", &source, &|| false)
                .is_err()
        );
        assert!(binding.paragraph(1, 0, "", &source, &|| false).is_err());
        assert!(
            binding
                .paragraph(0, 0, "😀A\u{301}\u{2028}\t", &source, &|| true)
                .is_err()
        );
        // A soft break is one model scalar but has its own computation marker.
        assert!(
            binding
                .paragraph(0, 0, "😀A\u{301}\n\t", &source, &|| false)
                .is_err()
        );
        let mut wrong = source.clone();
        wrong[1].run = 0;
        assert!(
            binding
                .paragraph(0, 0, "😀A\u{301}\u{2028}\t", &wrong, &|| false)
                .is_err()
        );
    }
    #[test]
    fn retained_cell_uses_original_physical_paragraph_index_even_for_equal_text() {
        let paragraphs: Vec<_> = (0..4)
            .map(|i| RetainedParagraph {
                id: ParagraphId::new(format!("p:{i}")).unwrap(),
                runs: vec![RetainedTextRun {
                    id: RunId::new(format!("r:{i}")).unwrap(),
                    kind: RetainedRunKind::Text,
                    text: "AAA".into(),
                }],
            })
            .collect();
        let binding = TextIdentity::Retained(&paragraphs);
        let source = sources(&[(0, 3, SourceRunKind::Text)]);
        let p = binding
            .paragraph(0, 3, "AAA", &source, &|| false)
            .unwrap()
            .unwrap();
        assert_eq!(p.id, paragraphs[3].id);
        assert_eq!(p.runs[0].id, paragraphs[3].runs[0].id);
        assert!(binding.paragraph(0, 4, "AAA", &source, &|| false).is_err());
    }
    #[test]
    fn authored_empty_cell_has_its_real_cell_id_and_no_invented_paragraph() {
        let raw: serde_json::Value = serde_json::from_str(include_str!(
            "../../../../fixtures/presentations/native-tables/request.json"
        ))
        .unwrap();
        let document: Document = serde_json::from_value(raw["document"].clone()).unwrap();
        let object = ObjectId::new("shape:1").unwrap();
        let binding = bind(
            Some(&document),
            Some(&object),
            Some(SourceCellAddress { row: 2, column: 2 }),
        )
        .unwrap();
        assert_eq!(binding.cell_id().unwrap().as_str(), "cell:2:2");
        assert!(
            binding
                .paragraph(0, 8, "", &[], &|| false)
                .unwrap()
                .is_none()
        );
        assert!(binding.paragraph(1, 8, "", &[], &|| false).is_err());
        assert!(binding.paragraph(0, 8, "A", &[], &|| false).is_err());
        assert!(bind(Some(&document), Some(&object), None).is_err());
    }
}
