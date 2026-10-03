mod range;
mod target;
mod types;
use crate::{
    EditError, Operation, OperationEntry, PreparedTransaction, Snapshot, Transaction,
    prepare_cancellable,
};
use mo_common::*;
use mo_presentation_model::*;
use range::*;
pub use types::*;

pub struct PreparedTextEdit {
    /// Binds the original intent, separately from its expanded transaction.
    pub command_digest: Digest,
    pub prepared: PreparedTransaction,
    /// The same semantic transaction used by ordinary editing/history; callers
    /// persist the original text command and this deterministic expansion.
    pub transaction: Transaction,
    pub selection: TextSelection,
    pub range_change: Option<TextRangeChange>,
}

impl PreparedTextEdit {
    pub fn into_candidate(self) -> TextEditCandidate {
        TextEditCandidate {
            snapshot: self.prepared.snapshot.into_record(),
            receipt: self.prepared.receipt,
            transaction: self.transaction,
            command_digest: self.command_digest,
            selection: self.selection,
            range_change: self.range_change,
        }
    }
}

pub fn text_edit_command_digest(command: &TextEditCommand) -> Result<Digest, EditError> {
    Ok(digest("musteroffice.text-edit-command/1", command)?)
}

pub fn prepare_text_edit(
    snapshot: &Snapshot,
    command: &TextEditCommand,
    limits: ValidationLimits,
    check: &dyn Fn() -> bool,
) -> Result<PreparedTextEdit, EditError> {
    cancelled(check)?;
    if snapshot.document().id != command.document_id {
        return Err(EditError::input("text command document differs"));
    }
    if snapshot.revision() != &command.base_revision {
        return Err(EditError::RevisionConflict {
            current: snapshot.revision().clone(),
        });
    }
    let body = target::authored_body(snapshot.document(), command, check)?;
    let mut ids = Ids::new(command)?;
    let (next, selection, mut range_change) = match (&command.action, body) {
        (TextEditAction::Initialize { text, setup }, None) => {
            let (next, selection) = target::initialize(text, setup, command, &mut ids, check)?;
            (next, selection, None)
        }
        (TextEditAction::Initialize { .. }, Some(_)) => {
            return Err(EditError::input("text body already exists"));
        }
        (_, None) => return Err(EditError::input("text body requires initialization")),
        (action, Some(body)) => {
            let mut next = body.clone();
            let (selection, change) = edit_body(body, &mut next, action, command, &mut ids, check)?;
            (next, selection, change)
        }
    };
    if let Some(change) = &mut range_change {
        change.cell = command.cell.clone();
    }
    let transaction = Transaction {
        document_id: command.document_id.clone(),
        request_id: command.request_id.clone(),
        base_revision: command.base_revision.clone(),
        operations: vec![OperationEntry {
            operation_id: command.operation_id.clone(),
            operation: match &command.cell {
                Some(cell) => Operation::EditTable {
                    object: command.object.clone(),
                    operation: crate::TableOperation::SetCellText {
                        cell: cell.clone(),
                        text: Some(next),
                    },
                },
                None => Operation::SetText {
                    object: command.object.clone(),
                    text: next,
                },
            },
        }],
    };
    let prepared = prepare_cancellable(snapshot, &transaction, limits, check)?;
    Ok(PreparedTextEdit {
        command_digest: text_edit_command_digest(command)?,
        prepared,
        transaction,
        selection,
        range_change,
    })
}

fn edit_body(
    body: &TextBody,
    next: &mut TextBody,
    action: &TextEditAction,
    command: &TextEditCommand,
    ids: &mut Ids,
    check: &dyn Fn() -> bool,
) -> Result<(TextSelection, Option<TextRangeChange>), EditError> {
    Ok(match action {
        TextEditAction::Initialize { .. } => unreachable!("initialization handled before editing"),
        TextEditAction::Replace { selection, text } => {
            let range = ordered(body, selection, check)?;
            replace(next, &range, text, &command.object, ids, check)?
        }
        TextEditAction::SetCharacterStyle { selection, patch } => {
            let range = ordered(body, selection, check)?;
            if range.first == range.last && range.start.scalar_offset == range.end.scalar_offset {
                return Err(EditError::input(
                    "character styling requires a nonempty selection",
                ));
            }
            for i in range.first..=range.last {
                cancelled(check)?;
                let p = &body.paragraphs[i];
                let start = if i == range.first {
                    range.start.scalar_offset as usize
                } else {
                    0
                };
                let end = if i == range.last {
                    range.end.scalar_offset as usize
                } else {
                    length(p)
                };
                next.paragraphs[i].runs = styled_runs(p, start, end, patch, check)?;
            }
            ids.unique_runs(&mut next.paragraphs);
            (selection.clone(), None)
        }
    })
}

