use super::*;
use crate::source::table::{SourceTableColumn, SourceTableRow};
use std::cell::Cell;

fn at(row: u32, column: u32) -> SourceCellAddress {
    SourceCellAddress { row, column }
}
fn table(rows: u32, columns: u32, regions: &[NativeTableRegion], redundant: bool) -> SourceTable {
    let mut table = SourceTable {
        source_ordinal: 1,
        properties: None,
        grid_ordinal: 2,
        columns: (0..columns)
            .map(|c| SourceTableColumn {
                source_ordinal: c + 3,
                width: "1".to_owned().try_into().unwrap(),
            })
            .collect(),
        rows: (0..rows)
            .map(|r| SourceTableRow {
                source_ordinal: r + columns + 3,
                height: "1".to_owned().try_into().unwrap(),
                cells: vec![SourceTableCell::default(); columns as usize],
            })
            .collect(),
        retained_ordinals: vec![],
    };
    for region in regions {
        for r in region.origin.row..region.origin.row + region.rows {
            for c in region.origin.column..region.origin.column + region.columns {
                let cell = &mut table.rows[r as usize].cells[c as usize];
                let is_origin = at(r, c) == region.origin;
                cell.row_span = (is_origin || (redundant && r == region.origin.row))
                    .then_some(region.rows as i32);
                cell.grid_span = (is_origin || (redundant && c == region.origin.column))
                    .then_some(region.columns as i32);
                cell.horizontal_merge = (c > region.origin.column).then_some(true);
                cell.vertical_merge = (r > region.origin.row).then_some(true);
                cell.native_id = Some(format!("{r}:{c}"));
                cell.paragraph_start = r * columns + c;
            }
        }
    }
    table
}
fn compile(table: &SourceTable) -> Result<NativeTableGrid<'_>, NativeTableGridError> {
    NativeTableGrid::compile(table, Default::default(), &|| false)
}
fn region(row: u32, column: u32, rows: u32, columns: u32) -> NativeTableRegion {
    NativeTableRegion {
        origin: at(row, column),
        rows,
        columns,
    }
}

