use super::*;
use std::collections::{BTreeMap, BTreeSet};

pub(super) fn apply(
    table: &mut Table,
    operation: &TableOperation,
    merges: &[TableRectangle],
    max_cells: usize,
    check: &dyn Fn() -> bool,
) -> Result<(), EditError> {
    match operation {
        TableOperation::InsertRow { index, row } => {
            let at = *index as usize;
            capacity(
                table.rows.len().checked_add(1),
                Some(table.columns.len()),
                max_cells,
            )?;
            if at > table.rows.len() || row.cells.len() != table.columns.len() {
                return Err(EditError::input(
                    "inserted row index or cell count is invalid",
                ));
            }
            unmerged(&row.cells, check)?;
            table.rows.insert(at, row.clone());
            for rect in merges {
                if rect.row < at && at < rect.row + rect.rows {
                    set_rectangle(
                        table,
                        rect.row,
                        rect.column,
                        rect.rows + 1,
                        rect.columns,
                        check,
                    )?;
                }
            }
        }
        TableOperation::InsertColumn {
            index,
            column,
            cells,
        } => {
            let at = *index as usize;
            capacity(
                Some(table.rows.len()),
                table.columns.len().checked_add(1),
                max_cells,
            )?;
            if at > table.columns.len() || cells.len() != table.rows.len() {
                return Err(EditError::input(
                    "inserted column index or cell count is invalid",
                ));
            }
            unmerged(cells, check)?;
            table.columns.insert(at, column.clone());
            for (row, cell) in table.rows.iter_mut().zip(cells) {
                cancel(check)?;
                row.cells.insert(at, cell.clone());
            }
            for rect in merges {
                if rect.column < at && at < rect.column + rect.columns {
                    set_rectangle(
                        table,
                        rect.row,
                        rect.column,
                        rect.rows,
                        rect.columns + 1,
                        check,
                    )?;
                }
            }
        }
        TableOperation::DeleteRow { row } => {
            if table.rows.len() == 1 {
                return Err(EditError::input("cannot remove the last table row"));
            }
            let at = table
                .rows
                .iter()
                .position(|r| &r.id == row)
                .ok_or_else(|| EditError::input("table row does not exist"))?;
            table.rows.remove(at);
            for rect in merges {
                if rect.row <= at && at < rect.row + rect.rows && rect.rows > 1 {
                    set_rectangle(
                        table,
                        rect.row,
                        rect.column,
                        rect.rows - 1,
                        rect.columns,
                        check,
                    )?;
                }
            }
        }
        TableOperation::DeleteColumn { column } => {
            if table.columns.len() == 1 {
                return Err(EditError::input("cannot remove the last table column"));
            }
            let at = table
                .columns
                .iter()
                .position(|c| &c.id == column)
                .ok_or_else(|| EditError::input("table column does not exist"))?;
            table.columns.remove(at);
            for row in &mut table.rows {
                cancel(check)?;
                row.cells.remove(at);
            }
            for rect in merges {
                if rect.column <= at && at < rect.column + rect.columns && rect.columns > 1 {
                    set_rectangle(
                        table,
                        rect.row,
                        rect.column,
                        rect.rows,
                        rect.columns - 1,
                        check,
                    )?;
                }
            }
        }
        TableOperation::ReorderRows { order } => {
            let indices = permutation(table.rows.iter().map(|r| &r.id), order, check)?;
            // Reorder by moves. Cell text is not cloned, including covered text.
            let mut old: Vec<_> = std::mem::take(&mut table.rows)
                .into_iter()
                .map(Some)
                .collect();
            for index in indices {
                cancel(check)?;
                table.rows.push(old[index].take().expect("permutation"));
            }
        }
        TableOperation::ReorderColumns { order } => {
            let indices = permutation(table.columns.iter().map(|c| &c.id), order, check)?;
            let mut old: Vec<_> = std::mem::take(&mut table.columns)
                .into_iter()
                .map(Some)
                .collect();
            for &index in &indices {
                cancel(check)?;
                table.columns.push(old[index].take().expect("permutation"));
            }
            for row in &mut table.rows {
                let mut cells: Vec<_> = std::mem::take(&mut row.cells)
                    .into_iter()
                    .map(Some)
                    .collect();
                for &index in &indices {
                    cancel(check)?;
                    row.cells.push(cells[index].take().expect("permutation"));
                }
            }
        }
        _ => unreachable!("non-structural table operation"),
    }
    Ok(())
}

fn capacity(rows: Option<usize>, columns: Option<usize>, limit: usize) -> Result<(), EditError> {
    rows.zip(columns)
        .and_then(|(r, c)| r.checked_mul(c))
        .filter(|&n| n <= limit)
        .map(|_| ())
        .ok_or_else(|| EditError::input("table grid exceeds cell budget"))
}
fn unmerged(cells: &[TableCell], check: &dyn Fn() -> bool) -> Result<(), EditError> {
    for cell in cells {
        cancel(check)?;
        if cell.merge != TableCellMerge::default() {
            return Err(EditError::input(
                "new row or column requires unmerged physical cells",
            ));
        }
    }
    Ok(())
}
fn permutation<'a, T: Ord + 'a>(
    existing: impl Iterator<Item = &'a T>,
    order: &[T],
    check: &dyn Fn() -> bool,
) -> Result<Vec<usize>, EditError> {
    let mut positions = BTreeMap::new();
    for (i, id) in existing.enumerate() {
        cancel(check)?;
        positions.insert(id, i);
    }
    if positions.len() != order.len() {
        return Err(EditError::input(
            "table reorder requires a complete permutation",
        ));
    }
    let mut seen = BTreeSet::new();
    let mut indices = Vec::with_capacity(order.len());
    for id in order {
        cancel(check)?;
        let index = positions
            .get(id)
            .filter(|_| seen.insert(id))
            .ok_or_else(|| EditError::input("table reorder repeats or omits an identity"))?;
        indices.push(*index);
    }
    Ok(indices)
}
