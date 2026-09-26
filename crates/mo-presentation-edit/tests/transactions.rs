mod support;
use mo_common::*;
use mo_presentation_edit::*;
use mo_presentation_model::*;
use support::*;

#[test]
fn changes_separate_metadata_from_static_page_dependencies() {
    let original = snapshot();
    let title = prepare(
        &original,
        &transaction(
            &original,
            vec![Operation::SetTitle {
                title: "renamed".into(),
            }],
        ),
        Default::default(),
    )
    .unwrap();
    assert!(title.receipt.changes.metadata_changed);
    assert!(!title.receipt.changes.invalidate_all_layout);
    assert!(title.receipt.changes.invalidated_slides.is_empty());
    let mut transform = original.document().objects[&id()].transform;
    transform.origin.x = Emu::new(100);
    let moved = prepare(
        &original,
        &transaction(
            &original,
            vec![Operation::SetTransform {
                object: id(),
                transform,
            }],
        ),
        Default::default(),
    )
    .unwrap();
    assert!(!moved.receipt.changes.invalidate_all_layout);
    assert_eq!(moved.receipt.changes.invalidated_slides, vec![slide_id()]);
    let accessibility = prepare(
        &original,
        &transaction(
            &original,
            vec![Operation::SetAccessibility {
                object: id(),
                accessibility: Accessibility {
                    title: "label".into(),
                    ..Default::default()
                },
            }],
        ),
        Default::default(),
    )
    .unwrap();
    assert_eq!(accessibility.receipt.changes.updated_objects, vec![id()]);
    assert!(accessibility.receipt.changes.invalidated_slides.is_empty());
}

pub fn snapshot() -> Snapshot {
    Snapshot::new(document(), ValidationLimits::default()).unwrap()
}

pub fn transaction(snapshot: &Snapshot, operations: Vec<Operation>) -> Transaction {
    Transaction {
        document_id: snapshot.document().id.clone(),
        request_id: RequestId::new("request:1").unwrap(),
        base_revision: snapshot.revision().clone(),
        operations: operations
            .into_iter()
            .enumerate()
            .map(|(i, operation)| OperationEntry {
                operation_id: OperationId::new(format!("op:{i}")).unwrap(),
                operation,
            })
            .collect(),
    }
}

pub fn text(snapshot: &Snapshot) -> &str {
    let ObjectContent::Shape {
        text: Some(body), ..
    } = &snapshot.document().objects[&id()].content
    else {
        panic!("missing shape text")
    };
    let InlineContent::Text { text } = &body.paragraphs[0].runs[0].content else {
        panic!("missing text run")
    };
    text
}

#[test]
fn failure_does_not_publish_earlier_operations() {
    let snapshot = snapshot();
    let before = canonical_bytes(snapshot.document()).unwrap();
    let request = transaction(
        &snapshot,
        vec![
            Operation::SetTitle {
                title: "would leak".into(),
            },
            Operation::DeleteObject {
                object: ObjectId::new("missing").unwrap(),
                policy: DeletePolicy::RejectDependencies,
            },
        ],
    );
    assert!(matches!(
        prepare(&snapshot, &request, ValidationLimits::default()),
        Err(EditError::Operation { .. })
    ));
    assert_eq!(canonical_bytes(snapshot.document()).unwrap(), before);
    assert_eq!(snapshot.document().title, "");
}

#[test]
fn final_reference_validation_rolls_back_entire_transaction() {
    let snapshot = snapshot();
    let request = transaction(
        &snapshot,
        vec![
            Operation::SetTitle {
                title: "unpublished".into(),
            },
            Operation::SetLayout {
                slide: slide_id(),
                layout: Some(LayoutId::new("missing").unwrap()),
            },
        ],
    );
    let Err(EditError::InvalidDocument(report)) =
        prepare(&snapshot, &request, ValidationLimits::default())
    else {
        panic!("expected semantic failure")
    };
    assert!(
        report
            .issues
            .iter()
            .any(|i| i.code == ValidationCode::MissingReference)
    );
    assert_eq!(snapshot.document().title, "");
    assert!(snapshot.document().slides[&slide_id()].layout.is_none());
}

#[test]
fn revision_conflicts_do_not_rebase_and_retries_use_original_receipt() {
    let snapshot = snapshot();
    let request = transaction(
        &snapshot,
        vec![Operation::SetTitle {
            title: "committed".into(),
        }],
    );
    let prepared = prepare(&snapshot, &request, ValidationLimits::default()).unwrap();
    assert_ne!(prepared.snapshot.revision(), snapshot.revision());
    assert!(matches!(
        prepare(&prepared.snapshot, &request, ValidationLimits::default()),
        Err(EditError::RevisionConflict { .. })
    ));
    check_replay(&request, &prepared.receipt).unwrap();
    let mut reused = request.clone();
    reused.operations[0].operation = Operation::SetTitle {
        title: "different".into(),
    };
    assert!(matches!(
        check_replay(&reused, &prepared.receipt),
        Err(EditError::RequestIdReused)
    ));
    assert_eq!(
        prepare(&snapshot, &request, ValidationLimits::default())
            .unwrap()
            .receipt,
        prepared.receipt
    );
}