fn replace(
    body: &mut TextBody,
    range: &Range,
    text: &str,
    object: &ObjectId,
    ids: &mut Ids,
    check: &dyn Fn() -> bool,
) -> Result<(TextSelection, Option<TextRangeChange>), EditError> {
    // Bound before allocating normalized text or derived paragraphs.
    segment(text, check)?;
    let normalized = text.replace("\r\n", "\n").replace('\r', "\n");
    let pieces: Vec<_> = normalized.split('\n').collect();
    let first = &body.paragraphs[range.first];
    let last = &body.paragraphs[range.last];
    let prefix = slice_runs(first, 0, range.start.scalar_offset as usize, check)?;
    let suffix = slice_runs(last, range.end.scalar_offset as usize, length(last), check)?;
    let style = style_at(
        first,
        range.start.scalar_offset as usize,
        range.start.affinity,
    );
    let mut inserted = Vec::with_capacity(pieces.len());
    for (i, piece) in pieces.iter().enumerate() {
        cancelled(check)?;
        let mut p = Paragraph {
            id: if i == 0 {
                first.id.clone()
            } else {
                ids.paragraph()
            },
            style: first.style.clone(),
            default_run_style: first.default_run_style.clone(),
            runs: if i == 0 { prefix.clone() } else { Vec::new() },
        };
        for (j, part) in piece.split('\t').enumerate() {
            cancelled(check)?;
            if j > 0 {
                p.runs.push(TextRun {
                    id: ids.run(),
                    style: style.clone(),
                    content: InlineContent::Tab,
                });
            }
            if !part.is_empty() {
                p.runs.push(TextRun {
                    id: ids.run(),
                    style: style.clone(),
                    content: InlineContent::Text { text: part.into() },
                });
            }
        }
        if i + 1 == pieces.len() {
            p.runs.extend(suffix.clone());
        }
        if p.runs.is_empty() {
            p.runs.push(TextRun {
                id: ids.run(),
                style: style.clone(),
                content: InlineContent::Text {
                    text: String::new(),
                },
            });
        }
        inserted.push(p);
    }
    let end = TextAnchor {
        paragraph: inserted.last().expect("split has a sentinel").id.clone(),
        scalar_offset: u32::try_from(pieces.last().unwrap().chars().count())
            .map_err(|_| EditError::input("text insertion length"))?
            + if pieces.len() == 1 {
                range.start.scalar_offset
            } else {
                0
            },
        affinity: Affinity::After,
    };
    let change = TextRangeChange {
        object: object.clone(),
        cell: None,
        before: TextSelection {
            anchor: range.start.clone(),
            focus: range.end.clone(),
        },
        after: TextSelection {
            anchor: range.start.clone(),
            focus: end.clone(),
        },
        before_paragraphs: body.paragraphs[range.first..=range.last]
            .iter()
            .map(|p| p.id.clone())
            .collect(),
        after_paragraphs: inserted.iter().map(|p| p.id.clone()).collect(),
    };
    // A replacement can join two previously separate graphemes (for example
    // deleting a space between a letter and a combining mark). Keep the change
    // map in exact scalar coordinates, but return a usable caret on the next
    // grapheme boundary. Validate the resulting paragraph budget as well.
    let mut caret = end;
    for p in &inserted {
        let segmentation = segment(&plain(p), check)?;
        if p.id == caret.paragraph {
            caret.scalar_offset = segmentation
                .boundaries
                .iter()
                .find(|b| b.scalar_offset >= caret.scalar_offset)
                .expect("paragraph end is always a boundary")
                .scalar_offset;
        }
    }
    body.paragraphs.splice(range.first..=range.last, inserted);
    ids.unique_runs(&mut body.paragraphs);
    Ok((
        TextSelection {
            anchor: caret.clone(),
            focus: caret,
        },
        Some(change),
    ))
}

fn cancelled(check: &dyn Fn() -> bool) -> Result<(), EditError> {
    if check() {
        Err(EditError::Cancelled)
    } else {
        Ok(())
    }
}
