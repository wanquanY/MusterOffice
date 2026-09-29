use super::*;
use crate::{Point, Size};
use std::collections::{BTreeMap, BTreeSet};
use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum TableGridError {
    #[error("invalid table at {path}: {message}")]
    Invalid { path: String, message: &'static str },
    #[error("table grid exceeds cell budget")]
    Limit,
    #[error("table computation cancelled")]
    Cancelled,
}
fn invalid(path: impl Into<String>, message: &'static str) -> TableGridError {
    TableGridError::Invalid {
        path: path.into(),
        message,
    }
}
fn cancel(check: &dyn Fn() -> bool) -> Result<(), TableGridError> {
    if check() {
        Err(TableGridError::Cancelled)
    } else {
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TableRectangle {
    pub row: usize,
    pub column: usize,
    pub rows: usize,
    pub columns: usize,
    pub origin: Point,
    pub size: Size,
}

/// Borrowed, validated structure. It owns only indexes and exact prefix sums,
/// never duplicate cell text or a second mutable copy of the table.
pub struct TableGrid<'a> {
    table: &'a Table,
    positions: BTreeMap<&'a CellId, (usize, usize)>,
    owners: Vec<usize>,
    x: Vec<Emu>,
    y: Vec<Emu>,
}
impl<'a> TableGrid<'a> {
    pub fn compile(
        table: &'a Table,
        max_cells: usize,
        check: &dyn Fn() -> bool,
    ) -> Result<Self, TableGridError> {
        cancel(check)?;
        let columns = table.columns.len();
        let rows = table.rows.len();
        if columns == 0 || rows == 0 {
            return Err(invalid("", "table requires at least one row and column"));
        }
        let count = rows
            .checked_mul(columns)
            .filter(|&n| n <= max_cells)
            .ok_or(TableGridError::Limit)?;
        let mut column_ids = BTreeSet::new();
        let mut row_ids = BTreeSet::new();
        let mut x = vec![Emu::new(0)];
        let mut y = vec![Emu::new(0)];
        for (i, column) in table.columns.iter().enumerate() {
            cancel(check)?;
            if !column_ids.insert(&column.id) {
                return Err(invalid(
                    format!("/columns/{i}/id"),
                    "duplicate column identity",
                ));
            }
            extent(&mut x, column.width, format!("/columns/{i}/width"))?;
        }
        let mut positions = BTreeMap::new();
        for (r, row) in table.rows.iter().enumerate() {
            cancel(check)?;
            if !row_ids.insert(&row.id) {
                return Err(invalid(format!("/rows/{r}/id"), "duplicate row identity"));
            }
            extent(&mut y, row.height, format!("/rows/{r}/height"))?;
            if row.cells.len() != columns {
                return Err(invalid(
                    format!("/rows/{r}/cells"),
                    "row must retain one physical cell per column",
                ));
            }
            for (c, cell) in row.cells.iter().enumerate() {
                cancel(check)?;
                if positions.insert(&cell.id, (r, c)).is_some() {
                    return Err(invalid(
                        format!("/rows/{r}/cells/{c}/id"),
                        "duplicate cell identity",
                    ));
                }
            }
        }
        let mut owners = vec![usize::MAX; count];
        for (r, row) in table.rows.iter().enumerate() {
            for (c, cell) in row.cells.iter().enumerate() {
                cancel(check)?;
                let TableCellMerge::Span {
                    rows: height,
                    columns: width,
                } = cell.merge
                else {
                    continue;
                };
                let height = height as usize;
                let width = width as usize;
                if height == 0 || width == 0 || height > rows - r || width > columns - c {
                    return Err(invalid(
                        format!("/rows/{r}/cells/{c}/merge"),
                        "span exceeds grid or is empty",
                    ));
                }
                let owner = r * columns + c;
                for (dr, covered_row) in table.rows[r..r + height].iter().enumerate() {
                    for (dc, covered) in covered_row.cells[c..c + width].iter().enumerate() {
                        cancel(check)?;
                        let at = (r + dr) * columns + c + dc;
                        if owners[at] != usize::MAX {
                            return Err(invalid(
                                format!("/rows/{r}/cells/{c}/merge"),
                                "merged rectangles overlap",
                            ));
                        }
                        if at != owner
                            && !matches!(&covered.merge, TableCellMerge::Covered { origin } if origin == &cell.id)
                        {
                            return Err(invalid(
                                format!("/rows/{}/cells/{}/merge", r + dr, c + dc),
                                "covered cell must name its upper-left origin",
                            ));
                        }
                        owners[at] = owner;
                    }
                }
            }
        }
        if let Some(at) = owners.iter().position(|&i| i == usize::MAX) {
            return Err(invalid(
                format!("/rows/{}/cells/{}/merge", at / columns, at % columns),
                "covered cell has no enclosing origin rectangle",
            ));
        }
        cancel(check)?;
        Ok(Self {
            table,
            positions,
            owners,
            x,
            y,
        })
    }

    pub fn cell_count(&self) -> usize {
        self.owners.len()
    }
    pub fn size(&self) -> Size {
        Size {
            width: *self.x.last().expect("nonempty columns"),
            height: *self.y.last().expect("nonempty rows"),
        }
    }
    pub fn position(&self, cell: &CellId) -> Option<(usize, usize)> {
        self.positions.get(cell).copied()
    }
    pub fn origin(&self, cell: &CellId) -> Option<&'a TableCell> {
        let (r, c) = self.position(cell)?;
        let columns = self.table.columns.len();
        let at = self.owners[r * columns + c];
        Some(&self.table.rows[at / columns].cells[at % columns])
    }
    /// The visible rectangle of an origin or any physical cell it covers.
    pub fn rectangle(&self, cell: &CellId) -> Option<TableRectangle> {
        let origin = self.origin(cell)?;
        let (r, c) = self.position(&origin.id)?;
        let TableCellMerge::Span { rows, columns } = origin.merge else {
            unreachable!("validated origin")
        };
        let rows = rows as usize;
        let columns = columns as usize;
        Some(TableRectangle {
            row: r,
            column: c,
            rows,
            columns,
            origin: Point {
                x: self.x[c],
                y: self.y[r],
            },
            size: Size {
                width: Emu::new(self.x[c + columns].get() - self.x[c].get()),
                height: Emu::new(self.y[r + rows].get() - self.y[r].get()),
            },
        })
    }
}

fn extent(offsets: &mut Vec<Emu>, value: Emu, path: String) -> Result<(), TableGridError> {
    if value.get() <= 0 {
        return Err(invalid(path, "grid extent must be positive"));
    }
    let total = offsets
        .last()
        .expect("zero prefix")
        .checked_add(value)
        .map_err(|_| invalid(path, "grid extent exceeds coordinate range"))?;
    offsets.push(total);
    Ok(())
}
