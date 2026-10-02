mod support;
use mo_common::*;
use mo_presentation_edit::*;
use mo_presentation_model::*;
use support::*;

fn edit(base: &Snapshot, name: &str, operations: Vec<Operation>) -> Transaction {
    Transaction {
        document_id: base.document().id.clone(),
        request_id: RequestId::new(name).unwrap(),
        base_revision: base.revision().clone(),
        operations: operations
            .into_iter()
            .enumerate()
            .map(|(i, operation)| OperationEntry {
                operation_id: OperationId::new(format!("{name}:{i}")).unwrap(),
                operation,
            })
            .collect(),
    }
}

fn history(
    current: &Snapshot,
    original: &Snapshot,
    transaction: &Transaction,
    direction: HistoryDirection,
) -> HistoryTransaction {
    HistoryTransaction {
        document_id: current.document().id.clone(),
        request_id: RequestId::new(match direction {
            HistoryDirection::Undo => "undo:1",
            HistoryDirection::Redo => "redo:1",
        })
        .unwrap(),
        base_revision: current.revision().clone(),
        direction,
        original_snapshot: original.clone().into_record(),
        original_transaction: transaction.clone(),
    }
}

fn apply(base: &Snapshot, transaction: &Transaction) -> Snapshot {
    prepare(base, transaction, Default::default())
        .unwrap()
        .snapshot
}

#[test]
fn unicode_history_preserves_independent_changes_and_produces_new_revisions() {
    let original = Snapshot::new(document(), Default::default()).unwrap();
    let transaction = edit(
        &original,
        "type:1",
        vec![Operation::SpliceText {
            object: id(),
            paragraph: paragraph_id(),
            run: run_id(),
            start: 1,
            delete: 1,
            insert: "火箭🚀".into(),
        }],
    );
    let applied = apply(&original, &transaction);
    let renamed = apply(
        &applied,
        &edit(
            &applied,
            "rename:1",
            vec![Operation::SetTitle {
                title: "independent".into(),
            }],
        ),
    );
    let undo_request = history(&renamed, &original, &transaction, HistoryDirection::Undo);
    let undo = prepare_history(&renamed, &undo_request, Default::default(), &|| false).unwrap();
    assert_eq!(
        undo.snapshot.document().objects,
        original.document().objects
    );
    assert_eq!(undo.snapshot.document().title, "independent");
    assert_ne!(undo.snapshot.revision(), original.revision());
    assert_ne!(undo.snapshot.revision(), renamed.revision());
    assert_eq!(undo.receipt.changes.invalidated_slides, vec![slide_id()]);
    let anchor = &undo.receipt.changes.anchor_maps[0];
    assert_eq!((anchor.start, anchor.deleted, anchor.inserted), (1, 3, 1));
    check_history_replay(&undo_request, &undo.receipt).unwrap();
    let mut reused = undo_request.clone();
    reused.direction = HistoryDirection::Redo;
    assert!(matches!(
        check_history_replay(&reused, &undo.receipt),
        Err(EditError::RequestIdReused)
    ));
    let redo = prepare_history(
        &undo.snapshot,
        &history(
            &undo.snapshot,
            &original,
            &transaction,
            HistoryDirection::Redo,
        ),
        Default::default(),
        &|| false,
    )
    .unwrap();
    assert_eq!(redo.snapshot.document(), renamed.document());
    assert_ne!(redo.snapshot.revision(), renamed.revision());
}

#[test]
fn cascading_deletion_restores_graph_and_order_without_replacing_current_document() {
    let mut doc = document();
    let connection = connector(&mut doc);
    let original = Snapshot::new(doc, Default::default()).unwrap();
    let transaction = edit(
        &original,
        "delete:1",
        vec![Operation::DeleteObject {
            object: id(),
            policy: DeletePolicy::Cascade,
        }],
    );
    let applied = apply(&original, &transaction);
    assert!(!applied.document().objects.contains_key(&connection));
    let undo = prepare_history(
        &applied,
        &history(&applied, &original, &transaction, HistoryDirection::Undo),
        Default::default(),
        &|| false,
    )
    .unwrap();
    assert_eq!(undo.snapshot.document(), original.document());
    assert_eq!(undo.receipt.changes.created_objects.len(), 2);
}