// Enumerate rectangular tilings independently of native hMerge/vMerge traversal.
// Every first free cell can own any still-free rectangle anchored there.
fn tilings(occupied: &mut [bool; 9], chosen: &mut Vec<NativeTableRegion>, count: &mut usize) {
    let Some(i) = occupied.iter().position(|v| !v) else {
        for redundant in [false, true] {
            let t = table(3, 3, chosen, redundant);
            let g = compile(&t).unwrap();
            assert_eq!(g.regions(), chosen);
            for region in chosen.iter() {
                for row in region.origin.row..region.origin.row + region.rows {
                    for column in region.origin.column..region.origin.column + region.columns {
                        assert_eq!(g.region(at(row, column)), Some(region));
                        assert!(std::ptr::eq(
                            g.cell(at(row, column)).unwrap(),
                            &t.rows[row as usize].cells[column as usize]
                        ));
                    }
                }
            }
        }
        *count += 1;
        return;
    };
    let (r, c) = (i / 3, i % 3);
    for h in 1..=3 - r {
        for w in 1..=3 - c {
            let cells = (r..r + h)
                .flat_map(|r| (c..c + w).map(move |c| 3 * r + c))
                .collect::<Vec<_>>();
            if cells.iter().any(|i| occupied[*i]) {
                continue;
            }
            for i in &cells {
                occupied[*i] = true;
            }
            chosen.push(region(r as u32, c as u32, h as u32, w as u32));
            tilings(occupied, chosen, count);
            chosen.pop();
            for i in cells {
                occupied[i] = false;
            }
        }
    }
}
#[test]
fn every_three_by_three_rectangular_tiling_resolves_without_losing_physical_cells() {
    let mut count = 0;
    tilings(&mut [false; 9], &mut vec![], &mut count);
    assert_eq!(count, 322);
}
fn invalid(t: &SourceTable) -> NativeTableGridIssue {
    match compile(t) {
        Err(NativeTableGridError::Invalid(i)) => i,
        _ => panic!("expected invalid topology"),
    }
}
#[test]
fn rejects_missing_links_holes_outside_spans_and_conflicting_neighbours() {
    let base = table(2, 2, &[region(0, 0, 2, 2)], true);
    let mut t = base.clone();
    t.rows[0].cells[0].horizontal_merge = Some(true);
    assert!(matches!(
        invalid(&t),
        NativeTableGridIssue::MissingNeighbour { .. }
    ));
    let mut t = base.clone();
    t.rows[0].cells[0].vertical_merge = Some(true);
    assert!(matches!(
        invalid(&t),
        NativeTableGridIssue::MissingNeighbour { .. }
    ));
    let mut t = base.clone();
    t.rows[1].cells[1].horizontal_merge = None;
    t.rows[1].cells[1].vertical_merge = None;
    assert!(matches!(
        invalid(&t),
        NativeTableGridIssue::IncompleteMerge { .. }
    ));
    let mut t = base.clone();
    t.rows[0].cells[0].grid_span = Some(1);
    assert!(matches!(
        invalid(&t),
        NativeTableGridIssue::OutsideMerge { .. }
    ));
    let mut t = table(
        2,
        2,
        &[region(0, 0, 2, 1), region(0, 1, 1, 1), region(1, 1, 1, 1)],
        false,
    );
    t.rows[1].cells[1].horizontal_merge = Some(true);
    t.rows[1].cells[1].vertical_merge = Some(true);
    assert!(matches!(
        invalid(&t),
        NativeTableGridIssue::ConflictingNeighbours { .. }
    ));
    let mut t = base.clone();
    t.rows[0].cells[1].row_span = Some(3);
    assert!(matches!(
        invalid(&t),
        NativeTableGridIssue::ConflictingSpan { .. }
    ));
}
#[test]
fn rejects_invalid_dimensions_duplicate_identity_and_ragged_rows() {
    let base = table(2, 2, &[region(0, 0, 2, 2)], false);
    for span in [0, -1, i32::MAX] {
        let mut t = base.clone();
        t.rows[0].cells[0].row_span = Some(span);
        assert!(matches!(
            invalid(&t),
            NativeTableGridIssue::InvalidSpan { .. }
        ));
    }
    let mut t = base.clone();
    t.rows[1].cells[1].native_id = t.rows[0].cells[0].native_id.clone();
    assert!(matches!(
        invalid(&t),
        NativeTableGridIssue::DuplicateCellId { .. }
    ));
    let mut t = base;
    t.rows[1].cells.pop();
    assert!(matches!(
        invalid(&t),
        NativeTableGridIssue::RowWidth {
            row: 1,
            actual: 1,
            expected: 2
        }
    ));
    assert_eq!(
        invalid(&table(0, 0, &[], false)),
        NativeTableGridIssue::EmptyGrid
    );
}
#[test]
fn exact_limits_cancellation_and_out_of_grid_queries() {
    let t = table(2, 2, &[region(0, 0, 2, 2)], false);
    let limits = NativeTableGridLimits {
        max_cells: 4,
        max_steps: 7,
        max_id_bytes: 12,
    };
    let g = NativeTableGrid::compile(&t, limits, &|| false).unwrap();
    assert!(g.cell(at(2, 0)).is_none());
    assert!(g.region(at(0, 2)).is_none());
    for limits in [
        NativeTableGridLimits {
            max_cells: 3,
            ..limits
        },
        NativeTableGridLimits {
            max_steps: 6,
            ..limits
        },
        NativeTableGridLimits {
            max_id_bytes: 11,
            ..limits
        },
    ] {
        assert!(matches!(
            NativeTableGrid::compile(&t, limits, &|| false),
            Err(NativeTableGridError::Limit)
        ));
    }
    for stop in 1..=8 {
        let calls = Cell::new(0);
        assert!(matches!(
            NativeTableGrid::compile(&t, limits, &|| {
                calls.set(calls.get() + 1);
                calls.get() == stop
            }),
            Err(NativeTableGridError::Cancelled)
        ));
    }
}
