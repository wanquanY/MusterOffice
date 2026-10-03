mod support;
use mo_common::*;
use mo_presentation_edit::*;
use mo_presentation_model::*;
use support::*;

fn initial() -> Snapshot {
    let mut doc = document();
    connector(&mut doc);
    let ObjectContent::Shape {
        text: Some(body), ..
    } = &mut doc.objects.get_mut(&id()).unwrap().content
    else {
        panic!()
    };
    let first = &mut body.paragraphs[0].runs[0];
    first.content = InlineContent::Text {
        text: "A😀".into()
    };
    first.style.color = Inherited::Value(Color::Theme {
        slot: ThemeColor::Accent1,
    });
    let mut second = first.clone();
    second.id = RunId::new("run:2").unwrap();
    second.content = InlineContent::Text {
        text: "e\u{301}中".into(),
    };
    second.style.italic = Inherited::Value(true);
    body.paragraphs[0].runs.push(second);
    Snapshot::new(doc, Default::default()).unwrap()
}
fn anchor(paragraph: ParagraphId, scalar_offset: u32) -> TextAnchor {
    TextAnchor {
        paragraph,
        scalar_offset,
        affinity: Affinity::After,
    }
}
fn selection(start: u32, end: u32) -> TextSelection {
    TextSelection {
        anchor: anchor(paragraph_id(), start),
        focus: anchor(paragraph_id(), end),
    }
}
fn command(base: &Snapshot, action: TextEditAction) -> TextEditCommand {
    TextEditCommand {
        document_id: base.document().id.clone(),
        base_revision: base.revision().clone(),
        request_id: RequestId::new("text:1").unwrap(),
        operation_id: OperationId::new("text-op:1").unwrap(),
        object: id(),
        cell: None,
        action,
    }
}
fn body(snapshot: &Snapshot) -> &TextBody {
    let ObjectContent::Shape {
        text: Some(body), ..
    } = &snapshot.document().objects[&id()].content
    else {
        panic!()
    };
    body
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
fn run_text(run: &TextRun) -> &str {
    let InlineContent::Text { text } = &run.content else {
        panic!()
    };
    text
}
fn prepare(base: &Snapshot, action: TextEditAction) -> PreparedTextEdit {
    prepare_text_edit(base, &command(base, action), Default::default(), &|| false).unwrap()
}

#[test]
fn replacement_across_runs_keeps_surviving_styles_identity_and_resources() {
    let base = initial();
    let result = prepare(
        &base,
        TextEditAction::Replace {
            selection: selection(1, 4),
            text: "新🚀".into(),
        },
    );
    let next = body(&result.prepared.snapshot);
    assert_eq!(plain(next), vec!["A新🚀中"]);
    assert_eq!(next.paragraphs[0].runs[0].id, run_id());
    assert_eq!(
        next.paragraphs[0].runs.last().unwrap().id,
        RunId::new("run:2").unwrap()
    );
    assert_eq!(
        next.paragraphs[0].runs.last().unwrap().style.italic,
        Inherited::Value(true)
    );
    assert_eq!(
        next.paragraphs[0].runs[1].style.color,
        body(&base).paragraphs[0].runs[0].style.color
    );
    assert_eq!(result.selection.focus.scalar_offset, 3);
    assert_eq!(
        base.document().resources,
        result.prepared.snapshot.document().resources
    );
    assert_eq!(plain(body(&base)), vec!["A😀e\u{301}中"]);
    let original_command = command(
        &base,
        TextEditAction::Replace {
            selection: selection(1, 4),
            text: "新🚀".into(),
        },
    );
    assert_eq!(
        result.command_digest,
        text_edit_command_digest(&original_command).unwrap()
    );
}

#[test]
fn paragraph_split_join_maps_tail_anchors_and_can_be_undone_by_the_same_history_engine() {
    let base = initial();
    let split = prepare(
        &base,
        TextEditAction::Replace {
            selection: selection(2, 2),
            text: "\r\n第二\r".into(),
        },
    );
    let next = &split.prepared.snapshot;
    assert_eq!(plain(body(next)), vec!["A😀", "第二", "e\u{301}中"]);
    let mapping = split.range_change.as_ref().unwrap();
    let tail = mapping.map(&anchor(paragraph_id(), 5)).unwrap();
    assert_eq!(tail.paragraph, body(next).paragraphs[2].id);
    assert_eq!(tail.scalar_offset, 3);
    assert_eq!(
        mapping.reversed().map(&tail).unwrap(),
        anchor(paragraph_id(), 5)
    );
    let join_range = TextSelection {
        anchor: anchor(paragraph_id(), 2),
        focus: anchor(body(next).paragraphs[2].id.clone(), 0),
    };
    let joined = prepare(
        next,
        TextEditAction::Replace {
            selection: join_range,
            text: String::new(),
        },
    );
    assert_eq!(plain(body(&joined.prepared.snapshot)), plain(body(&base)));
    let undo = prepare_history(
        next,
        &HistoryTransaction {
            document_id: next.document().id.clone(),
            request_id: RequestId::new("undo-text:1").unwrap(),
            base_revision: next.revision().clone(),
            direction: HistoryDirection::Undo,
            original_snapshot: base.clone().into_record(),
            original_transaction: split.transaction,
        },
        Default::default(),
        &|| false,
    )
    .unwrap();
    assert_eq!(undo.snapshot.document(), base.document());
}

#[test]
fn local_style_preserves_other_declarations_and_reversed_selection() {
    let base = initial();
    let result = prepare(
        &base,
        TextEditAction::SetCharacterStyle {
            selection: selection(4, 1),
            patch: CharacterStylePatch {
                bold: Some(Inherited::Value(true)),
                ..Default::default()
            },
        },
    );
    let p = &body(&result.prepared.snapshot).paragraphs[0];
    assert_eq!(plain(body(&result.prepared.snapshot)), plain(body(&base)));
    assert_eq!(
        p.runs.iter().map(run_text).collect::<Vec<_>>(),
        vec!["A", "😀", "e\u{301}", "中"]
    );
    assert_eq!(p.runs[0].style.bold, Inherited::Inherit);
    assert_eq!(p.runs[1].style.bold, Inherited::Value(true));
    assert_eq!(p.runs[2].style.bold, Inherited::Value(true));
    assert_eq!(p.runs[3].style.bold, Inherited::Inherit);
    assert_eq!(p.runs[2].style.italic, Inherited::Value(true));
    assert_eq!(p.runs[1].style.font, Inherited::Inherit);
    assert_eq!(p.runs[0].style.color, p.runs[1].style.color);
    assert_eq!(p.runs[0].id, run_id());
    assert_ne!(p.runs[0].id, p.runs[1].id);
    assert_eq!(result.selection, selection(4, 1));
}

#[test]
fn empty_runs_outside_selected_text_are_preserved() {
    let base = initial();
    let mut doc = base.document().clone();
    let ObjectContent::Shape {
        text: Some(text), ..
    } = &mut doc.objects.get_mut(&id()).unwrap().content
    else {
        panic!()
    };
    let empty = TextRun {
        id: RunId::new("empty:1").unwrap(),
        style: CharacterStyle {
            underline: Inherited::Value(true),
            ..Default::default()
        },
        content: InlineContent::Text {
            text: String::new(),
        },
    };
    text.paragraphs[0].runs.push(empty.clone());
    let base = Snapshot::new(doc, Default::default()).unwrap();
    let result = prepare(
        &base,
        TextEditAction::SetCharacterStyle {
            selection: selection(0, 1),
            patch: CharacterStylePatch {
                italic: Some(Inherited::Value(true)),
                ..Default::default()
            },
        },
    );
    assert_eq!(
        body(&result.prepared.snapshot).paragraphs[0].runs.last(),
        Some(&empty)
    );
}

#[test]
fn rejects_interior_graphemes_invalid_styles_and_oversized_input_atomically() {
    let base = initial();
    let before = base.clone().into_record();
    for action in [
        TextEditAction::Replace {
            selection: selection(3, 4),
            text: "x".into(),
        },
        TextEditAction::Replace {
            selection: selection(0, 6),
            text: "x".into(),
        },
        TextEditAction::Replace {
            selection: selection(0, 1),
            text: "x".repeat(65537),
        },
        TextEditAction::SetCharacterStyle {
            selection: selection(1, 2),
            patch: CharacterStylePatch {
                font: Some(Inherited::Value(FontId::new("absent").unwrap())),
                ..Default::default()
            },
        },
    ] {
        assert!(
            prepare_text_edit(&base, &command(&base, action), Default::default(), &|| {
                false
            })
            .is_err()
        );
        assert_eq!(base.clone().into_record(), before);
    }
}

#[test]
fn request_is_deterministic_and_revision_and_cancellation_are_enforced() {
    let base = initial();
    let request = command(
        &base,
        TextEditAction::Replace {
            selection: selection(1, 1),
            text: "中文".into(),
        },
    );
    let result = prepare_text_edit(&base, &request, Default::default(), &|| false).unwrap();
    let replay = prepare_text_edit(&base, &request, Default::default(), &|| false).unwrap();
    assert_eq!(
        result.prepared.snapshot.clone().into_record(),
        replay.prepared.snapshot.into_record()
    );
    assert_eq!(result.transaction, replay.transaction);
    assert!(matches!(
        prepare_text_edit(
            &result.prepared.snapshot,
            &request,
            Default::default(),
            &|| false
        ),
        Err(EditError::RevisionConflict { .. })
    ));
    assert!(matches!(
        prepare_text_edit(&base, &request, Default::default(), &|| true),
        Err(EditError::Cancelled)
    ));
}

#[test]
fn insertion_that_joins_a_grapheme_returns_a_valid_caret() {
    let base = initial();
    let mut doc = base.document().clone();
    let ObjectContent::Shape {
        text: Some(text), ..
    } = &mut doc.objects.get_mut(&id()).unwrap().content
    else {
        panic!()
    };
    text.paragraphs[0].runs.truncate(1);
    text.paragraphs[0].runs[0].content = InlineContent::Text {
        text: "👩🚀".into(),
    };
    let base = Snapshot::new(doc, Default::default()).unwrap();
    let result = prepare(
        &base,
        TextEditAction::Replace {
            selection: selection(1, 1),
            text: "\u{200d}".into(),
        },
    );
    assert_eq!(plain(body(&result.prepared.snapshot)), vec!["👩\u{200d}🚀"]);
    assert_eq!(
        result
            .range_change
            .as_ref()
            .unwrap()
            .after
            .focus
            .scalar_offset,
        2
    );
    assert_eq!(result.selection.focus.scalar_offset, 3);
    // The next command accepts the returned selection without UI-side repair.
    let next = prepare(
        &result.prepared.snapshot,
        TextEditAction::Replace {
            selection: result.selection,
            text: "!".into(),
        },
    );
    assert_eq!(plain(body(&next.prepared.snapshot)), vec!["👩\u{200d}🚀!"]);
}

#[test]
fn resulting_paragraphs_remain_within_the_segmentation_budget() {
    let base = initial();
    let request = command(
        &base,
        TextEditAction::Replace {
            selection: selection(1, 1),
            text: "x".repeat(65536),
        },
    );
    assert!(prepare_text_edit(&base, &request, Default::default(), &|| false).is_err());
}
