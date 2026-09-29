#![allow(dead_code)]
mod support;
use mo_common::*;
use mo_presentation_edit::*;
use mo_presentation_model::*;
use std::cell::Cell;
use support::{id, slide_id};

fn cell_id(r: usize, c: usize) -> CellId {
    CellId::new(format!("cell:{r}:{c}")).unwrap()
}
fn fresh_cell(name: &str) -> TableCell {
    TableCell {
        id: CellId::new(name).unwrap(),
        merge: Default::default(),
        text: None,
        style: Default::default(),
    }
}
fn snapshot() -> Snapshot {
    let mut d = support::document();
    let ObjectContent::Shape {
        text: Some(body), ..
    } = &d.objects[&id()].content
    else {
        panic!()
    };
    let body = body.clone();
    let rows = (0..3)
        .map(|r| TableRow {
            id: RowId::new(format!("row:{r}")).unwrap(),
            height: Emu::new(100),
            cells: (0..3)
                .map(|c| {
                    let mut text = body.clone();
                    text.paragraphs[0].id = ParagraphId::new(format!("paragraph:{r}:{c}")).unwrap();
                    text.paragraphs[0].runs[0].id = RunId::new(format!("run:{r}:{c}")).unwrap();
                    TableCell {
                        id: cell_id(r, c),
                        merge: Default::default(),
                        text: Some(text),
                        style: Default::default(),
                    }
                })
                .collect(),
        })
        .collect();
    d.objects.get_mut(&id()).unwrap().content = ObjectContent::Table {
        table: Table {
            columns: (0..3)
                .map(|c| TableColumn {
                    id: ColumnId::new(format!("column:{c}")).unwrap(),
                    width: Emu::new(200),
                })
                .collect(),
            rows,
        },
    };
    d.objects
        .get_mut(&id())
        .unwrap()
        .transform
        .as_mut()
        .unwrap()
        .size = Size {
        width: Emu::new(600),
        height: Emu::new(300),
    };
    Snapshot::new(d, Default::default()).unwrap()
}
fn table(s: &Snapshot) -> &Table {
    let ObjectContent::Table { table } = &s.document().objects[&id()].content else {
        panic!()
    };
    table
}
fn transaction(s: &Snapshot, operations: Vec<Operation>) -> Transaction {
    Transaction {
        document_id: s.document().id.clone(),
        request_id: RequestId::new("request:table").unwrap(),
        base_revision: s.revision().clone(),
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
fn operation(operation: TableOperation) -> Operation {
    Operation::EditTable {
        object: id(),
        operation,
    }
}
fn edit(s: &Snapshot, op: TableOperation) -> PreparedTransaction {
    prepare(s, &transaction(s, vec![operation(op)]), Default::default()).unwrap()
}
fn merged(s: &Snapshot, r: usize, c: usize, h: usize, w: usize) -> Snapshot {
    edit(
        s,
        TableOperation::Merge {
            origin: cell_id(r, c),
            rows: h as u32,
            columns: w as u32,
        },
    )
    .snapshot
}
#[test]
fn merge_and_split_preserve_all_physical_content_and_identity() {
    let original = snapshot();
    let before = table(&original).clone();
    let merged = merged(&original, 0, 0, 2, 2);
    for (old, new) in before
        .rows
        .iter()
        .flat_map(|r| &r.cells)
        .zip(table(&merged).rows.iter().flat_map(|r| &r.cells))
    {
        assert_eq!(
            (&old.id, &old.text, &old.style),
            (&new.id, &new.text, &new.style)
        );
    }
    let split = edit(
        &merged,
        TableOperation::Split {
            cell: cell_id(1, 1),
        },
    );
    assert_eq!(*table(&split.snapshot), before);
    assert_eq!(split.receipt.changes.updated_objects, vec![id()]);
    assert_eq!(split.receipt.changes.invalidated_slides, vec![slide_id()]);
    assert!(!split.receipt.changes.invalidate_all_layout);
    // Cutting only part of another merge is rejected, including via a covered origin.
    let cut = transaction(
        &merged,
        vec![operation(TableOperation::Merge {
            origin: cell_id(1, 1),
            rows: 2,
            columns: 2,
        })],
    );
    assert!(prepare(&merged, &cut, Default::default()).is_err());
    assert_eq!(*table(&original), before);
}
#[test]
fn all_insert_and_delete_boundaries_preserve_surviving_cell_payloads() {
    let original = snapshot();
    for r in 0..3 {
        for c in 0..3 {
            for h in 1..=3 - r {
                for w in 1..=3 - c {
                    let s = merged(&original, r, c, h, w);
                    for axis in 0..2 {
                        for insertion in [false, true] {
                            for at in 0..if insertion { 4 } else { 3 } {
                                let op = match (axis, insertion) {
                                    (0, true) => TableOperation::InsertRow {
                                        index: at as u32,
                                        row: TableRow {
                                            id: RowId::new("new-row").unwrap(),
                                            height: Emu::new(50),
                                            cells: (0..3)
                                                .map(|c| fresh_cell(&format!("new:{c}")))
                                                .collect(),
                                        },
                                    },
                                    (1, true) => TableOperation::InsertColumn {
                                        index: at as u32,
                                        column: TableColumn {
                                            id: ColumnId::new("new-column").unwrap(),
                                            width: Emu::new(50),
                                        },
                                        cells: (0..3)
                                            .map(|r| fresh_cell(&format!("new:{r}")))
                                            .collect(),
                                    },
                                    (0, false) => TableOperation::DeleteRow {
                                        row: table(&s).rows[at].id.clone(),
                                    },
                                    (1, false) => TableOperation::DeleteColumn {
                                        column: table(&s).columns[at].id.clone(),
                                    },
                                    _ => unreachable!(),
                                };
                                let next = edit(&s, op).snapshot;
                                let t = table(&next);
                                let grid = TableGrid::compile(t, 12, &|| false).unwrap();
                                let old = table(&s);
                                for cell in t.rows.iter().flat_map(|r| &r.cells) {
                                    if let Some(previous) = old
                                        .rows
                                        .iter()
                                        .flat_map(|r| &r.cells)
                                        .find(|p| p.id == cell.id)
                                    {
                                        assert_eq!(
                                            (&cell.text, &cell.style),
                                            (&previous.text, &previous.style)
                                        );
                                    }
                                }
                                // Independent ownership oracle: surviving members of the original
                                // merged rectangle still share one owner. New interior cells join it.
                                let members: Vec<_> = (r..r + h)
                                    .flat_map(|rr| (c..c + w).map(move |cc| cell_id(rr, cc)))
                                    .filter(|id| grid.position(id).is_some())
                                    .collect();
                                if let Some(first) = members.first() {
                                    let owner = &grid.origin(first).unwrap().id;
                                    for member in &members {
                                        assert_eq!(&grid.origin(member).unwrap().id, owner);
                                    }
                                    let count = t
                                        .rows
                                        .iter()
                                        .flat_map(|r| &r.cells)
                                        .filter(|cell| &grid.origin(&cell.id).unwrap().id == owner)
                                        .count();
                                    let (start, length, cross) =
                                        if axis == 0 { (r, h, w) } else { (c, w, h) };
                                    let extra = if insertion && start < at && at < start + length {
                                        cross
                                    } else {
                                        0
                                    };
                                    assert_eq!(
                                        count,
                                        members.len() + extra,
                                        "{r},{c},{h},{w} axis {axis} insert {insertion} at {at}"
                                    );
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
#[test]
fn reorder_moves_whole_merged_blocks_and_rejects_broken_permutations_atomically() {
    let s = merged(&snapshot(), 0, 0, 2, 2);
    let before = canonical_bytes(s.document()).unwrap();
    let row_ids: Vec<_> = table(&s).rows.iter().map(|r| r.id.clone()).collect();
    let col_ids: Vec<_> = table(&s).columns.iter().map(|c| c.id.clone()).collect();
    let moved = edit(
        &s,
        TableOperation::ReorderRows {
            order: vec![row_ids[2].clone(), row_ids[0].clone(), row_ids[1].clone()],
        },
    )
    .snapshot;
    let moved = edit(
        &moved,
        TableOperation::ReorderColumns {
            order: vec![col_ids[2].clone(), col_ids[0].clone(), col_ids[1].clone()],
        },
    )
    .snapshot;
    let grid = TableGrid::compile(table(&moved), 9, &|| false).unwrap();
    let rect = grid.rectangle(&cell_id(1, 1)).unwrap();
    assert_eq!(
        (rect.row, rect.column, rect.rows, rect.columns),
        (1, 1, 2, 2)
    );
    for order in [
        vec![row_ids[0].clone(), row_ids[2].clone(), row_ids[1].clone()],
        vec![row_ids[1].clone(), row_ids[0].clone(), row_ids[2].clone()],
        vec![row_ids[0].clone(); 3],
        vec![row_ids[0].clone()],
    ] {
        let txn = transaction(
            &s,
            vec![
                Operation::SetTitle {
                    title: "must roll back".into(),
                },
                operation(TableOperation::ReorderRows { order }),
            ],
        );
        assert!(prepare(&s, &txn, Default::default()).is_err());
        assert_eq!(canonical_bytes(s.document()).unwrap(), before);
    }
}
#[test]
fn unicode_splice_in_a_covered_cell_preserves_anchors_and_reappears_after_split() {
    let s = merged(&snapshot(), 0, 0, 2, 2);
    let body = table(&s).rows[1].cells[1].text.as_ref().unwrap();
    let paragraph = body.paragraphs[0].id.clone();
    let run = body.paragraphs[0].runs[0].id.clone();
    let txn = transaction(
        &s,
        vec![Operation::SpliceText {
            object: id(),
            paragraph: paragraph.clone(),
            run,
            start: 1,
            delete: 1,
            insert: "火箭🚀".into(),
        }],
    );
    let edited = prepare(&s, &txn, Default::default()).unwrap();
    let map = &edited.receipt.changes.anchor_maps[0];
    assert_eq!((map.start, map.deleted, map.inserted), (1, 1, 3));
    assert_eq!(
        map.map(&TextAnchor {
            paragraph,
            scalar_offset: 5,
            affinity: Affinity::After
        })
        .unwrap()
        .scalar_offset,
        7
    );
    let split = edit(
        &edited.snapshot,
        TableOperation::Split {
            cell: cell_id(1, 1),
        },
    )
    .snapshot;
    assert_eq!(
        table(&split).rows[1].cells[1]
            .text
            .as_ref()
            .unwrap()
            .paragraphs[0]
            .runs[0]
            .content,
        InlineContent::Text {
            text: "A火箭🚀e\u{301}中".into()
        }
    );
}
#[test]
fn every_cancellation_checkpoint_keeps_the_input_revision_unchanged() {
    let s = merged(&snapshot(), 0, 0, 3, 3);
    let before = canonical_bytes(s.document()).unwrap();
    let txn = transaction(
        &s,
        vec![
            operation(TableOperation::Split {
                cell: cell_id(1, 1),
            }),
            operation(TableOperation::ReorderRows {
                order: table(&s).rows.iter().rev().map(|r| r.id.clone()).collect(),
            }),
        ],
    );
    let count = Cell::new(0);
    prepare_cancellable(&s, &txn, Default::default(), &|| {
        count.set(count.get() + 1);
        false
    })
    .unwrap();
    for stop in 0..count.get() {
        let calls = Cell::new(0);
        let result = prepare_cancellable(&s, &txn, Default::default(), &|| {
            let n = calls.get();
            calls.set(n + 1);
            n >= stop
        });
        assert!(result.is_err(), "checkpoint {stop}");
        assert_eq!(canonical_bytes(s.document()).unwrap(), before);
    }
}
#[test]
fn grid_resize_updates_frame_and_invalid_styles_or_sizes_roll_back() {
    let s = snapshot();
    let resized = edit(
        &s,
        TableOperation::SetColumnWidth {
            column: ColumnId::new("column:1").unwrap(),
            width: Emu::new(350),
        },
    )
    .snapshot;
    let resized = edit(
        &resized,
        TableOperation::SetRowHeight {
            row: RowId::new("row:2").unwrap(),
            height: Emu::new(250),
        },
    )
    .snapshot;
    assert_eq!(
        resized.document().objects[&id()].transform.unwrap().size,
        Size {
            width: Emu::new(750),
            height: Emu::new(450)
        }
    );
    let mut transform = resized.document().objects[&id()].transform.unwrap();
    transform.size.width = Emu::new(749);
    assert!(
        prepare(
            &resized,
            &transaction(
                &resized,
                vec![Operation::SetTransform {
                    object: id(),
                    transform
                }]
            ),
            Default::default()
        )
        .is_err()
    );
    let style: TableCellStyle =
        serde_json::from_str(r#"{"borders":{"left":{"kind":"value","value":{"kind":"none"}}}}"#)
            .unwrap();
    assert_eq!(style.borders.left, Inherited::Value(Stroke::None {}));
    let mut negative = style;
    negative.borders.right = Inherited::Value(Stroke::Solid {
        color: Color::Theme {
            slot: ThemeColor::Accent1,
        },
        width: Emu::new(-1),
        cap: None,
        join: None,
    });
    for op in [
        TableOperation::SetRowHeight {
            row: RowId::new("row:0").unwrap(),
            height: Emu::ZERO,
        },
        TableOperation::SetCellStyle {
            cell: cell_id(0, 0),
            style: negative,
        },
    ] {
        assert!(
            prepare(
                &resized,
                &transaction(&resized, vec![operation(op)]),
                Default::default()
            )
            .is_err()
        );
    }
    assert_eq!(table(&s).columns[1].width, Emu::new(200));
}

#[test]
fn cell_budget_is_document_wide_and_covered_text_keeps_global_ids_and_font_dependencies() {
    let s = merged(&snapshot(), 0, 0, 2, 2);
    let mut d = s.document().clone();
    let second = ObjectId::new("table:2").unwrap();
    let mut object = d.objects[&id()].clone();
    object.id = second.clone();
    let ObjectContent::Table { table: t } = &mut object.content else {
        panic!()
    };
    for cell in t.rows.iter_mut().flat_map(|r| &mut r.cells) {
        cell.text = None;
    }
    d.objects.insert(second.clone(), object);
    d.slides.get_mut(&slide_id()).unwrap().objects.push(second);
    assert!(
        validate(
            &d,
            ValidationLimits {
                max_table_cells: 18,
                ..Default::default()
            }
        )
        .is_valid()
    );
    assert!(
        validate(
            &d,
            ValidationLimits {
                max_table_cells: 17,
                ..Default::default()
            }
        )
        .issues
        .iter()
        .any(|i| i.code == ValidationCode::LimitExceeded)
    );
    let ObjectContent::Table { table: t } = &mut d.objects.get_mut(&id()).unwrap().content else {
        panic!()
    };
    t.rows[1].cells[1].text = t.rows[0].cells[0].text.clone();
    assert!(
        validate(&d, Default::default())
            .issues
            .iter()
            .any(|i| i.code == ValidationCode::DuplicateIdentity)
    );
    let mut d = s.document().clone();
    let font = FontId::new("font:table").unwrap();
    let resource = ResourceId::new("resource:table-font").unwrap();
    d.fonts.insert(
        font.clone(),
        FontFace {
            id: font.clone(),
            resource: resource.clone(),
            face_index: 0,
            family: "OwnedFixture".into(),
            weight: 400,
            italic: false,
        },
    );
    d.resources.insert(
        resource.clone(),
        Resource {
            id: resource.clone(),
            kind: ResourceKind::Font,
            sha256: Digest::from_sha256([1; 32]),
            media_type: "font/ttf".into(),
        },
    );
    let ObjectContent::Table { table: t } = &mut d.objects.get_mut(&id()).unwrap().content else {
        panic!()
    };
    t.rows[1].cells[1].text.as_mut().unwrap().style.font = Inherited::Value(font);
    assert!(validate(&d, Default::default()).is_valid());
    let key = PageDependencies::new(&d, &slide_id())
        .unwrap()
        .digest()
        .unwrap();
    d.resources.get_mut(&resource).unwrap().sha256 = Digest::from_sha256([2; 32]);
    assert_ne!(
        key,
        PageDependencies::new(&d, &slide_id())
            .unwrap()
            .digest()
            .unwrap()
    );
    d.fonts.clear();
    assert!(PageDependencies::new(&d, &slide_id()).is_none());
}
