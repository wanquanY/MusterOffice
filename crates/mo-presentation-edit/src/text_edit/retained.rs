//! High-level ranges over immutable native text leaves. No authored-body adapter,
//! source-binding rewrite, run deletion, or frontend-generated splice list.
use super::*;

pub(super) struct NativeRange<'a> {
    paragraph: &'a RetainedParagraph,
    range: TextSelection,
    insertion: Option<usize>,
    leaves: Vec<(u32, u32)>,
}
fn paragraph_text(p: &RetainedParagraph, check: &dyn Fn() -> bool) -> Result<String, EditError> {
    if p.runs.len() > 65_536 {
        return Err(EditError::input("retained paragraph run budget"));
    }
    let mut text = String::new();
    for run in &p.runs {
        cancelled(check)?;
        let part = if run.kind == RetainedRunKind::Break {
            "\u{2028}"
        } else {
            &run.text
        };
        if text.len().saturating_add(part.len()) > 262_144 {
            return Err(EditError::input("retained paragraph text budget"));
        }
        text.push_str(part);
    }
    Ok(text)
}
fn editable(
    document: &Document,
    object: &ObjectId,
    run: &RetainedTextRun,
) -> Result<(), EditError> {
    let binding = document
        .source_bindings
        .as_ref()
        .and_then(|b| b.objects.get(object))
        .and_then(|o| o.runs.get(&run.id))
        .ok_or_else(|| EditError::input("retained text binding missing"))?;
    if run.kind != RetainedRunKind::Text {
        return Err(restricted(TextEditRestriction::StructuredRun {
            run: run.id.clone(),
            run_kind: run.kind,
        }));
    }
    if let Some(constraint) = binding.constraint {
        return Err(restricted(TextEditRestriction::NativeRun {
            run: run.id.clone(),
            constraint,
        }));
    }
    Ok(())
}

pub(super) fn range<'a>(
    document: &Document,
    object: &ObjectId,
    paragraphs: &'a [RetainedParagraph],
    selection: &TextSelection,
    inserting: bool,
    check: &dyn Fn() -> bool,
) -> Result<NativeRange<'a>, EditError> {
    let locate = |a: &TextAnchor| -> Result<&'a RetainedParagraph, EditError> {
        let p = paragraphs
            .iter()
            .find(|p| p.id == a.paragraph)
            .ok_or_else(|| EditError::input("text selection paragraph does not exist"))?;
        let text = paragraph_text(p, check)?;
        if !segment(&text, check)?
            .boundaries
            .iter()
            .any(|b| b.scalar_offset == a.scalar_offset)
        {
            return Err(EditError::input("selection must be at a grapheme boundary"));
        }
        Ok(p)
    };
    let p = locate(&selection.anchor)?;
    locate(&selection.focus)?;
    if selection.anchor.paragraph != selection.focus.paragraph {
        return Err(restricted(TextEditRestriction::RetainedParagraphBoundary));
    }
    let (start, end) = if selection.anchor.scalar_offset <= selection.focus.scalar_offset {
        (&selection.anchor, &selection.focus)
    } else {
        (&selection.focus, &selection.anchor)
    };
    // A collapsed empty replacement is an ordinary zero-length splice, so it
    // still needs an editable native insertion owner.
    let needs_owner = inserting || start.scalar_offset == end.scalar_offset;
    let mut cursor = 0u32;
    let mut insertion = None;
    let mut leaves = Vec::with_capacity(p.runs.len());
    for (i, run) in p.runs.iter().enumerate() {
        cancelled(check)?;
        let next = cursor
            .checked_add(run.scalar_len() as u32)
            .ok_or_else(|| EditError::input("retained text scalar range"))?;
        if needs_owner
            && insertion.is_none()
            && (start.scalar_offset < next
                || (start.scalar_offset == next && start.affinity == Affinity::Before))
        {
            insertion = Some(i);
        }
        if start.scalar_offset.max(cursor) < end.scalar_offset.min(next) {
            editable(document, object, run)?;
        }
        leaves.push((cursor, next));
        cursor = next;
    }
    if needs_owner {
        let i = insertion
            .or_else(|| p.runs.len().checked_sub(1))
            .ok_or_else(|| restricted(TextEditRestriction::NativeInsertionTarget))?;
        editable(document, object, &p.runs[i])?;
        insertion = Some(i);
    }
    Ok(NativeRange {
        paragraph: p,
        range: TextSelection {
            anchor: start.clone(),
            focus: end.clone(),
        },
        insertion,
        leaves,
    })
}

