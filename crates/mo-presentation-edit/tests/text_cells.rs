use mo_common::*;
use mo_presentation_edit::*;
use mo_presentation_model::*;

fn initial() -> Snapshot {
    let value: serde_json::Value = serde_json::from_str(include_str!(
        "../../../fixtures/presentations/native-tables/request.json"
    ))
    .unwrap();
    Snapshot::new(
        serde_json::from_value(value["document"].clone()).unwrap(),
        Default::default(),
    )
    .unwrap()
}
fn object() -> ObjectId {
    ObjectId::new("shape:1").unwrap()
}
fn cell_id(id: &str) -> CellId {
    CellId::new(id).unwrap()
}
fn table(s: &Snapshot) -> &Table {
    let ObjectContent::Table { table } = &s.document().objects[&object()].content else {
        panic!()
    };
    table
}
fn body<'a>(s: &'a Snapshot, id: &str) -> &'a TextBody {
    table(s)
        .rows
        .iter()
        .flat_map(|r| &r.cells)
        .find(|c| c.id.as_str() == id)
        .unwrap()
        .text
        .as_ref()
        .unwrap()
}
fn setup() -> TextBodySetup {
    TextBodySetup {
        style: CharacterStyle {
            bold: Inherited::Value(true),
            ..Default::default()
        },
        insets: Insets {
            left: Emu::new(91440),
            right: Emu::new(91440),
            top: Emu::new(45720),
            bottom: Emu::new(45720),
        },
        wrap: true,
        overflow: OverflowPolicy::Clip,
        paragraph_style: ParagraphStyle::default(),
        default_run_style: CharacterStyle {
            italic: Inherited::Value(true),
            ..Default::default()
        },
    }
}
fn anchor(id: ParagraphId, offset: u32) -> TextAnchor {
    TextAnchor {
        paragraph: id,
        scalar_offset: offset,
        affinity: Affinity::After,
    }
}
fn selection(s: &Snapshot, cell: &str, start: u32, end: u32) -> TextSelection {
    let p = &body(s, cell).paragraphs[0].id;
    TextSelection {
        anchor: anchor(p.clone(), start),
        focus: anchor(p.clone(), end),
    }
}
fn command(s: &Snapshot, cell: Option<&str>, action: TextEditAction) -> TextEditCommand {
    TextEditCommand {
        document_id: s.document().id.clone(),
        base_revision: s.revision().clone(),
        request_id: RequestId::new("text-cell:request").unwrap(),
        operation_id: OperationId::new("text-cell:operation").unwrap(),
        object: object(),
        cell: cell.map(cell_id),
        action,
    }
}
fn edit(s: &Snapshot, q: &TextEditCommand) -> PreparedTextEdit {
    prepare_text_edit(s, q, Default::default(), &|| false).unwrap()
}
fn history(
    s: &Snapshot,
    original: &Snapshot,
    tx: &Transaction,
    direction: HistoryDirection,
) -> Snapshot {
    prepare_history(
        s,
        &HistoryTransaction {
            document_id: s.document().id.clone(),
            base_revision: s.revision().clone(),
            request_id: RequestId::new("cell:history").unwrap(),
            original_snapshot: original.clone().into_record(),
            original_transaction: tx.clone(),
            direction,
        },
        Default::default(),
        &|| false,
    )
    .unwrap()
    .snapshot
}
fn plain(body: &TextBody) -> Vec<String> {
    body.paragraphs
        .iter()
        .map(|p| {
            p.runs
                .iter()
                .map(|r| match &r.content {
                    InlineContent::Text { text } => text.as_str(),
                    InlineContent::Break => "\n",
                    InlineContent::Tab => "\t",
                })
                .collect()
        })
        .collect()
}
#[test]
fn merged_origin_range_edit_preserves_every_other_cell_and_supports_history() {
    let base = initial();
    let original = body(&base, "cell:0:0");
    let old = plain(original);
    let q = command(
        &base,
        Some("cell:0:0"),
        TextEditAction::Replace {
            selection: selection(&base, "cell:0:0", 0, 0),
            text: "中😀\r\ne\u{301}".into(),
        },
    );
    let result = edit(&base, &q);
    let next = &result.prepared.snapshot;
    assert_eq!(
        plain(body(next, "cell:0:0")),
        vec!["中😀".into(), format!("e\u{301}{}", old[0])]
    );
    assert_eq!(result.selection.focus.scalar_offset, 2);
    let change = result.range_change.as_ref().unwrap();
    assert_eq!(change.cell, Some(cell_id("cell:0:0")));
    assert_eq!(change.reversed().cell, change.cell);
    assert!(matches!(&result.transaction.operations[0].operation,
        Operation::EditTable { operation:TableOperation::SetCellText { cell, .. }, .. } if cell==&cell_id("cell:0:0")));
    for (a, b) in table(&base)
        .rows
        .iter()
        .flat_map(|r| &r.cells)
        .zip(table(next).rows.iter().flat_map(|r| &r.cells))
    {
        if a.id != cell_id("cell:0:0") {
            assert_eq!(a, b);
        } else {
            assert_eq!(a.merge, b.merge);
            assert_eq!(a.style, b.style);
        }
    }
    assert_eq!(body(next, "cell:0:0").insets, original.insets);
    let undo = history(next, &base, &result.transaction, HistoryDirection::Undo);
    assert_eq!(undo.document(), base.document());
    assert_ne!(undo.revision(), base.revision());
    let redo = history(&undo, &base, &result.transaction, HistoryDirection::Redo);
    assert_eq!(redo.document(), next.document());
}
#[test]
fn cell_style_patch_and_cross_paragraph_join_reuse_the_shape_algorithm() {
    let base = initial();
    let first = edit(
        &base,
        &command(
            &base,
            Some("cell:0:2"),
            TextEditAction::Replace {
                selection: selection(&base, "cell:0:2", 0, 0),
                text: "A😀\n中".into(),
            },
        ),
    )
    .prepared
    .snapshot;
    let b = body(&first, "cell:0:2");
    let selected = TextSelection {
        anchor: anchor(b.paragraphs[1].id.clone(), 1),
        focus: anchor(b.paragraphs[0].id.clone(), 1),
    };
    let styled = edit(
        &first,
        &command(
            &first,
            Some("cell:0:2"),
            TextEditAction::SetCharacterStyle {
                selection: selected.clone(),
                patch: CharacterStylePatch {
                    underline: Some(Inherited::Value(true)),
                    ..Default::default()
                },
            },
        ),
    );
    assert_eq!(styled.selection, selected);
    assert!(styled.range_change.is_none());
    assert!(
        body(&styled.prepared.snapshot, "cell:0:2").paragraphs[0]
            .runs
            .iter()
            .any(|r| r.style.underline == Inherited::Value(true))
    );
    let joined = edit(
        &first,
        &command(
            &first,
            Some("cell:0:2"),
            TextEditAction::Replace {
                selection: selected,
                text: "Z".into(),
            },
        ),
    );
    assert_eq!(
        body(&joined.prepared.snapshot, "cell:0:2").paragraphs.len(),
        1
    );
    assert!(plain(body(&joined.prepared.snapshot, "cell:0:2"))[0].starts_with("AZ"));
}
#[test]
fn missing_cell_body_initializes_with_kernel_ids_and_no_fictional_range_map() {
    let base = initial();
    let settings = setup();
    let q = command(
        &base,
        Some("cell:2:2"),
        TextEditAction::Initialize {
            text: "中😀\r\ne\u{301}".into(),
            setup: settings.clone(),
        },
    );
    let result = edit(&base, &q);
    let next = body(&result.prepared.snapshot, "cell:2:2");
    assert_eq!(plain(next), vec!["中😀", "e\u{301}"]);
    assert_eq!(next.style, settings.style);
    assert_eq!(next.insets, settings.insets);
    assert_eq!(
        next.paragraphs[0].default_run_style,
        settings.default_run_style
    );
    assert_eq!(next.overflow, settings.overflow);
    assert!(result.range_change.is_none());
    assert_eq!(result.selection.focus.paragraph, next.paragraphs[1].id);
    assert_eq!(result.selection.focus.scalar_offset, 2);
    assert_eq!(edit(&base, &q).into_candidate(), result.into_candidate());
    let result = edit(&base, &q);
    let undone = history(
        &result.prepared.snapshot,
        &base,
        &result.transaction,
        HistoryDirection::Undo,
    );
    assert_eq!(undone.document(), base.document());
    let next = &result.prepared.snapshot;
    let mut again = q.clone();
    again.base_revision = next.revision().clone();
    assert!(prepare_text_edit(next, &again, Default::default(), &|| false).is_err());
    let empty = edit(
        &base,
        &command(
            &base,
            Some("cell:2:2"),
            TextEditAction::Initialize {
                text: String::new(),
                setup: setup(),
            },
        ),
    );
    assert_eq!(plain(body(&empty.prepared.snapshot, "cell:2:2")), vec![""]);
}
#[test]
fn explicit_cell_scope_rejects_covered_foreign_missing_and_cross_cell_selections() {
    let base = initial();
    let before = base.clone().into_record();
    for cell in [
        None,
        Some("cell:0:1"),
        Some("cell:missing"),
        Some("cell:2:2"),
        Some("cell:0:2"),
    ] {
        let q = command(
            &base,
            cell,
            TextEditAction::Replace {
                selection: selection(&base, "cell:0:0", 0, 0),
                text: "A".into(),
            },
        );
        assert!(prepare_text_edit(&base, &q, Default::default(), &|| false).is_err());
    }
    let q = command(
        &base,
        Some("cell:0:0"),
        TextEditAction::Replace {
            selection: TextSelection {
                anchor: anchor(body(&base, "cell:0:0").paragraphs[0].id.clone(), 0),
                focus: anchor(body(&base, "cell:0:2").paragraphs[0].id.clone(), 0),
            },
            text: "A".into(),
        },
    );
    assert!(prepare_text_edit(&base, &q, Default::default(), &|| false).is_err());
    assert_eq!(base.clone().into_record(), before);
}
#[test]
fn initialization_limits_cancellation_and_revision_are_atomic() {
    let base = initial();
    let q = command(
        &base,
        Some("cell:2:2"),
        TextEditAction::Initialize {
            text: "A😀".into(),
            setup: setup(),
        },
    );
    assert!(matches!(
        prepare_text_edit(&base, &q, Default::default(), &|| true),
        Err(EditError::Cancelled)
    ));
    let mut bad = q.clone();
    bad.base_revision = Digest::from_sha256([0; 32]);
    assert!(matches!(
        prepare_text_edit(&base, &bad, Default::default(), &|| false),
        Err(EditError::RevisionConflict { .. })
    ));
    let mut settings = setup();
    settings.insets.left = Emu::new(-1);
    let invalid = command(
        &base,
        Some("cell:2:2"),
        TextEditAction::Initialize {
            text: "A".into(),
            setup: settings,
        },
    );
    assert!(prepare_text_edit(&base, &invalid, Default::default(), &|| false).is_err());
    let huge = command(
        &base,
        Some("cell:2:2"),
        TextEditAction::Initialize {
            text: "A".repeat(2 * 1024 * 1024),
            setup: setup(),
        },
    );
    assert!(prepare_text_edit(&base, &huge, Default::default(), &|| false).is_err());
    assert!(table(&base).rows[2].cells[2].text.is_none());
}
#[test]
fn shape_initialization_rejects_a_cell_and_omitted_cell_keeps_legacy_digest() {
    let schema: serde_json::Value = serde_json::from_str(include_str!(
        "../../../contracts/generated/document.schema.json"
    ))
    .unwrap();
    let mut document: Document = serde_json::from_value(schema["examples"][0].clone()).unwrap();
    let (id, shape) = document
        .objects
        .iter_mut()
        .find(|(_, o)| matches!(o.content, ObjectContent::Shape { .. }))
        .unwrap();
    let ObjectContent::Shape { text, .. } = &mut shape.content else {
        panic!()
    };
    *text = None;
    let id = id.clone();
    let base = Snapshot::new(document, Default::default()).unwrap();
    let mut q = command(
        &base,
        None,
        TextEditAction::Initialize {
            text: "A😀".into(),
            setup: setup(),
        },
    );
    q.object = id.clone();
    let result = edit(&base, &q);
    let ObjectContent::Shape {
        text: Some(body), ..
    } = &result.prepared.snapshot.document().objects[&id].content
    else {
        panic!()
    };
    assert_eq!(plain(body), vec!["A😀"]);
    assert!(result.range_change.is_none());
    q.cell = Some(cell_id("cell:2:2"));
    assert!(prepare_text_edit(&base, &q, Default::default(), &|| false).is_err());
    q.cell = None;
    let value = serde_json::to_value(&q).unwrap();
    assert!(value.get("cell").is_none());
    assert_eq!(
        text_edit_command_digest(&q).unwrap(),
        digest("musteroffice.text-edit-command/1", &value).unwrap()
    );
}

