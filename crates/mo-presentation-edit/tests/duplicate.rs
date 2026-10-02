mod support;
use mo_common::*;
use mo_presentation_edit::*;
use mo_presentation_model::*;
use mo_timeline::*;
use support::*;

fn copy_id() -> SlideId {
    SlideId::new("copy:slide").unwrap()
}
fn transaction(base: &Snapshot) -> Transaction {
    Transaction {
        document_id: base.document().id.clone(),
        base_revision: base.revision().clone(),
        request_id: RequestId::new("duplicate:1").unwrap(),
        operations: vec![OperationEntry {
            operation_id: OperationId::new("duplicate-op:1").unwrap(),
            operation: Operation::DuplicateSlide {
                source: slide_id(),
                slide: copy_id(),
                index: 1,
            },
        }],
    }
}
fn fixture() -> Snapshot {
    let mut doc = document();
    let connector_id = connector(&mut doc);
    let group_id = ObjectId::new("group").unwrap();
    let mut group = doc.objects[&id()].clone();
    group.id = group_id.clone();
    group.content = ObjectContent::Group {
        children: vec![id(), connector_id.clone()],
        viewport: doc.page_size,
    };
    for child in [&id(), &connector_id] {
        doc.objects.get_mut(child).unwrap().parent = ContainerId::Group(group_id.clone());
    }
    doc.objects.insert(group_id.clone(), group);
    doc.slides.get_mut(&slide_id()).unwrap().objects = vec![group_id];
    doc.timelines.insert(
        slide_id(),
        Timeline {
            format: TimelineVersion::V01,
            tree: None,
            nodes: vec![TimingNode {
                id: TimingNodeId::new("animation").unwrap(),
                restart: RestartMode::Never,
                start: TimeCondition::Click {
                    target: Some(connector_id),
                    delay: RationalTime::new(0, 1).unwrap(),
                }
                .into(),
                end_conditions: vec![],
                duration: RationalTime::new(1, 1).unwrap(),
                repeat_milli: RepeatCount::Finite(1000),
                repeat_duration: None,
                time_transform: None,
                fill: FillMode::Hold,
                effect: Effect::Rotation {
                    target: id(),
                    from: 0,
                    to: 60000,
                    composition: Default::default(),
                },
            }],
        },
    );
    Snapshot::new(doc, Default::default()).unwrap()
}

#[test]
fn page_copy_remaps_groups_connectors_text_and_timeline_but_shares_resources() {
    let base = fixture();
    let tx = transaction(&base);
    let result = prepare(&base, &tx, Default::default()).unwrap();
    let doc = result.snapshot.document();
    let copy = &doc.slides[&copy_id()];
    let group = &doc.objects[&copy.objects[0]];
    assert_eq!(group.parent, ContainerId::Slide(copy_id()));
    let ObjectContent::Group { children, .. } = &group.content else {
        panic!()
    };
    let (shape, connector) = (&doc.objects[&children[0]], &doc.objects[&children[1]]);
    assert_eq!(shape.parent, ContainerId::Group(group.id.clone()));
    let ObjectContent::Connector {
        start: ConnectorEndpoint::Attached { object, .. },
        ..
    } = &connector.content
    else {
        panic!()
    };
    assert_eq!(object, &shape.id);
    let ObjectContent::Shape {
        text: Some(text), ..
    } = &shape.content
    else {
        panic!()
    };
    assert_ne!(text.paragraphs[0].id, paragraph_id());
    assert_ne!(text.paragraphs[0].runs[0].id, run_id());
    assert_eq!(
        text.paragraphs[0].runs[0].content,
        InlineContent::Text {
            text: "A😀e\u{301}中".into()
        }
    );
    let timeline = &doc.timelines[&copy_id()];
    assert_eq!(timeline.nodes[0].target(), &shape.id);
    assert_eq!(
        timeline.nodes[0].start.conditions()[0].target(),
        Some(&connector.id)
    );
    assert_eq!(doc.resources, base.document().resources);
    assert_eq!(doc.fonts, base.document().fonts);
    assert_eq!(
        doc.timelines[&slide_id()],
        base.document().timelines[&slide_id()]
    );
    assert_eq!(result.receipt.changes.created_objects.len(), 3);
    assert_eq!(result.receipt.changes.changed_timelines, vec![copy_id()]);
    assert_eq!(
        result.snapshot.clone().into_record(),
        prepare(&base, &tx, Default::default())
            .unwrap()
            .snapshot
            .into_record()
    );
    let undo = prepare_history(
        &result.snapshot,
        &HistoryTransaction {
            document_id: doc.id.clone(),
            base_revision: result.snapshot.revision().clone(),
            request_id: RequestId::new("undo:copy").unwrap(),
            direction: HistoryDirection::Undo,
            original_snapshot: base.clone().into_record(),
            original_transaction: tx,
        },
        Default::default(),
        &|| false,
    )
    .unwrap();
    assert_eq!(undo.snapshot.document(), base.document());
}

#[test]
fn copied_tables_retain_scoped_grid_identity_and_receive_new_global_text_identity() {
    let mut doc = document();
    let ObjectContent::Shape { text, .. } = doc.objects[&id()].content.clone() else {
        panic!()
    };
    let size = doc.page_size;
    doc.objects.get_mut(&id()).unwrap().content = ObjectContent::Table {
        table: Table {
            columns: vec![TableColumn {
                id: ColumnId::new("column").unwrap(),
                width: size.width,
            }],
            rows: vec![TableRow {
                id: RowId::new("row").unwrap(),
                height: size.height,
                cells: vec![TableCell {
                    id: CellId::new("cell").unwrap(),
                    merge: Default::default(),
                    text,
                    style: Default::default(),
                }],
            }],
        },
    };
    let base = Snapshot::new(doc, Default::default()).unwrap();
    let result = prepare(&base, &transaction(&base), Default::default()).unwrap();
    let copied = &result.snapshot.document().slides[&copy_id()].objects[0];
    let ObjectContent::Table { table } = &result.snapshot.document().objects[copied].content else {
        panic!()
    };
    assert_eq!(table.rows[0].cells[0].id.as_str(), "cell");
    assert_ne!(
        table.rows[0].cells[0].text.as_ref().unwrap().paragraphs[0].id,
        paragraph_id()
    );
}

#[test]
fn copy_conflicts_limits_and_every_cancel_checkpoint_are_atomic() {
    let base = fixture();
    let tx = transaction(&base);
    let initial = base.clone().into_record();
    let count = std::cell::Cell::new(0);
    prepare_cancellable(&base, &tx, Default::default(), &|| {
        count.set(count.get() + 1);
        false
    })
    .unwrap();
    for stop in 0..count.get() {
        let calls = std::cell::Cell::new(0);
        assert!(
            prepare_cancellable(&base, &tx, Default::default(), &|| {
                let cancelled = calls.get() == stop;
                calls.set(calls.get() + 1);
                cancelled
            })
            .is_err()
        );
        assert_eq!(base.clone().into_record(), initial);
    }
    assert!(
        prepare(
            &base,
            &tx,
            ValidationLimits {
                max_objects: 5,
                ..Default::default()
            }
        )
        .is_err()
    );
    let mut invalid = tx.clone();
    invalid.operations[0].operation = Operation::DuplicateSlide {
        source: slide_id(),
        slide: slide_id(),
        index: 1,
    };
    assert!(prepare(&base, &invalid, Default::default()).is_err());
    assert_eq!(base.clone().into_record(), initial);
}
