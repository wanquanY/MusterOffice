//! Immutable per-source derived table state. This owns no host resources.
use super::*;
use crate::{source_frame::SourceFrameError, source_page::SourcePageError};
use mo_presentation_source::source::{SourceIndex, SourceObjectRef, prepared::*};
use std::{collections::BTreeMap, sync::Arc};
pub(crate) struct RetainedTables {
    pub(crate) source: RetainedSourcePreparation,
    layouts: BTreeMap<(String, u32), Arc<DeclaredTableLayout>>,
}
impl RetainedTables {
    pub(crate) fn capture(
        index: Arc<SourceIndex>,
        source: SourcePreparation<'_>,
        tables: &SharedTables<'_>,
        check: &dyn Fn() -> bool,
    ) -> Result<Self, SourcePageError> {
        let mut layouts = BTreeMap::new();
        for (key, table) in tables {
            if check() {
                return Err(mo_raster::RasterError::Cancelled.into());
            }
            if !std::ptr::eq(index.as_ref(), table.source.index())
                || key.0 != table.source.part()
                || key.1 != table.source.object().native_id
                || !table.geometry.is_bound_to(&table.source)
            {
                return Err(SourcePageError::SourceConflict);
            }
            layouts.insert(key.clone(), table.geometry.retained_layout());
        }
        Ok(Self {
            source: source.retain(index, check)?,
            layouts,
        })
    }
    pub(crate) fn admit(
        &self,
        reference: &SourceObjectRef,
        budget: &mut TablePreparationBudget,
        check: &dyn Fn() -> bool,
    ) -> Result<TableGeometryLimits, SourcePageError> {
        let cost = self
            .source
            .cost(reference)
            .ok_or(SourcePageError::Invalid("unprepared retained table"))?;
        let layout = self
            .layouts
            .get(&(reference.part.clone(), reference.native_id))
            .ok_or(SourcePageError::Invalid(
                "unprepared retained table geometry",
            ))?;
        Ok(budget
            .admit_retained(cost, layout.coordinate_bytes, check)
            .map_err(SourceFrameError::from)?)
    }
    pub(crate) fn geometry<'a>(
        &self,
        table: &PreparedSourceTable<'a>,
        limits: TableGeometryLimits,
        check: &dyn Fn() -> bool,
    ) -> Result<DeclaredTableGeometry<'a>, SourcePageError> {
        if !std::ptr::eq(self.source.index(), table.index()) {
            return Err(SourcePageError::SourceConflict);
        }
        let layout = self
            .layouts
            .get(&(table.part().into(), table.object().native_id))
            .ok_or(SourcePageError::Invalid(
                "unprepared retained table geometry",
            ))?;
        Ok(
            DeclaredTableGeometry::from_retained(table, Arc::clone(layout), limits, check)
                .map_err(SourceFrameError::from)?,
        )
    }
}
