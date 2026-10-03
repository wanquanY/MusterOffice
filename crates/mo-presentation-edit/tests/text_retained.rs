#[allow(dead_code)]
mod support;
use mo_common::*;
use mo_presentation_edit::*;
use mo_presentation_model::*;
use std::{cell::Cell, collections::BTreeMap};
use support::*;

fn native(parts: &[(RetainedRunKind, &str)]) -> Snapshot {
    let mut d = document();
    let resource = ResourceId::new("source").unwrap();
    d.resources.insert(
        resource.clone(),
        Resource {
            id: resource.clone(),
            kind: ResourceKind::SourcePackage,
            media_type: "application/vnd.openxmlformats-officedocument.presentationml.presentation"
                .into(),
            sha256: Digest::from_sha256([0; 32]),
        },
    );
    let paragraphs = vec![RetainedParagraph {
        id: paragraph_id(),
        runs: parts
            .iter()
            .enumerate()
            .map(|(i, (kind, text))| RetainedTextRun {
                id: RunId::new(format!("native:{i}")).unwrap(),
                kind: *kind,
                text: (*text).into(),
            })
            .collect(),
    }];
    let runs = paragraphs[0]
        .runs
        .iter()
        .enumerate()
        .map(|(i, r)| {
            (
                r.id.clone(),
                NativeRunBinding {
                    paragraph: 0,
                    run: i as u32,
                    constraint: (r.kind != RetainedRunKind::Text)
                        .then_some(NativeEditConstraint::StructuredLeaf),
                },
            )
        })
        .collect();
    d.objects.get_mut(&id()).unwrap().content = ObjectContent::RetainedSource {
        native_kind: RetainedObjectKind::Shape,
        children: vec![],
        paragraphs,
    };
    d.source_bindings = Some(SourceBindings {
        profile: SourceBindingProfile::PresentationmlRetainedFieldsV4,
        identity_scope: None,
        resource,
        slides: BTreeMap::from([(slide_id(), "/ppt/slides/slide1.xml".into())]),
        masters: BTreeMap::new(),
        layouts: BTreeMap::new(),
        themes: BTreeMap::new(),
        objects: BTreeMap::from([(
            id(),
            NativeObjectBinding {
                part: "/ppt/slides/slide1.xml".into(),
                native_id: 2,
                transform_constraint: None,
                runs,
            },
        )]),
    });
    Snapshot::new(d, Default::default()).unwrap()
}
fn anchor(offset: u32) -> TextAnchor {
    TextAnchor {
        paragraph: paragraph_id(),
        scalar_offset: offset,
        affinity: Affinity::After,
    }
}
fn selection(a: u32, f: u32) -> TextSelection {
    TextSelection {
        anchor: anchor(a),
        focus: anchor(f),
    }
}
fn command(s: &Snapshot, selection: TextSelection, text: &str) -> TextEditCommand {
    TextEditCommand {
        document_id: s.document().id.clone(),
        request_id: RequestId::new("native:edit").unwrap(),
        base_revision: s.revision().clone(),
        operation_id: OperationId::new("native:intent").unwrap(),
        object: id(),
        cell: None,
        action: TextEditAction::Replace {
            selection,
            text: text.into(),
        },
    }
}
fn paras(s: &Snapshot) -> &[RetainedParagraph] {
    let ObjectContent::RetainedSource { paragraphs, .. } = &s.document().objects[&id()].content
    else {
        panic!()
    };
    paragraphs
}
fn plain(s: &Snapshot) -> String {
    paras(s)[0]
        .runs
        .iter()
        .map(|r| {
            if r.kind == RetainedRunKind::Break {
                "\u{2028}"
            } else {
                &r.text
            }
        })
        .collect()
}
fn edit(s: &Snapshot, a: u32, f: u32, text: &str) -> PreparedTextEdit {
    prepare_text_edit(
        s,
        &command(s, selection(a, f), text),
        Default::default(),
        &|| false,
    )
    .unwrap()
}
fn capabilities(s: &Snapshot, selection: Option<TextSelection>) -> TextEditingCapabilities {
    text_capabilities(
        s,
        &TextCapabilitiesQuery {
            object: id(),
            cell: None,
            selection,
        },
        &|| false,
    )
    .unwrap()
}
fn unavailable(value: &TextEditAvailability) -> &TextEditRestriction {
    let TextEditAvailability::Unavailable { reason } = value else {
        panic!("{value:?}")
    };
    reason
}

