//! Bounded preparation against one immutable source. Derived grids and styles
//! retain actual source bindings; consumers cannot substitute same-id objects.
use super::table::{grid::*, styles::*};
use super::{SourceIndex, SourceObject, SourceObjectKind, SourceObjectRef, SourceSurface};
use crate::{PptxError, cancelled};
use mo_common::Digest;
use std::{collections::BTreeMap, sync::Arc};

mod objects;
pub(in crate::source) use objects::ObjectBindings;
mod retained;
pub use retained::RetainedSourcePreparation;
#[derive(Debug, Clone, Copy)]
pub struct SourcePreparationLimits {
    pub max_surfaces: usize,
    pub max_objects: usize,
    pub max_tables: usize,
    pub grids: NativeTableGridLimits,
}
impl Default for SourcePreparationLimits {
    fn default() -> Self {
        Self {
            max_surfaces: 10_000,
            max_objects: 100_000,
            max_tables: 10_000,
            grids: Default::default(),
        }
    }
}
/// Logical admitted state, not a wall-clock or allocation-size measurement.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct SourcePreparationWork {
    pub indexed_objects: usize,
    pub compiled_tables: usize,
    pub reused_tables: usize,
    pub grid_cells: usize,
    pub grid_steps: usize,
    pub grid_id_bytes: usize,
}
#[derive(Debug, Clone, Copy)]
pub struct PreparedTableCost {
    pub cells: usize,
    pub steps: usize,
    pub id_bytes: usize,
}
pub struct PreparedSourceTable<'a> {
    index: &'a SourceIndex,
    part: &'a str,
    surface: &'a SourceSurface,
    object: &'a SourceObject,
    grid: Arc<NativeTableGrid<'a>>,
    style: Result<Arc<BoundTableStyle<'a>>, TableStyleSelectionError>,
    cost: PreparedTableCost,
}
impl<'a> PreparedSourceTable<'a> {
    pub fn index(&self) -> &'a SourceIndex {
        self.index
    }
    pub fn part(&self) -> &'a str {
        self.part
    }
    pub fn surface(&self) -> &'a SourceSurface {
        self.surface
    }
    pub fn object(&self) -> &'a SourceObject {
        self.object
    }
    pub fn reference(&self) -> SourceObjectRef {
        SourceObjectRef {
            part: self.part.into(),
            native_id: self.object.native_id,
        }
    }
    pub fn grid(&self) -> &NativeTableGrid<'a> {
        &self.grid
    }
    pub fn shared_grid(&self) -> Arc<NativeTableGrid<'a>> {
        Arc::clone(&self.grid)
    }
    pub fn style(&self) -> Result<Arc<BoundTableStyle<'a>>, TableStyleSelectionError> {
        self.style.clone()
    }
    pub fn cost(&self) -> PreparedTableCost {
        self.cost
    }
}
#[derive(Clone)]
pub enum SourceTablePreparation<'a> {
    Prepared {
        table: Arc<PreparedSourceTable<'a>>,
    },
    InvalidGrid {
        reason: NativeTableGridIssue,
        cost: PreparedTableCost,
    },
}
impl SourceTablePreparation<'_> {
    pub fn cost(&self) -> PreparedTableCost {
        match self {
            Self::Prepared { table } => table.cost(),
            Self::InvalidGrid { cost, .. } => *cost,
        }
    }
}
/// Invocation-scoped computation state. No files, permissions, model, network or
/// persistence. An aborted preparation is poisoned and cannot continue partially.
pub struct SourcePreparation<'a> {
    index: &'a SourceIndex,
    objects: ObjectBindings<'a>,
    tables: BTreeMap<(&'a str, u32), SourceTablePreparation<'a>>,
    limits: SourcePreparationLimits,
    work: SourcePreparationWork,
    failed: bool,
    retained: Option<&'a RetainedSourcePreparation>,
}
fn conflict() -> PptxError {
    PptxError::SourceConflict("source preparation binding differs".into())
}
impl<'a> SourcePreparation<'a> {
    pub fn new(
        index: &'a SourceIndex,
        expected: &Digest,
        limits: SourcePreparationLimits,
        check: &dyn Fn() -> bool,
    ) -> Result<Self, PptxError> {
        cancelled(check)?;
        if &index.source_sha256 != expected {
            return Err(conflict());
        }
        if index.surfaces.len() > limits.max_surfaces {
            return Err(PptxError::Limit("prepared source surfaces"));
        }
        let mut count = 0usize;
        let objects = ObjectBindings::new(index, &mut || {
            cancelled(check)?;
            count = count
                .checked_add(1)
                .filter(|n| *n <= limits.max_objects)
                .ok_or(PptxError::Limit("prepared source objects"))?;
            Ok(())
        })?;
        Ok(Self {
            index,
            objects,
            tables: BTreeMap::new(),
            limits,
            work: SourcePreparationWork {
                indexed_objects: count,
                ..Default::default()
            },
            failed: false,
            retained: None,
        })
    }
    pub fn index(&self) -> &'a SourceIndex {
        self.index
    }
    pub fn work(&self) -> SourcePreparationWork {
        self.work
    }
    pub(in crate::source) fn bindings(&self) -> Result<ObjectBindings<'a>, PptxError> {
        self.ensure_ready()?;
        Ok(self.objects.clone())
    }
    fn ensure_ready(&self) -> Result<(), PptxError> {
        if self.failed {
            Err(PptxError::SourceConflict(
                "failed source preparation".into(),
            ))
        } else {
            Ok(())
        }
    }
    pub fn table(
        &mut self,
        object: &SourceObjectRef,
        check: &dyn Fn() -> bool,
    ) -> Result<SourceTablePreparation<'a>, PptxError> {
        self.ensure_ready()?;
        let result = self.prepare_table(object, check);
        self.failed = result.is_err();
        result
    }
    fn prepare_table(
        &mut self,
        reference: &SourceObjectRef,
        check: &dyn Fn() -> bool,
    ) -> Result<SourceTablePreparation<'a>, PptxError> {
        cancelled(check)?;
        let part = self
            .index
            .surfaces
            .get_key_value(&reference.part)
            .ok_or_else(conflict)?
            .0;
        let (key, object) = self
            .objects
            .get_key_value(&(part.as_str(), reference.native_id))
            .ok_or_else(conflict)?;
        if let Some(table) = self.tables.get(&key) {
            return Ok(table.clone());
        }
        if let Some(retained) = self.retained {
            let table = retained.bind(key, object, check)?;
            self.tables.insert(key, table.clone());
            self.work.reused_tables += 1;
            return Ok(table);
        }
        if self.tables.len() >= self.limits.max_tables {
            return Err(PptxError::Limit("prepared source tables"));
        }
        let table = object
            .table
            .as_ref()
            .filter(|_| object.kind == SourceObjectKind::GraphicFrame)
            .ok_or_else(conflict)?;
        let cells = table
            .rows
            .len()
            .checked_mul(table.columns.len())
            .ok_or(PptxError::Limit("prepared table cells"))?;
        let steps = cells
            .checked_mul(2)
            .and_then(|n| n.checked_add(table.rows.len()))
            .ok_or(PptxError::Limit("prepared table steps"))?;
        let next_cells = self
            .work
            .grid_cells
            .checked_add(cells)
            .filter(|n| *n <= self.limits.grids.max_cells)
            .ok_or(PptxError::Limit("prepared table cells"))?;
        let next_steps = self
            .work
            .grid_steps
            .checked_add(steps)
            .filter(|n| *n <= self.limits.grids.max_steps)
            .ok_or(PptxError::Limit("prepared table steps"))?;
        let mut id_bytes = 0usize;
        for row in &table.rows {
            cancelled(check)?;
            // Do not visit cell payload outside the reserved rectangular grid.
            if row.cells.len() != table.columns.len() {
                break;
            }
            for cell in &row.cells {
                cancelled(check)?;
                if let Some(id) = &cell.native_id {
                    id_bytes = id_bytes
                        .checked_add(id.len())
                        .filter(|n| {
                            self.work
                                .grid_id_bytes
                                .checked_add(*n)
                                .is_some_and(|v| v <= self.limits.grids.max_id_bytes)
                        })
                        .ok_or(PptxError::Limit("prepared table id bytes"))?;
                }
            }
        }
        self.work.grid_cells = next_cells;
        self.work.grid_steps = next_steps;
        self.work.grid_id_bytes += id_bytes;
        self.work.compiled_tables += 1;
        let grid = match NativeTableGrid::compile(
            table,
            NativeTableGridLimits {
                max_cells: cells,
                max_steps: steps,
                max_id_bytes: id_bytes,
            },
            check,
        ) {
            Ok(grid) => Arc::new(grid),
            Err(NativeTableGridError::Invalid(reason)) => {
                let result = SourceTablePreparation::InvalidGrid {
                    reason,
                    cost: PreparedTableCost {
                        cells,
                        steps,
                        id_bytes,
                    },
                };
                self.tables.insert(key, result.clone());
                return Ok(result);
            }
            Err(NativeTableGridError::Limit) => {
                return Err(PptxError::Limit("prepared table grid"));
            }
            Err(NativeTableGridError::Cancelled) => return Err(PptxError::Cancelled),
        };
        let style = match BoundTableStyle::bind(table, self.index.table_styles.as_deref(), check) {
            Err(TableStyleSelectionError::Cancelled) => return Err(PptxError::Cancelled),
            result => result.map(Arc::new),
        };
        let prepared = SourceTablePreparation::Prepared {
            table: Arc::new(PreparedSourceTable {
                index: self.index,
                part: key.0,
                surface: &self.index.surfaces[key.0],
                object,
                grid,
                style,
                cost: PreparedTableCost {
                    cells,
                    steps,
                    id_bytes,
                },
            }),
        };
        self.tables.insert(key, prepared.clone());
        Ok(prepared)
    }
}
