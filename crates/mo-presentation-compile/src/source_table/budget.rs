//! Aggregate native-table admission shared by page paint and text preparation.
//! Charge before building a grid or allocating any per-cell query arrays.
use super::{TableGeometryError, TableGeometryLimits};
use mo_presentation_source::source::table::SourceTable;

pub(crate) struct TablePreparationBudget {
    remaining: TableGeometryLimits,
}
fn charge(remaining: &mut usize, n: usize, label: &'static str) -> Result<(), TableGeometryError> {
    *remaining = remaining
        .checked_sub(n)
        .ok_or(TableGeometryError::Limit(label))?;
    Ok(())
}
impl TablePreparationBudget {
    pub fn new(limits: TableGeometryLimits) -> Self {
        Self { remaining: limits }
    }
    pub fn admit_retained(
        &mut self,
        cost: mo_presentation_source::source::prepared::PreparedTableCost,
        coordinate_bytes: usize,
        check: &dyn Fn() -> bool,
    ) -> Result<TableGeometryLimits, TableGeometryError> {
        if check() {
            return Err(TableGeometryError::Cancelled);
        }
        charge(
            &mut self.remaining.grid.max_cells,
            cost.cells,
            "page table cells",
        )?;
        charge(
            &mut self.remaining.grid.max_steps,
            cost.steps,
            "page table steps",
        )?;
        charge(
            &mut self.remaining.grid.max_id_bytes,
            cost.id_bytes,
            "page table id bytes",
        )?;
        charge(
            &mut self.remaining.max_coordinate_bytes,
            coordinate_bytes,
            "page table coordinate bytes",
        )?;
        Ok(TableGeometryLimits {
            grid: mo_presentation_source::source::table::grid::NativeTableGridLimits {
                max_cells: cost.cells,
                max_steps: cost.steps,
                max_id_bytes: cost.id_bytes,
            },
            max_objects: self.remaining.max_objects,
            max_coordinate_bytes: coordinate_bytes,
        })
    }
    pub fn admit(
        &mut self,
        table: &SourceTable,
        check: &dyn Fn() -> bool,
    ) -> Result<TableGeometryLimits, super::TableGeometryError> {
        let cells = table
            .rows
            .len()
            .checked_mul(table.columns.len())
            .ok_or(super::TableGeometryError::Limit("page table cells"))?;
        let steps = cells
            .checked_mul(2)
            .and_then(|v| v.checked_add(table.rows.len()))
            .ok_or(super::TableGeometryError::Limit("page table steps"))?;
        charge(
            &mut self.remaining.grid.max_cells,
            cells,
            "page table cells",
        )?;
        charge(
            &mut self.remaining.grid.max_steps,
            steps,
            "page table steps",
        )?;
        let mut id_bytes = 0usize;
        let mut coordinate_bytes = 0usize;
        for (row_index, row) in table.rows.iter().enumerate() {
            if check() {
                return Err(super::TableGeometryError::Cancelled);
            }
            // The reservation above bounds declared cells. Reject a malformed
            // inspected index before walking any unreserved physical payload.
            if row.cells.len() != table.columns.len() {
                use mo_presentation_source::source::table::grid::{
                    NativeTableGridError, NativeTableGridIssue,
                };
                return Err(
                    NativeTableGridError::Invalid(NativeTableGridIssue::RowWidth {
                        row: u32::try_from(row_index)
                            .map_err(|_| TableGeometryError::Limit("page table rows"))?,
                        actual: u32::try_from(row.cells.len())
                            .map_err(|_| TableGeometryError::Limit("page table cells"))?,
                        expected: u32::try_from(table.columns.len())
                            .map_err(|_| TableGeometryError::Limit("page table columns"))?,
                    })
                    .into(),
                );
            }
            charge(
                &mut self.remaining.max_coordinate_bytes,
                row.height.lexical().len(),
                "page table coordinate bytes",
            )?;
            coordinate_bytes += row.height.lexical().len();
            for cell in &row.cells {
                if check() {
                    return Err(super::TableGeometryError::Cancelled);
                }
                if let Some(id) = &cell.native_id {
                    charge(
                        &mut self.remaining.grid.max_id_bytes,
                        id.len(),
                        "page table id bytes",
                    )?;
                    id_bytes += id.len();
                }
            }
        }
        for column in &table.columns {
            if check() {
                return Err(super::TableGeometryError::Cancelled);
            }
            charge(
                &mut self.remaining.max_coordinate_bytes,
                column.width.lexical().len(),
                "page table coordinate bytes",
            )?;
            coordinate_bytes += column.width.lexical().len();
        }
        Ok(TableGeometryLimits {
            grid: mo_presentation_source::source::table::grid::NativeTableGridLimits {
                max_cells: cells,
                max_steps: steps,
                max_id_bytes: id_bytes,
            },
            max_objects: self.remaining.max_objects,
            max_coordinate_bytes: coordinate_bytes,
        })
    }
}
