//! Owned immutable source and derived topology. No self-references or unsafe
//! lifetime extension; sessions borrow this owner and cannot admit new tables.
use super::*;
struct Table {
    grid: Result<Arc<NativeTableTopology>, NativeTableGridIssue>,
    style: Result<RetainedTableStyle, TableStyleSelectionError>,
    cost: PreparedTableCost,
}
pub struct RetainedSourcePreparation {
    index: Arc<SourceIndex>,
    directory: Arc<objects::ObjectDirectory>,
    tables: BTreeMap<String, BTreeMap<u32, Table>>,
    limits: SourcePreparationLimits,
    work: SourcePreparationWork,
}
impl<'a> SourcePreparation<'a> {
    /// Freeze only against the exact immutable owner that was prepared. A clone
    /// with the same digest cannot substitute a different inspected index.
    pub fn retain(
        self,
        index: Arc<SourceIndex>,
        check: &dyn Fn() -> bool,
    ) -> Result<RetainedSourcePreparation, PptxError> {
        self.ensure_ready()?;
        cancelled(check)?;
        if !std::ptr::eq(index.as_ref(), self.index) {
            return Err(conflict());
        }
        let mut tables = BTreeMap::<String, BTreeMap<u32, Table>>::new();
        let mut work = SourcePreparationWork {
            indexed_objects: self.objects.len(),
            ..Default::default()
        };
        for ((part, id), prepared) in self.tables {
            cancelled(check)?;
            let table = match prepared {
                SourceTablePreparation::Prepared { table } => Table {
                    grid: Ok(table.grid.topology()),
                    style: table
                        .style
                        .as_ref()
                        .map(|s| s.retain())
                        .map_err(Clone::clone),
                    cost: table.cost,
                },
                SourceTablePreparation::InvalidGrid { reason, cost } => Table {
                    grid: Err(reason),
                    style: Err(TableStyleSelectionError::GridMismatch),
                    cost,
                },
            };
            // This is the retained admission, including when a session is
            // frozen again. Session counters only describe its new/reused work.
            work.compiled_tables += 1;
            work.grid_cells += table.cost.cells;
            work.grid_steps += table.cost.steps;
            work.grid_id_bytes += table.cost.id_bytes;
            tables.entry(part.into()).or_default().insert(id, table);
        }
        cancelled(check)?;
        Ok(RetainedSourcePreparation {
            index,
            directory: self.objects.directory,
            tables,
            limits: self.limits,
            work,
        })
    }
}
impl RetainedSourcePreparation {
    pub fn index(&self) -> &SourceIndex {
        &self.index
    }
    pub fn work(&self) -> SourcePreparationWork {
        self.work
    }
    pub fn session(&self, check: &dyn Fn() -> bool) -> Result<SourcePreparation<'_>, PptxError> {
        cancelled(check)?;
        Ok(SourcePreparation {
            index: &self.index,
            objects: ObjectBindings::retained(&self.index, Arc::clone(&self.directory)),
            tables: BTreeMap::new(),
            limits: self.limits,
            work: SourcePreparationWork {
                indexed_objects: self.work.indexed_objects,
                ..Default::default()
            },
            failed: false,
            retained: Some(self),
        })
    }
    pub fn cost(&self, reference: &SourceObjectRef) -> Option<PreparedTableCost> {
        self.tables
            .get(&reference.part)?
            .get(&reference.native_id)
            .map(|t| t.cost)
    }
    pub(super) fn bind<'a>(
        &'a self,
        key: (&'a str, u32),
        object: &'a SourceObject,
        check: &dyn Fn() -> bool,
    ) -> Result<SourceTablePreparation<'a>, PptxError> {
        cancelled(check)?;
        let retained = self
            .tables
            .get(key.0)
            .and_then(|s| s.get(&key.1))
            .ok_or_else(conflict)?;
        let grid = match &retained.grid {
            Ok(grid) => Arc::clone(grid),
            Err(reason) => {
                return Ok(SourceTablePreparation::InvalidGrid {
                    reason: reason.clone(),
                    cost: retained.cost,
                });
            }
        };
        let native = object.table.as_ref().ok_or_else(conflict)?;
        let style = match &retained.style {
            Ok(style) => {
                BoundTableStyle::bind_retained(native, self.index.table_styles.as_deref(), style)
                    .map(Arc::new)
            }
            Err(reason) => Err(reason.clone()),
        };
        Ok(SourceTablePreparation::Prepared {
            table: Arc::new(PreparedSourceTable {
                index: &self.index,
                part: key.0,
                surface: &self.index.surfaces[key.0],
                object,
                grid: Arc::new(NativeTableGrid::bind_topology(native, grid)),
                style,
                cost: retained.cost,
            }),
        })
    }
}
