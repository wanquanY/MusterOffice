//! Declared native table geometry in table-local coordinates. Row text fitting,
//! style resolution, borders, clipping and world placement are separate stages.
//! This must not be advertised as a complete rendered table.
mod geometry;
pub use geometry::*;

mod frame;
pub use frame::TableFrameCompiler;

mod budget;
pub(crate) use budget::TablePreparationBudget;

/// One actual table binding and coordinate layout, shared across paint and text.
pub(crate) struct SharedTableLayout<'a> {
    pub source: std::sync::Arc<mo_presentation_source::source::prepared::PreparedSourceTable<'a>>,
    pub geometry: std::sync::Arc<DeclaredTableGeometry<'a>>,
}

pub(crate) type SharedTables<'a> = std::collections::BTreeMap<(String, u32), SharedTableLayout<'a>>;

mod retained;
pub(crate) use retained::RetainedTables;
