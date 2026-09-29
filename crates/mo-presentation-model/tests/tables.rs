use mo_common::*;
use mo_presentation_model::*;
use std::cell::Cell;

fn table() -> Table {
    Table {
        columns: (0..4)
            .map(|c| TableColumn {
                id: ColumnId::new(format!("column:{c}")).unwrap(),
                width: Emu::new((c + 1) * 100),
            })
            .collect(),
        rows: (0..4)
            .map(|r| TableRow {
                id: RowId::new(format!("row:{r}")).unwrap(),
                height: Emu::new((r + 1) * 10),
                cells: (0..4)
                    .map(|c| TableCell {
                        id: cell_id(r, c),
                        merge: Default::default(),
                        text: None,
                        style: Default::default(),
                    })
                    .collect(),
            })
            .collect(),
    }
}
fn cell_id(r: i64, c: i64) -> CellId {
    CellId::new(format!("cell:{r}:{c}")).unwrap()
}
fn merged() -> Table {
    let mut table = table();
    table.rows[1].cells[1].merge = TableCellMerge::Span {
        rows: 2,
        columns: 2,
    };
    for (r, c) in [(1, 2), (2, 1), (2, 2)] {
        table.rows[r].cells[c].merge = TableCellMerge::Covered {
            origin: cell_id(1, 1),
        };
    }
    table
}
#[test]
fn exact_grid_geometry_maps_every_covered_cell_to_one_visible_rectangle() {
    let table = merged();
    let grid = TableGrid::compile(&table, 16, &|| false).unwrap();
    assert_eq!(grid.cell_count(), 16);
    assert_eq!(
        grid.size(),
        Size {
            width: Emu::new(1000),
            height: Emu::new(100)
        }
    );
    let rect = TableRectangle {
        row: 1,
        column: 1,
        rows: 2,
        columns: 2,
        origin: Point {
            x: Emu::new(100),
            y: Emu::new(10),
        },
        size: Size {
            width: Emu::new(500),
            height: Emu::new(50),
        },
    };
    for (r, c) in [(1, 1), (1, 2), (2, 1), (2, 2)] {
        assert_eq!(grid.rectangle(&cell_id(r, c)), Some(rect));
        assert_eq!(grid.origin(&cell_id(r, c)).unwrap().id, cell_id(1, 1));
        assert_eq!(
            grid.position(&cell_id(r, c)),
            Some((r as usize, c as usize))
        );
    }
    assert!(grid.rectangle(&CellId::new("missing").unwrap()).is_none());
    assert_eq!(
        serde_json::from_slice::<Table>(&serde_json::to_vec(&table).unwrap()).unwrap(),
        table
    );
}
#[test]
fn invalid_rectangles_and_physical_identity_are_rejected() {
    let mutations: [fn(&mut Table); 12] = [
        |t| t.rows[2].cells[2].merge = TableCellMerge::default(),
        |t| {
            t.rows[2].cells[2].merge = TableCellMerge::Covered {
                origin: cell_id(0, 0),
            }
        },
        |t| {
            t.rows[0].cells[0].merge = TableCellMerge::Covered {
                origin: cell_id(1, 1),
            }
        },
        |t| {
            t.rows[0].cells[0].merge = TableCellMerge::Span {
                rows: 0,
                columns: 1,
            }
        },
        |t| {
            t.rows[3].cells[3].merge = TableCellMerge::Span {
                rows: 2,
                columns: 1,
            }
        },
        |t| {
            t.rows[0].cells[0].merge = TableCellMerge::Span {
                rows: u32::MAX,
                columns: u32::MAX,
            }
        },
        |t| {
            t.rows[0].cells.pop();
        },
        |t| t.rows[0].cells[0].id = cell_id(1, 1),
        |t| t.rows[0].id = t.rows[1].id.clone(),
        |t| t.columns[0].id = t.columns[1].id.clone(),
        |t| t.rows[0].height = Emu::ZERO,
        |t| t.columns[0].width = Emu::new(-1),
    ];
    for (i, mutation) in mutations.into_iter().enumerate() {
        let mut table = merged();
        mutation(&mut table);
        assert!(
            matches!(
                TableGrid::compile(&table, 16, &|| false),
                Err(TableGridError::Invalid { .. })
            ),
            "case {i}"
        );
    }
}
#[test]
fn limits_coordinate_overflow_and_cancellation_are_checked_before_publication() {
    let mut table = merged();
    assert!(matches!(
        TableGrid::compile(&table, 15, &|| false),
        Err(TableGridError::Limit)
    ));
    for at in [0, 5, 20, 40] {
        let calls = Cell::new(0);
        let check = || {
            let n = calls.get();
            calls.set(n + 1);
            n >= at
        };
        assert!(matches!(
            TableGrid::compile(&table, 16, &check),
            Err(TableGridError::Cancelled)
        ));
    }
    table.columns[0].width = Emu::new(i64::MAX);
    assert!(matches!(
        TableGrid::compile(&table, 16, &|| false),
        Err(TableGridError::Invalid { .. })
    ));
    table.columns.clear();
    assert!(matches!(
        TableGrid::compile(&table, 16, &|| false),
        Err(TableGridError::Invalid { .. })
    ));
}