#[test]
fn replacement_and_initialization_encode_tabs_as_typed_runs_with_exact_scalar_carets() {
    let base = initial();
    let initialized = edit(
        &base,
        &command(
            &base,
            Some("cell:2:2"),
            TextEditAction::Initialize {
                text: "\tA😀\t\t".into(),
                setup: setup(),
            },
        ),
    );
    let next = &initialized.prepared.snapshot;
    let b = body(next, "cell:2:2");
    assert_eq!(plain(b), ["\tA😀\t\t"]);
    assert_eq!(initialized.selection.focus.scalar_offset, 5);
    assert_eq!(
        b.paragraphs[0]
            .runs
            .iter()
            .filter(|r| matches!(r.content, InlineContent::Tab))
            .count(),
        3
    );
    let replaced = edit(
        next,
        &command(
            next,
            Some("cell:2:2"),
            TextEditAction::Replace {
                selection: selection(next, "cell:2:2", 1, 3),
                text: "\t中".into(),
            },
        ),
    );
    assert_eq!(
        plain(body(&replaced.prepared.snapshot, "cell:2:2")),
        ["\t\t中\t\t"]
    );
    assert_eq!(replaced.selection.focus.scalar_offset, 3);
    let undone = history(
        &replaced.prepared.snapshot,
        next,
        &replaced.transaction,
        HistoryDirection::Undo,
    );
    assert_eq!(undone.document(), next.document());
}