#[test]
fn scalar_splice_preserves_emoji_combining_marks_and_run_identity() {
    let snapshot = snapshot();
    let request = transaction(
        &snapshot,
        vec![Operation::SpliceText {
            object: id(),
            paragraph: paragraph_id(),
            run: run_id(),
            start: 1,
            delete: 1,
            insert: "火箭🚀".into(),
        }],
    );
    let prepared = prepare(&snapshot, &request, ValidationLimits::default()).unwrap();
    assert_eq!(text(&prepared.snapshot), "A火箭🚀e\u{301}中");
    assert_eq!(text(&snapshot), "A😀e\u{301}中");
    assert_eq!(prepared.receipt.changes.updated_objects, vec![id()]);
    let map = &prepared.receipt.changes.anchor_maps[0];
    assert_eq!((map.start, map.deleted, map.inserted), (1, 1, 3));
    for (offset, affinity, expected) in [
        (0, Affinity::After, 0),
        (1, Affinity::Before, 1),
        (1, Affinity::After, 4),
        (2, Affinity::Before, 4),
        (5, Affinity::After, 7),
    ] {
        let mapped = map
            .map(&TextAnchor {
                paragraph: paragraph_id(),
                scalar_offset: offset,
                affinity,
            })
            .unwrap();
        assert_eq!(mapped.scalar_offset, expected);
    }
}

#[test]
fn out_of_range_splices_fail_without_mutating_content() {
    let snapshot = snapshot();
    for (start, delete) in [(6, 0), (0, 6), (u32::MAX, 1)] {
        let request = transaction(
            &snapshot,
            vec![Operation::SpliceText {
                object: id(),
                paragraph: paragraph_id(),
                run: run_id(),
                start,
                delete,
                insert: String::new(),
            }],
        );
        assert!(prepare(&snapshot, &request, ValidationLimits::default()).is_err());
        assert_eq!(text(&snapshot), "A😀e\u{301}中");
    }
}

#[test]
fn deletion_rejects_dependencies_unless_explicitly_cascaded() {
    let mut doc = document();
    let connector = connector(&mut doc);
    let snapshot = Snapshot::new(doc, ValidationLimits::default()).unwrap();
    let request = transaction(
        &snapshot,
        vec![Operation::DeleteObject {
            object: id(),
            policy: DeletePolicy::RejectDependencies,
        }],
    );
    assert!(prepare(&snapshot, &request, ValidationLimits::default()).is_err());
    let request = transaction(
        &snapshot,
        vec![Operation::DeleteObject {
            object: id(),
            policy: DeletePolicy::Cascade,
        }],
    );
    let prepared = prepare(&snapshot, &request, ValidationLimits::default()).unwrap();
    assert!(prepared.snapshot.document().objects.is_empty());
    assert!(
        prepared.snapshot.document().slides[&slide_id()]
            .objects
            .is_empty()
    );
    assert_eq!(
        prepared.receipt.changes.deleted_objects,
        vec![connector, id()]
    );
    assert_eq!(snapshot.document().objects.len(), 2);
}

#[test]
fn ownership_cycle_cannot_be_committed() {
    let mut doc = document();
    let group_id = ObjectId::new("group:1").unwrap();
    let mut group = doc.objects[&id()].clone();
    group.id = group_id.clone();
    group.content = ObjectContent::Group {
        children: vec![],
        viewport: doc.page_size,
    };
    doc.objects.insert(group_id.clone(), group);
    doc.slides
        .get_mut(&slide_id())
        .unwrap()
        .objects
        .push(group_id.clone());
    let snapshot = Snapshot::new(doc, ValidationLimits::default()).unwrap();
    let request = transaction(
        &snapshot,
        vec![Operation::MoveObject {
            object: group_id.clone(),
            parent: ContainerId::Group(group_id),
            index: 0,
            transform: snapshot.document().objects[&id()].transform,
        }],
    );
    let Err(EditError::InvalidDocument(report)) =
        prepare(&snapshot, &request, ValidationLimits::default())
    else {
        panic!("expected cycle failure")
    };
    assert!(
        report
            .issues
            .iter()
            .any(|issue| issue.code == ValidationCode::ReferenceCycle)
    );
}

#[test]
fn unknown_fields_and_variants_are_rejected_at_wire_boundary() {
    let snapshot = snapshot();
    let request = transaction(&snapshot, vec![Operation::SetTitle { title: "ok".into() }]);
    let mut wire = serde_json::to_value(request).unwrap();
    wire["operations"][0]["operation"]["arbitraryScript"] = serde_json::json!("run()");
    assert!(serde_json::from_value::<Transaction>(wire).is_err());
    let mut wire = serde_json::to_value(snapshot.document()).unwrap();
    wire["objects"][id().as_str()]["content"]["kind"] = serde_json::json!("unimplementedFeature");
    assert!(serde_json::from_value::<Document>(wire).is_err());
}