#[test]
fn cross_run_unicode_replacement_keeps_bindings_and_maps_the_complete_intent() {
    use RetainedRunKind::*;
    let s = native(&[
        (Text, "A😀"),
        (Text, "e"),
        (Text, "\u{301}中"),
        (Break, ""),
        (Text, "尾"),
    ]);
    let result = edit(&s, 4, 1, "α🚀");
    let next = &result.prepared.snapshot;
    assert_eq!(plain(next), "Aα🚀中\u{2028}尾");
    assert_eq!(
        next.document().source_bindings,
        s.document().source_bindings
    );
    assert_eq!(next.document().resources, s.document().resources);
    assert_eq!(
        paras(next)[0]
            .runs
            .iter()
            .map(|r| (&r.id, r.kind))
            .collect::<Vec<_>>(),
        paras(&s)[0]
            .runs
            .iter()
            .map(|r| (&r.id, r.kind))
            .collect::<Vec<_>>()
    );
    assert_eq!(paras(next)[0].runs[1].text, "");
    assert_eq!(paras(next)[0].runs[2].text, "中");
    assert_eq!(result.transaction.operations.len(), 3);
    assert_eq!(result.selection.focus.scalar_offset, 3);
    let change = result.range_change.as_ref().unwrap();
    for offset in 0..=7 {
        for affinity in [Affinity::Before, Affinity::After] {
            let a = TextAnchor {
                affinity,
                ..anchor(offset)
            };
            let complete = AnchorMap {
                paragraph: paragraph_id(),
                start: 1,
                deleted: 3,
                inserted: 2,
            };
            assert_eq!(
                complete.map(&a).unwrap(),
                change.map(&a).unwrap(),
                "offset {offset} {affinity:?}"
            );
        }
    }
    assert_eq!(plain(&s), "A😀e\u{301}中\u{2028}尾");
    let again = edit(&s, 4, 1, "α🚀");
    assert_eq!(again.transaction, result.transaction);
    assert_eq!(
        again.prepared.snapshot.into_record(),
        next.clone().into_record()
    );
    // Primitive receipts remain identical to ordinary preparation/history;
    // they are not rewritten to disguise the original transaction semantics.
    let replay = prepare(&s, &result.transaction, Default::default()).unwrap();
    assert_eq!(replay.receipt, result.prepared.receipt);
}

#[test]
fn replacement_resegments_joined_graphemes_and_bounds_surviving_text() {
    use RetainedRunKind::*;
    let s = native(&[(Text, "x\u{301}")]);
    let result = edit(&s, 0, 0, "e");
    assert_eq!(result.selection.focus.scalar_offset, 1);
    let s = native(&[(Text, "\u{301}")]);
    let result = edit(&s, 0, 0, "e");
    assert_eq!(result.range_change.unwrap().after.focus.scalar_offset, 1);
    assert_eq!(result.selection.focus.scalar_offset, 2);
    let text = "A".repeat(262_144);
    let s = native(&[(Text, &text)]);
    assert!(
        prepare_text_edit(
            &s,
            &command(&s, selection(0, 0), "A"),
            Default::default(),
            &|| false
        )
        .is_err()
    );
    assert_eq!(plain(&s), text);
}

#[test]
fn insertion_affinity_empty_leaves_and_tab_keep_original_leaf_identity() {
    use RetainedRunKind::*;
    let s = native(&[(Text, "A"), (Text, ""), (Text, "中")]);
    for (affinity, expected) in [
        (Affinity::Before, vec!["Aα\t", "", "中"]),
        (Affinity::After, vec!["A", "", "α\t中"]),
    ] {
        let a = TextAnchor {
            affinity,
            ..anchor(1)
        };
        let q = command(
            &s,
            TextSelection {
                anchor: a.clone(),
                focus: a,
            },
            "α\t",
        );
        let result = prepare_text_edit(&s, &q, Default::default(), &|| false).unwrap();
        assert_eq!(
            paras(&result.prepared.snapshot)[0]
                .runs
                .iter()
                .map(|r| r.text.as_str())
                .collect::<Vec<_>>(),
            expected
        );
        assert_eq!(result.selection.focus.scalar_offset, 3);
    }
    let s = native(&[(Text, "")]);
    assert_eq!(plain(&edit(&s, 0, 0, "😀").prepared.snapshot), "😀");
    let empty = edit(&s, 0, 0, "");
    assert_eq!(empty.transaction.operations.len(), 1);
    assert_eq!(empty.prepared.snapshot.document(), s.document());
}