pub(super) fn prepare(
    snapshot: &Snapshot,
    command: &TextEditCommand,
    paragraphs: &[RetainedParagraph],
    limits: ValidationLimits,
    check: &dyn Fn() -> bool,
) -> Result<PreparedTextEdit, EditError> {
    let TextEditAction::Replace { selection, text } = &command.action else {
        return Err(restricted(TextEditRestriction::NativeStructureRequired));
    };
    if text.contains(['\n', '\r', '\u{2028}', '\u{2029}']) {
        return Err(restricted(TextEditRestriction::NativeStructureRequired));
    }
    // Apply the same bounded Unicode input admission as authored replacement.
    segment(text, check)?;
    let native = range(
        snapshot.document(),
        &command.object,
        paragraphs,
        selection,
        !text.is_empty(),
        check,
    )?;
    let start = native.range.anchor.scalar_offset;
    let end = native.range.focus.scalar_offset;
    // Validate the resulting paragraph too: inserted text may join a suffix
    // combining mark or exceed the segmentation budget with surviving text.
    let original = paragraph_text(native.paragraph, check)?;
    let updated: String = original
        .chars()
        .take(start as usize)
        .chain(text.chars())
        .chain(original.chars().skip(end as usize))
        .collect();
    let segmentation = segment(&updated, check)?;
    let inserted_end = start + text.chars().count() as u32;
    let caret_end = segmentation
        .boundaries
        .iter()
        .find(|b| b.scalar_offset >= inserted_end)
        .expect("paragraph end is always a boundary")
        .scalar_offset;
    let seed = text_edit_command_digest(command)?;
    let mut operations = Vec::new();
    for (i, (&(from, to), run)) in native
        .leaves
        .iter()
        .zip(&native.paragraph.runs)
        .enumerate()
        .rev()
    {
        cancelled(check)?;
        let lo = start.max(from).min(to);
        let hi = end.min(to).max(lo);
        let insert = native.insertion == Some(i);
        if lo == hi && !insert {
            continue;
        }
        if operations.len() >= 10_000 {
            return Err(EditError::input("retained text operation budget"));
        }
        operations.push(OperationEntry {
            operation_id: OperationId::new(format!("text-native:{seed}:{i}"))
                .expect("bounded native operation ID"),
            operation: Operation::SpliceText {
                object: command.object.clone(),
                paragraph: native.paragraph.id.clone(),
                run: run.id.clone(),
                start: lo - from,
                delete: hi - lo,
                insert: if insert { text.clone() } else { String::new() },
            },
        });
    }
    let transaction = Transaction {
        document_id: command.document_id.clone(),
        base_revision: command.base_revision.clone(),
        request_id: command.request_id.clone(),
        operations,
    };
    let prepared = prepare_cancellable(snapshot, &transaction, limits, check)?;
    let focus = TextAnchor {
        paragraph: native.paragraph.id.clone(),
        scalar_offset: inserted_end,
        affinity: Affinity::After,
    };
    let caret = TextAnchor {
        scalar_offset: caret_end,
        ..focus.clone()
    };
    let selection = TextSelection {
        anchor: caret.clone(),
        focus: caret,
    };
    let range_change = TextRangeChange {
        object: command.object.clone(),
        cell: None,
        before: native.range,
        after: TextSelection {
            anchor: TextAnchor {
                scalar_offset: start,
                affinity: Affinity::Before,
                ..focus.clone()
            },
            focus,
        },
        before_paragraphs: vec![native.paragraph.id.clone()],
        after_paragraphs: vec![native.paragraph.id.clone()],
    };
    Ok(PreparedTextEdit {
        command_digest: seed,
        prepared,
        transaction,
        selection,
        range_change: Some(range_change),
    })
}
