//! One native table topology and style binding shared by stroke and fill work.
//! Preserves both sides of a shared boundary and all covered-cell declarations;
//! rendering conflict policy is deliberately separate from source computation.
mod stroke;
mod types;
use super::{SourceCellAddress, TableCellEdge, grid, styles::*};
use crate::{
    PptxError, cancelled,
    source::{
        SourceIndex, SourceObjectRef,
        color::*,
        fill::{colors::*, resolve::*},
        line::resolve::*,
    },
};
pub use types::*;

pub fn query(
    index: &SourceIndex,
    request: &SourceTableBorderQuery,
    limits: TableBorderLimits,
    check: &dyn Fn() -> bool,
) -> Result<SourceTableBorders, PptxError> {
    query_in_context(index, request, &request.surface, limits, check)
}
pub fn query_in_context(
    index: &SourceIndex,
    request: &SourceTableBorderQuery,
    drawing_surface: &str,
    limits: TableBorderLimits,
    check: &dyn Fn() -> bool,
) -> Result<SourceTableBorders, PptxError> {
    query_prepared(index, request, drawing_surface, limits, None, check)
}
/// Shares object lookup, merge topology and selected table styles with fill/text.
pub fn query_in_preparation(
    preparation: &mut crate::source::prepared::SourcePreparation<'_>,
    request: &SourceTableBorderQuery,
    drawing_surface: &str,
    limits: TableBorderLimits,
    check: &dyn Fn() -> bool,
) -> Result<SourceTableBorders, PptxError> {
    query_prepared(
        preparation.index(),
        request,
        drawing_surface,
        limits,
        Some(preparation),
        check,
    )
}
fn query_prepared<'a>(
    index: &'a SourceIndex,
    request: &SourceTableBorderQuery,
    drawing_surface: &str,
    limits: TableBorderLimits,
    preparation: Option<&mut crate::source::prepared::SourcePreparation<'a>>,
    check: &dyn Fn() -> bool,
) -> Result<SourceTableBorders, PptxError> {
    cancelled(check)?;
    if request.targets.len() > limits.lines.max_queries.min(limits.fills.fills.max_queries) {
        return Err(PptxError::Limit("table border queries"));
    }
    let fills = SourceFillColorQuery {
        expected_source_sha256: request.expected_source_sha256.clone(),
        surface: request.surface.clone(),
        targets: request
            .targets
            .iter()
            .map(|t| FillTarget::TableCellBorder {
                native_id: t.native_id,
                cell: t.cell,
                edge: t.edge,
            })
            .collect(),
        fill_profile: request.fill_profile,
        color_profile: request.color_profile,
        context: request.context.clone(),
    };
    let (styles, tables) = crate::source::fill::resolve::query_on_page_with_preparation(
        index,
        &SourceFillQuery {
            expected_source_sha256: fills.expected_source_sha256.clone(),
            surface: fills.surface.clone(),
            targets: fills.targets.clone(),
            profile: fills.fill_profile,
        },
        drawing_surface,
        drawing_surface,
        limits.fills.fills,
        preparation,
        check,
    )?;
    let colors = crate::source::fill::colors::colorize(
        index,
        styles,
        &fills,
        drawing_surface,
        drawing_surface,
        limits.fills,
        check,
    )?;
    if colors.targets.len() != request.targets.len() {
        return Err(PptxError::SourceConflict(
            "table border fill result cardinality differs".into(),
        ));
    }
    let mut budget = LineBudget {
        limits: limits.lines,
        check,
        steps: 0,
        values: 0,
        lexical_bytes: 0,
    };
    let part = index
        .surfaces
        .get_key_value(&request.surface)
        .expect("fill surface checked")
        .0
        .as_str();
    let mut targets = Vec::with_capacity(request.targets.len());
    for (target, fill) in request.targets.iter().zip(colors.targets) {
        budget.step()?;
        budget.bytes(request.surface.len())?;
        let object = SourceObjectRef {
            part: request.surface.clone(),
            native_id: target.native_id,
        };
        let mut topology = None;
        let result = (|| {
            let bound = match tables.get(&(part, target.native_id)) {
                Some(Ok(bound)) => bound,
                Some(Err(TablePaintBindIssue::Grid(reason))) => {
                    return Ok(LineGeometryOutcome::Unresolved {
                        reason: LineUnresolved::TableGrid {
                            object,
                            reason: reason.clone(),
                        },
                    });
                }
                Some(Err(TablePaintBindIssue::Style(reason))) => {
                    return Ok(LineGeometryOutcome::Unresolved {
                        reason: LineUnresolved::TableStyle {
                            object,
                            reason: reason.clone(),
                        },
                    });
                }
                None => {
                    return Ok(LineGeometryOutcome::Unresolved {
                        reason: LineUnresolved::UnsupportedObject { object },
                    });
                }
            };
            if let Some(region) = bound.grid.region(target.cell) {
                let table = bound.grid.table();
                let rtl = table
                    .properties
                    .as_ref()
                    .is_some_and(|p| p.right_to_left == Some(true));
                let neighbour = target.edge.neighbour(target.cell, table);
                let neighbour_region = neighbour.and_then(|n| bound.grid.region(n)).copied();
                let at = target.cell;
                let position = match target.edge {
                    TableCellEdge::Top => TableBorderPosition::Horizontal {
                        row: at.row,
                        column: at.column,
                    },
                    TableCellEdge::Bottom => TableBorderPosition::Horizontal {
                        row: at.row + 1,
                        column: at.column,
                    },
                    TableCellEdge::Left => TableBorderPosition::Vertical {
                        row: at.row,
                        column: at.column + u32::from(rtl),
                    },
                    TableCellEdge::Right => TableBorderPosition::Vertical {
                        row: at.row,
                        column: at.column + u32::from(!rtl),
                    },
                    edge => TableBorderPosition::Diagonal { cell: at, edge },
                };
                topology = Some(TableBorderTopology {
                    position,
                    region: *region,
                    neighbour,
                    neighbour_region,
                    inside_merge: neighbour_region.is_some_and(|n| n.origin == region.origin),
                });
            }
            stroke::resolve(
                index,
                &index.surfaces[drawing_surface],
                &object,
                target,
                bound,
                &mut budget,
            )
        })()?;
        targets.push(SourceTableBorderResult {
            target: *target,
            topology,
            stroke: result,
            fill,
        });
    }
    cancelled(check)?;
    Ok(SourceTableBorders {
        source_sha256: colors.source_sha256,
        surface: colors.surface,
        line_profile: request.line_profile,
        fill_profile: colors.fill_profile,
        color_profile: colors.color_profile,
        color_mapping: colors.color_mapping,
        color_scheme: colors.color_scheme,
        targets,
    })
}
