//! Merge-origin cell frames share ordinary text layout and native clipping.
use super::*;
use crate::{source_frame::*, source_text::*};
use mo_common::Digest;
use mo_geometry::Fixed;
use mo_presentation_source::source::{
    SourceIndex, SourceObjectRef, table::SourceCellAddress, text::body::*,
};
use mo_text::{backend::TextBackend, manifest::PreparedManifest};

pub struct TableFrameCompiler<'a> {
    pub(crate) index: &'a SourceIndex,
    pub(crate) geometry: std::sync::Arc<DeclaredTableGeometry<'a>>,
    pub(crate) text: TableTextCompiler<'a>,
    pub(crate) body: CellTextBodyResolver<'a>,
}
impl<'a> TableFrameCompiler<'a> {
    pub fn bind(
        index: &'a SourceIndex,
        expected: &Digest,
        object: &SourceObjectRef,
        limits: TableGeometryLimits,
        check: &dyn Fn() -> bool,
    ) -> Result<Self, SourceFrameError> {
        let geometry = DeclaredTableGeometry::prepare(index, expected, object, limits, check)?;
        let text = match TableTextCompiler::bind_grid(
            index,
            expected,
            object,
            geometry.shared_grid(),
            SourceTextLimits::default(),
            check,
        )? {
            TableTextPreparation::Prepared { table } => table,
            TableTextPreparation::Unresolved { issue } => {
                return Err(SourceFrameError::Mapping(Box::new(
                    SourceFrameIssue::Text { reason: issue },
                )));
            }
        };
        let body =
            CellTextBodyResolver::bind(index, expected, object, TextBodyLimits::default(), check)?;
        Ok(Self {
            index,
            geometry: std::sync::Arc::new(geometry),
            text,
            body,
        })
    }
    pub(crate) fn bind_prepared(
        table: &SharedTableLayout<'a>,
        check: &dyn Fn() -> bool,
    ) -> Result<Self, SourceFrameError> {
        let text = match TableTextCompiler::bind_prepared(
            &table.source,
            SourceTextLimits::default(),
            check,
        )? {
            TableTextPreparation::Prepared { table } => table,
            TableTextPreparation::Unresolved { issue } => {
                return Err(SourceFrameError::Mapping(Box::new(
                    SourceFrameIssue::Text { reason: issue },
                )));
            }
        };
        let body =
            CellTextBodyResolver::bind_prepared(&table.source, TextBodyLimits::default(), check)?;
        Ok(Self {
            index: table.source.index(),
            geometry: std::sync::Arc::clone(&table.geometry),
            text,
            body,
        })
    }
    pub fn geometry(&self) -> &DeclaredTableGeometry<'a> {
        &self.geometry
    }
    pub fn compile(
        &self,
        cell: SourceCellAddress,
        manifest: &PreparedManifest<'_, '_>,
        backend: &mut dyn TextBackend,
        limits: SourceFrameLimits,
        bounds_tolerance: Fixed,
        check: &dyn Fn() -> bool,
    ) -> Result<SourceFramePlan, SourceFrameError> {
        let frame = self.prepare_frame(cell, manifest, limits, bounds_tolerance, check)?;
        crate::source_frame::compute(frame, manifest, backend, limits, check)
    }
}
