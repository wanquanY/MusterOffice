//! Table mutations share the document transaction's private working copy.
//! Failure or cancellation cannot publish a half-updated merge rectangle.
mod structure;
use crate::EditError;
use mo_common::{CellId, ColumnId, Emu, RowId};
use mo_presentation_model::*;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "camelCase", deny_unknown_fields)]
pub enum TableOperation {
    Replace {
        table: Table,
    },
    SetCellText {
        cell: CellId,
        text: Option<TextBody>,
    },
    SetCellStyle {
        cell: CellId,
        style: TableCellStyle,
    },
    SetColumnWidth {
        column: ColumnId,
        width: Emu,
    },
    SetRowHeight {
        row: RowId,
        height: Emu,
    },
    /// Existing merged rectangles must be wholly inside this rectangle.
    /// Covered cells retain their identities, styles and text for later split.
    Merge {
        origin: CellId,
        rows: u32,
        columns: u32,
    },
    Split {
        cell: CellId,
    },
    /// Supplied cells must be unmerged. Crossing merges expand over the row.
    InsertRow {
        index: u32,
        row: TableRow,
    },
    /// Supply one unmerged physical cell per existing row.
    InsertColumn {
        index: u32,
        column: TableColumn,
        cells: Vec<TableCell>,
    },
    /// Removes only this row's physical cells. Surviving merge rectangles
    /// shrink; when the origin is removed, the next upper-left cell takes over
    /// with its own retained text/style. Deleted content is never transferred.
    DeleteRow {
        row: RowId,
    },
    DeleteColumn {
        column: ColumnId,
    },
    /// A complete permutation. A merged cell must remain a rectangle with the
    /// same upper-left origin; split explicitly before rearranging its members.
    ReorderRows {
        order: Vec<RowId>,
    },
    ReorderColumns {
        order: Vec<ColumnId>,
    },
}

fn error(error: TableGridError) -> EditError {
    match error {
        TableGridError::Cancelled => EditError::Cancelled,
        error => EditError::input(error.to_string()),
    }
}
fn cancel(check: &dyn Fn() -> bool) -> Result<(), EditError> {
    if check() {
        Err(EditError::Cancelled)
    } else {
        Ok(())
    }
}
fn locate(grid: &TableGrid<'_>, cell: &CellId) -> Result<(usize, usize), EditError> {
    grid.position(cell)
        .ok_or_else(|| EditError::input("table cell does not exist"))
}

pub(crate) fn apply(
    table: &mut Table,
    operation: &TableOperation,
    max_cells: usize,
    check: &dyn Fn() -> bool,
) -> Result<Size, EditError> {
    let grid = TableGrid::compile(table, max_cells, check).map_err(error)?;
    match operation {
        TableOperation::Replace { table: replacement } => {
            TableGrid::compile(replacement, max_cells, check).map_err(error)?;
            *table = replacement.clone();
        }
        TableOperation::SetCellText { cell, text } => {
            let (r, c) = locate(&grid, cell)?;
            table.rows[r].cells[c].text = text.clone();
        }
        TableOperation::SetCellStyle { cell, style } => {
            let (r, c) = locate(&grid, cell)?;
            table.rows[r].cells[c].style = style.clone();
        }
        TableOperation::SetRowHeight { row, height } => {
            let row = table
                .rows
                .iter_mut()
                .find(|r| &r.id == row)
                .ok_or_else(|| EditError::input("table row does not exist"))?;
            row.height = *height;
        }
        TableOperation::SetColumnWidth { column, width } => {
            let column = table
                .columns
                .iter_mut()
                .find(|c| &c.id == column)
                .ok_or_else(|| EditError::input("table column does not exist"))?;
            column.width = *width;
        }
        TableOperation::Merge {
            origin,
            rows,
            columns,
        } => {
            let (r, c) = locate(&grid, origin)?;
            let (height, width) = (*rows as usize, *columns as usize);
            if height == 0
                || width == 0
                || height > table.rows.len() - r
                || width > table.columns.len() - c
            {
                return Err(EditError::input(
                    "merge rectangle exceeds table grid or is empty",
                ));
            }
            for row in &table.rows[r..r + height] {
                for cell in &row.cells[c..c + width] {
                    cancel(check)?;
                    let rect = grid.rectangle(&cell.id).expect("validated cell");
                    if rect.row < r
                        || rect.column < c
                        || rect.row + rect.rows > r + height
                        || rect.column + rect.columns > c + width
                    {
                        return Err(EditError::ReferenceConflict(
                            "merge cuts an existing merged rectangle".into(),
                        ));
                    }
                }
            }
            set_rectangle(table, r, c, height, width, check)?;
        }
        TableOperation::Split { cell } => {
            let rect = grid
                .rectangle(cell)
                .ok_or_else(|| EditError::input("table cell does not exist"))?;
            for row in &mut table.rows[rect.row..rect.row + rect.rows] {
                for cell in &mut row.cells[rect.column..rect.column + rect.columns] {
                    cancel(check)?;
                    cell.merge = TableCellMerge::default();
                }
            }
        }
        _ => {
            let mut merges = vec![];
            for row in &table.rows {
                for cell in &row.cells {
                    cancel(check)?;
                    if matches!(cell.merge,TableCellMerge::Span {rows,columns} if rows>1 || columns>1)
                    {
                        merges.push(grid.rectangle(&cell.id).expect("validated origin"));
                    }
                }
            }
            structure::apply(table, operation, &merges, max_cells, check)?;
        }
    }
    Ok(TableGrid::compile(table, max_cells, check)
        .map_err(error)?
        .size())
}

fn set_rectangle(
    table: &mut Table,
    r: usize,
    c: usize,
    rows: usize,
    columns: usize,
    check: &dyn Fn() -> bool,
) -> Result<(), EditError> {
    let origin = table.rows[r].cells[c].id.clone();
    for (dr, row) in table.rows[r..r + rows].iter_mut().enumerate() {
        for (dc, cell) in row.cells[c..c + columns].iter_mut().enumerate() {
            cancel(check)?;
            cell.merge = if dr == 0 && dc == 0 {
                TableCellMerge::Span {
                    rows: u32::try_from(rows).map_err(|_| EditError::input("row span range"))?,
                    columns: u32::try_from(columns)
                        .map_err(|_| EditError::input("column span range"))?,
                }
            } else {
                TableCellMerge::Covered {
                    origin: origin.clone(),
                }
            };
        }
    }
    Ok(())
}
