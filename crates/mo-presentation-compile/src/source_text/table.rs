//! Reuse the ordinary itemization/font/shaping path with a native cell scope.
use super::*;
use mo_presentation_source::source::table::{SourceCellAddress, grid::NativeTableGridLimits};

pub enum TableTextPreparation<'a> {
    Prepared { table: TableTextCompiler<'a> },
    Unresolved { issue: SourceTextIssue },
}
pub struct TableTextCompiler<'a> {
    index: &'a SourceIndex,
    table: cascade::TableTextResolver<'a>,
}
impl<'a> TableTextCompiler<'a> {
    pub fn bind(
        index: &'a SourceIndex,
        expected: &Digest,
        object: &SourceObjectRef,
        grid_limits: NativeTableGridLimits,
        limits: SourceTextLimits,
        check: &dyn Fn() -> bool,
    ) -> Result<TableTextPreparation<'a>, SourceTextError> {
        Ok(
            match cascade::TableTextResolver::bind(
                index,
                expected,
                object,
                grid_limits,
                limits.cascade,
                check,
            )? {
                cascade::TableTextBinding::Bound { table } => TableTextPreparation::Prepared {
                    table: Self { index, table },
                },
                cascade::TableTextBinding::Unresolved { reason } => {
                    TableTextPreparation::Unresolved {
                        issue: SourceTextIssue::Cascade { reason },
                    }
                }
            },
        )
    }
    pub fn bind_grid(
        index: &'a SourceIndex,
        expected: &Digest,
        object: &SourceObjectRef,
        grid: std::sync::Arc<mo_presentation_source::source::table::grid::NativeTableGrid<'a>>,
        limits: SourceTextLimits,
        check: &dyn Fn() -> bool,
    ) -> Result<TableTextPreparation<'a>, SourceTextError> {
        Ok(
            match cascade::TableTextResolver::bind_grid(
                index,
                expected,
                object,
                grid,
                limits.cascade,
                check,
            )? {
                cascade::TableTextBinding::Bound { table } => TableTextPreparation::Prepared {
                    table: Self { index, table },
                },
                cascade::TableTextBinding::Unresolved { reason } => {
                    TableTextPreparation::Unresolved {
                        issue: SourceTextIssue::Cascade { reason },
                    }
                }
            },
        )
    }
    pub fn bind_prepared(
        table: &mo_presentation_source::source::prepared::PreparedSourceTable<'a>,
        limits: SourceTextLimits,
        check: &dyn Fn() -> bool,
    ) -> Result<TableTextPreparation<'a>, SourceTextError> {
        Ok(
            match cascade::TableTextResolver::bind_prepared(table, limits.cascade, check)? {
                cascade::TableTextBinding::Bound { table: resolved } => {
                    TableTextPreparation::Prepared {
                        table: Self {
                            index: table.index(),
                            table: resolved,
                        },
                    }
                }
                cascade::TableTextBinding::Unresolved { reason } => {
                    TableTextPreparation::Unresolved {
                        issue: SourceTextIssue::Cascade { reason },
                    }
                }
            },
        )
    }
    /// Prepares one physical cell. A table renderer must call merge origins once;
    /// covered cells remain independently available for editing and diagnostics.
    /// No text fitting or frame/grid height reconciliation is implied here.
    pub fn prepare(
        &self,
        cell: SourceCellAddress,
        limits: SourceTextLimits,
        check: &dyn Fn() -> bool,
    ) -> Result<SourceTextPreparation, SourceTextError> {
        cancel(check)?;
        match self.table.resolve(cell, limits.cascade, check)? {
            cascade::TextCascadeOutcome::Cascaded { text } => {
                prepare_bound(self.index, self.table.object(), *text, limits, check)
            }
            cascade::TextCascadeOutcome::Unresolved { reason } => {
                Ok(SourceTextPreparation::Unresolved {
                    issue: SourceTextIssue::Cascade { reason },
                })
            }
        }
    }
}