#[test]
fn protected_insertion_style_owner_does_not_prevent_plain_deletion() {
    use RetainedRunKind::*;
    let s = native(&[(Field, "1"), (Text, "中")]);
    let mut selected = selection(1, 2);
    selected.anchor.affinity = Affinity::Before;
    let caps = capabilities(&s, Some(selected.clone()));
    assert!(matches!(
        unavailable(&caps.replace),
        TextEditRestriction::StructuredRun {
            run_kind: Field,
            ..
        }
    ));
    assert_eq!(caps.delete, TextEditAvailability::Available);
    let rejected = prepare_text_edit(
        &s,
        &command(&s, selected.clone(), "A"),
        Default::default(),
        &|| false,
    )
    .err()
    .unwrap()
    .diagnostic();
    assert_eq!(
        rejected.text_restriction.as_ref(),
        Some(unavailable(&caps.replace))
    );
    let deleted = prepare_text_edit(&s, &command(&s, selected, ""), Default::default(), &|| {
        false
    })
    .unwrap();
    assert_eq!(plain(&deleted.prepared.snapshot), "1");
    assert_eq!(plain(&edit(&s, 1, 2, "A").prepared.snapshot), "1A");
}

#[test]
fn all_native_constraints_and_structured_runs_are_atomic_and_queryable() {
    use RetainedRunKind::*;
    for constraint in [
        NativeEditConstraint::MissingDirectTransform,
        NativeEditConstraint::RetainedTransform,
        NativeEditConstraint::CompatibilityBranch,
        NativeEditConstraint::StructuredLeaf,
        NativeEditConstraint::DynamicField,
        NativeEditConstraint::TimingReferences,
        NativeEditConstraint::RetainedReferences,
    ] {
        let base = native(&[(Text, "A"), (Text, "中")]);
        let mut d = base.document().clone();
        d.source_bindings
            .as_mut()
            .unwrap()
            .objects
            .get_mut(&id())
            .unwrap()
            .runs
            .get_mut(&paras(&base)[0].runs[1].id)
            .unwrap()
            .constraint = Some(constraint);
        let s = Snapshot::new(d, Default::default()).unwrap();
        let caps = capabilities(&s, Some(selection(0, 2)));
        let q = command(&s, selection(0, 2), "α");
        let before = s.clone().into_record();
        let error = prepare_text_edit(&s, &q, Default::default(), &|| false)
            .err()
            .unwrap();
        assert_eq!(
            error.diagnostic().text_restriction.as_ref(),
            Some(unavailable(&caps.replace))
        );
        assert_eq!(before, s.clone().into_record());
        assert_eq!(plain(&edit(&s, 0, 1, "α").prepared.snapshot), "α中");
    }
    for kind in [Break, Field] {
        let s = native(&[
            (Text, "A"),
            (kind, if kind == Field { "1" } else { "" }),
            (Text, "中"),
        ]);
        assert!(matches!(
            unavailable(&capabilities(&s, Some(selection(0, 3))).replace),
            TextEditRestriction::StructuredRun { .. }
        ));
        assert!(
            prepare_text_edit(
                &s,
                &command(&s, selection(0, 3), ""),
                Default::default(),
                &|| false
            )
            .is_err()
        );
    }
}

#[test]
fn native_structure_empty_paragraph_and_malformed_selection_never_convert_to_authored_text() {
    use RetainedRunKind::*;
    let s = native(&[(Text, "e\u{301}😀")]);
    for text in ["\n", "\r", "\r\n", "\u{2028}", "\u{2029}"] {
        let e = prepare_text_edit(
            &s,
            &command(&s, selection(0, 2), text),
            Default::default(),
            &|| false,
        )
        .err()
        .unwrap();
        assert_eq!(
            e.diagnostic().text_restriction,
            Some(TextEditRestriction::NativeStructureRequired)
        );
    }
    for selected in [
        selection(1, 2),
        selection(0, 4),
        TextSelection {
            anchor: anchor(0),
            focus: TextAnchor {
                paragraph: ParagraphId::new("missing").unwrap(),
                ..anchor(0)
            },
        },
    ] {
        assert!(
            text_capabilities(
                &s,
                &TextCapabilitiesQuery {
                    object: id(),
                    cell: None,
                    selection: Some(selected.clone())
                },
                &|| false
            )
            .is_err()
        );
        assert!(
            prepare_text_edit(&s, &command(&s, selected, "A"), Default::default(), &|| {
                false
            })
            .is_err()
        );
    }
    let mut d = s.document().clone();
    let ObjectContent::RetainedSource { paragraphs, .. } =
        &mut d.objects.get_mut(&id()).unwrap().content
    else {
        panic!()
    };
    paragraphs.push(RetainedParagraph {
        id: ParagraphId::new("other").unwrap(),
        runs: vec![],
    });
    let s = Snapshot::new(d, Default::default()).unwrap();
    let selected = TextSelection {
        anchor: anchor(0),
        focus: TextAnchor {
            paragraph: ParagraphId::new("other").unwrap(),
            ..anchor(0)
        },
    };
    assert_eq!(
        unavailable(&capabilities(&s, Some(selected)).replace),
        &TextEditRestriction::RetainedParagraphBoundary
    );
    let empty = native(&[]);
    assert_eq!(
        unavailable(&capabilities(&empty, Some(selection(0, 0))).replace),
        &TextEditRestriction::NativeInsertionTarget
    );
    assert_eq!(
        unavailable(&capabilities(&s, None).replace),
        &TextEditRestriction::SelectionRequired
    );
}