#[test]
fn overlapping_changes_conflict_atomically() {
    let original = Snapshot::new(document(), Default::default()).unwrap();
    let transaction = edit(
        &original,
        "rename:1",
        vec![Operation::SetTitle {
            title: "one".into(),
        }],
    );
    let applied = apply(&original, &transaction);
    let newer = apply(
        &applied,
        &edit(
            &applied,
            "rename:2",
            vec![Operation::SetTitle {
                title: "two".into(),
            }],
        ),
    );
    let before = canonical_bytes(newer.document()).unwrap();
    assert!(matches!(
        prepare_history(
            &newer,
            &history(&newer, &original, &transaction, HistoryDirection::Undo),
            Default::default(),
            &|| false
        ),
        Err(EditError::ReferenceConflict(_))
    ));
    assert_eq!(canonical_bytes(newer.document()).unwrap(), before);
}

#[test]
fn repeated_undo_requires_reconciliation_and_cannot_ignore_revision() {
    let original = Snapshot::new(document(), Default::default()).unwrap();
    let transaction = edit(
        &original,
        "rename:1",
        vec![Operation::SetTitle {
            title: "one".into(),
        }],
    );
    let applied = apply(&original, &transaction);
    let request = history(&applied, &original, &transaction, HistoryDirection::Undo);
    let undone = prepare_history(&applied, &request, Default::default(), &|| false).unwrap();
    assert!(matches!(
        prepare_history(&undone.snapshot, &request, Default::default(), &|| false),
        Err(EditError::RevisionConflict { .. })
    ));
    let mut fresh = request.clone();
    fresh.base_revision = undone.snapshot.revision().clone();
    assert!(matches!(
        prepare_history(&undone.snapshot, &fresh, Default::default(), &|| false),
        Err(EditError::ReferenceConflict(_))
    ));
}

#[test]
fn original_is_recomputed_not_trusted_and_cancellation_never_publishes() {
    let original = Snapshot::new(document(), Default::default()).unwrap();
    let transaction = edit(
        &original,
        "rename:1",
        vec![Operation::SetTitle {
            title: "one".into(),
        }],
    );
    let applied = apply(&original, &transaction);
    let mut request = history(&applied, &original, &transaction, HistoryDirection::Undo);
    request.original_snapshot.document.title = "tampered".into();
    assert!(prepare_history(&applied, &request, Default::default(), &|| false).is_err());
    let request = history(&applied, &original, &transaction, HistoryDirection::Undo);
    assert!(matches!(
        prepare_history(&applied, &request, Default::default(), &|| true),
        Err(EditError::Cancelled)
    ));
    assert_eq!(applied.document().title, "one");
}

#[test]
fn undo_cannot_delete_an_object_now_referenced_by_an_independent_connector() {
    let original = Snapshot::new(document(), Default::default()).unwrap();
    let mut new_object = original.document().objects[&id()].clone();
    new_object.id = ObjectId::new("new:shape").unwrap();
    new_object.content = ObjectContent::Shape {
        geometry: Geometry::Rectangle,
        text: None,
    };
    let transaction = edit(
        &original,
        "insert:1",
        vec![Operation::InsertObject {
            object: new_object.clone(),
            index: 1,
        }],
    );
    let applied = apply(&original, &transaction);
    let mut later = applied.document().clone();
    let connection = connector(&mut later);
    if let ObjectContent::Connector { start, .. } =
        &mut later.objects.get_mut(&connection).unwrap().content
    {
        *start = ConnectorEndpoint::Attached {
            object: new_object.id,
            site: 0,
        };
    }
    let later = Snapshot::new(later, Default::default()).unwrap();
    assert!(
        prepare_history(
            &later,
            &history(&later, &original, &transaction, HistoryDirection::Undo),
            Default::default(),
            &|| false
        )
        .is_err()
    );
    assert_eq!(later.document().objects.len(), 3);
}
