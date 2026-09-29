//! Physical native merge topology, shared by layout and future cell operations.
//! Resolves indices only; exact coordinate conversion belongs to the compiler.
use super::{SourceCellAddress, SourceTable, SourceTableCell};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::{collections::BTreeSet, sync::Arc};
#[cfg(test)]
mod tests;
#[derive(Debug, Clone, Copy)]
pub struct NativeTableGridLimits {
    pub max_cells: usize,
    pub max_steps: usize,
    pub max_id_bytes: usize,
}
impl Default for NativeTableGridLimits {
    fn default() -> Self {
        Self {
            max_cells: 1_000_000,
            max_steps: 8_000_000,
            max_id_bytes: 32 * 1024 * 1024,
        }
    }
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum NativeTableGridIssue {
    EmptyGrid,
    RowWidth {
        row: u32,
        actual: u32,
        expected: u32,
    },
    DuplicateCellId {
        cell: SourceCellAddress,
    },
    InvalidSpan {
        cell: SourceCellAddress,
    },
    MissingNeighbour {
        cell: SourceCellAddress,
    },
    ConflictingNeighbours {
        cell: SourceCellAddress,
    },
    OutsideMerge {
        cell: SourceCellAddress,
        origin: SourceCellAddress,
    },
    ConflictingSpan {
        cell: SourceCellAddress,
        origin: SourceCellAddress,
    },
    IncompleteMerge {
        origin: SourceCellAddress,
    },
}
#[derive(Debug, thiserror::Error)]
pub enum NativeTableGridError {
    #[error("native table merge topology: {0:?}")]
    Invalid(NativeTableGridIssue),
    #[error("native table grid budget exceeded")]
    Limit,
    #[error("native table grid cancelled")]
    Cancelled,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct NativeTableRegion {
    pub origin: SourceCellAddress,
    pub rows: u32,
    pub columns: u32,
}
/// Borrowed source plus a linear-sized owner map. Covered payloads are neither
/// copied nor discarded. Querying a cell is O(1), with no repeated grid scans.
pub struct NativeTableGrid<'a> {
    table: &'a SourceTable,
    topology: Arc<NativeTableTopology>,
}
/// Immutable derived indices; source binding remains in NativeTableGrid.
pub(in crate::source) struct NativeTableTopology {
    columns: usize,
    owners: Vec<u32>,
    regions: Vec<NativeTableRegion>,
}
impl<'a> NativeTableGrid<'a> {
    pub fn compile(
        table: &'a SourceTable,
        limits: NativeTableGridLimits,
        check: &dyn Fn() -> bool,
    ) -> Result<Self, NativeTableGridError> {
        use NativeTableGridError::*;
        use NativeTableGridIssue::*;
        let columns = table.columns.len();
        let rows = table.rows.len();
        let count = columns
            .checked_mul(rows)
            .filter(|n| *n <= limits.max_cells && *n <= u32::MAX as usize)
            .ok_or(Limit)?;
        if check() {
            return Err(Cancelled);
        }
        if columns == 0 || rows == 0 {
            return Err(Invalid(EmptyGrid));
        }
        let mut out = NativeTableTopology {
            columns,
            owners: Vec::with_capacity(count),
            regions: vec![],
        };
        let mut counts = Vec::<usize>::new();
        let mut ids = BTreeSet::new();
        let mut id_bytes = limits.max_id_bytes;
        let mut steps = 0usize;
        let mut step = || {
            if check() {
                return Err(Cancelled);
            }
            steps = steps
                .checked_add(1)
                .filter(|n| *n <= limits.max_steps)
                .ok_or(Limit)?;
            Ok(())
        };
        for (r, row) in table.rows.iter().enumerate() {
            step()?;
            if row.cells.len() != columns {
                return Err(Invalid(RowWidth {
                    row: r as u32,
                    actual: row.cells.len().try_into().map_err(|_| Limit)?,
                    expected: columns as u32,
                }));
            }
            for (c, cell) in row.cells.iter().enumerate() {
                step()?;
                let address = SourceCellAddress {
                    row: r as u32,
                    column: c as u32,
                };
                if let Some(id) = cell.native_id.as_deref() {
                    id_bytes = id_bytes.checked_sub(id.len()).ok_or(Limit)?;
                    if !ids.insert(id) {
                        return Err(Invalid(DuplicateCellId { cell: address }));
                    }
                }
                let height = cell.row_span.unwrap_or(1);
                let width = cell.grid_span.unwrap_or(1);
                if height < 1 || width < 1 {
                    return Err(Invalid(InvalidSpan { cell: address }));
                }
                let h = cell.horizontal_merge == Some(true);
                let v = cell.vertical_merge == Some(true);
                let previous = if h {
                    if c == 0 {
                        return Err(Invalid(MissingNeighbour { cell: address }));
                    }
                    Some(out.owners[r * columns + c - 1])
                } else {
                    None
                };
                let above = if v {
                    if r == 0 {
                        return Err(Invalid(MissingNeighbour { cell: address }));
                    }
                    Some(out.owners[(r - 1) * columns + c])
                } else {
                    None
                };
                if previous.is_some() && above.is_some() && previous != above {
                    return Err(Invalid(ConflictingNeighbours { cell: address }));
                }
                let owner = if let Some(owner) = previous.or(above) {
                    let root = out.regions[owner as usize];
                    if r < root.origin.row as usize
                        || c < root.origin.column as usize
                        || r >= root.origin.row as usize + root.rows as usize
                        || c >= root.origin.column as usize + root.columns as usize
                    {
                        return Err(Invalid(OutsideMerge {
                            cell: address,
                            origin: root.origin,
                        }));
                    }
                    // Covered cells may omit redundant spans. Explicit spans
                    // greater than one must agree with the rectangular owner.
                    if (height > 1
                        && (address.row != root.origin.row || height as u32 != root.rows))
                        || (width > 1
                            && (address.column != root.origin.column
                                || width as u32 != root.columns))
                    {
                        return Err(Invalid(ConflictingSpan {
                            cell: address,
                            origin: root.origin,
                        }));
                    }
                    owner
                } else {
                    if (height as usize) > rows - r || (width as usize) > columns - c {
                        return Err(Invalid(InvalidSpan { cell: address }));
                    }
                    let owner = out.regions.len() as u32;
                    out.regions.push(NativeTableRegion {
                        origin: address,
                        rows: height as u32,
                        columns: width as u32,
                    });
                    counts.push(0);
                    owner
                };
                counts[owner as usize] += 1;
                out.owners.push(owner);
            }
        }
        for (region, count) in out.regions.iter().zip(counts) {
            step()?;
            if count != (region.rows as usize) * (region.columns as usize) {
                return Err(Invalid(IncompleteMerge {
                    origin: region.origin,
                }));
            }
        }
        Ok(Self {
            table,
            topology: Arc::new(out),
        })
    }
    pub(in crate::source) fn topology(&self) -> Arc<NativeTableTopology> {
        Arc::clone(&self.topology)
    }
    pub(in crate::source) fn bind_topology(
        table: &'a SourceTable,
        topology: Arc<NativeTableTopology>,
    ) -> Self {
        Self { table, topology }
    }
    pub fn regions(&self) -> &[NativeTableRegion] {
        &self.topology.regions
    }
    pub fn table(&self) -> &'a SourceTable {
        self.table
    }
    pub fn cell(&self, at: SourceCellAddress) -> Option<&'a SourceTableCell> {
        self.table
            .rows
            .get(at.row as usize)?
            .cells
            .get(at.column as usize)
    }
    pub fn region(&self, at: SourceCellAddress) -> Option<&NativeTableRegion> {
        self.cell(at)?;
        self.topology.regions.get(
            self.topology.owners[at.row as usize * self.topology.columns + at.column as usize]
                as usize,
        )
    }
}