#[test]
fn retained_history_uses_normal_transactions_and_preserves_later_title() {
    let s = native(&[(RetainedRunKind::Text, "A"), (RetainedRunKind::Text, "中")]);
    let edited = edit(&s, 0, 2, "α");
    let current = &edited.prepared.snapshot;
    let title = prepare(
        current,
        &Transaction {
            document_id: current.document().id.clone(),
            base_revision: current.revision().clone(),
            request_id: RequestId::new("title").unwrap(),
            operations: vec![OperationEntry {
                operation_id: OperationId::new("title-op").unwrap(),
                operation: Operation::SetTitle {
                    title: "Later title".into(),
                },
            }],
        },
        Default::default(),
    )
    .unwrap();
    let h = HistoryTransaction {
        document_id: s.document().id.clone(),
        base_revision: title.snapshot.revision().clone(),
        request_id: RequestId::new("undo").unwrap(),
        direction: HistoryDirection::Undo,
        original_snapshot: s.clone().into_record(),
        original_transaction: edited.transaction.clone(),
    };
    let undone = prepare_history(&title.snapshot, &h, Default::default(), &|| false).unwrap();
    assert_eq!(plain(&undone.snapshot), plain(&s));
    assert_eq!(undone.snapshot.document().title, "Later title");
    let redone = prepare_history(
        &undone.snapshot,
        &HistoryTransaction {
            base_revision: undone.snapshot.revision().clone(),
            direction: HistoryDirection::Redo,
            request_id: RequestId::new("redo").unwrap(),
            ..h
        },
        Default::default(),
        &|| false,
    )
    .unwrap();
    assert_eq!(plain(&redone.snapshot), "α");
    assert_eq!(
        redone.snapshot.document().source_bindings,
        s.document().source_bindings
    );
}

#[test]
fn cancellation_input_limits_and_revision_guards_leave_the_snapshot_unchanged() {
    let s = native(&[(RetainedRunKind::Text, "A"), (RetainedRunKind::Text, "中")]);
    let q = command(&s, selection(0, 2), "α");
    let count = Cell::new(0);
    prepare_text_edit(&s, &q, Default::default(), &|| {
        count.set(count.get() + 1);
        false
    })
    .unwrap();
    for at in 1..=count.get() {
        let seen = Cell::new(0);
        assert!(
            matches!(
                prepare_text_edit(&s, &q, Default::default(), &|| {
                    seen.set(seen.get() + 1);
                    seen.get() == at
                }),
                Err(EditError::Cancelled) | Err(EditError::Operation { .. })
            ),
            "{at}"
        );
    }
    let query = TextCapabilitiesQuery {
        object: id(),
        cell: None,
        selection: Some(selection(0, 2)),
    };
    assert!(matches!(
        text_capabilities(&s, &query, &|| true),
        Err(EditError::Cancelled)
    ));
    let mut stale = q.clone();
    stale.base_revision = Digest::from_sha256([1; 32]);
    assert!(matches!(
        prepare_text_edit(&s, &stale, Default::default(), &|| false),
        Err(EditError::RevisionConflict { .. })
    ));
    let oversized = command(&s, selection(0, 1), &"A".repeat(262145));
    assert!(prepare_text_edit(&s, &oversized, Default::default(), &|| false).is_err());
    assert!(
        prepare_text_edit(
            &s,
            &command(&s, selection(0, 1), "\0"),
            Default::default(),
            &|| false
        )
        .is_err()
    );
    assert_eq!(plain(&s), "A中");
}
