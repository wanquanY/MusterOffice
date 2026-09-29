//! Preserve distinct native sides. Only equivalent evaluated sides may share
//! a draw until application-calibrated conflict precedence is implemented.
use super::*;
use mo_presentation_source::source::{
    color::ColorSample,
    table::{TableCellEdge, borders::*},
};
use mo_raster::StrokeStyle;
use std::collections::BTreeMap;
type Key = (u8, u32, u32);
type PaintKey = Option<([u64; 4], StrokeStyle)>;
struct Edge {
    key: PaintKey,
    source: SourceTableBorderResult,
}
fn key(position: &TableBorderPosition) -> Key {
    match position {
        TableBorderPosition::Horizontal { row, column } => (0, *row, *column),
        TableBorderPosition::Vertical { row, column } => (1, *row, *column),
        TableBorderPosition::Diagonal { cell, edge } => (
            2 + u8::from(*edge == TableCellEdge::BottomLeftToTopRight),
            cell.row,
            cell.column,
        ),
    }
}
pub(super) fn prepare(
    source: &mut SourcePreparation<'_>,
    q: &SourcePageRequest,
    base: &SourcePagePaintBinding,
    geometry: &DeclaredTableGeometry<'_>,
    check: &dyn Fn() -> bool,
) -> Result<Vec<PreparedPaint>, SourcePageError> {
    let native_id = base.location.object.expect("table object");
    let table = geometry.grid().table();
    let mut targets = Vec::new();
    for (r, row) in table.rows.iter().enumerate() {
        for c in 0..row.cells.len() {
            cancel(check)?;
            for edge in [
                TableCellEdge::Left,
                TableCellEdge::Right,
                TableCellEdge::Top,
                TableCellEdge::Bottom,
                TableCellEdge::TopLeftToBottomRight,
                TableCellEdge::BottomLeftToTopRight,
            ] {
                targets.push(TableBorderTarget {
                    native_id,
                    cell: SourceCellAddress {
                        row: r as u32,
                        column: c as u32,
                    },
                    edge,
                });
            }
        }
    }
    let limit = targets.len();
    let result = mo_presentation_source::source::table::borders::query_in_preparation(
        source,
        &SourceTableBorderQuery {
            expected_source_sha256: q.expected_source_sha256.clone(),
            surface: base.location.part.clone(),
            targets,
            line_profile: LineProfile::Drawingml2024DraftV1,
            fill_profile: FillProfile::Drawingml2024DraftV1,
            color_profile: ColorProfile::Ecma3762016DraftV1,
            context: q.color_context.clone(),
        },
        &base.location.part,
        TableBorderLimits {
            fills: fill_limits(limit),
            lines: LineResolveLimits {
                max_queries: limit,
                ..Default::default()
            },
        },
        check,
    )?;
    if result.targets.len() != limit {
        return Err(SourcePageError::Invalid("table border cardinality"));
    }
    let mut selected = BTreeMap::<Key, Edge>::new();
    for source in result.targets {
        cancel(check)?;
        let topology = source
            .topology
            .as_ref()
            .ok_or_else(|| mapping(&base.location, SourcePageIssue::Line {}))?;
        if topology.inside_merge {
            continue;
        }
        if matches!(topology.position, TableBorderPosition::Diagonal { .. })
            && topology.region.origin != source.target.cell
        {
            continue;
        }
        let value = match &source.fill.colors {
            FillPaintColors::None {} => None,
            FillPaintColors::Solid { color } => {
                let ColorSample::Resolved { srgb, .. } = color.outcome else {
                    return Err(mapping(&base.location, SourcePageIssue::Line {}));
                };
                let LineGeometryOutcome::Resolved { geometry } = &source.stroke else {
                    return Err(mapping(&base.location, SourcePageIssue::Line {}));
                };
                Some((
                    srgb.map(f64::to_bits),
                    paint::table_stroke(geometry, &base.location)?,
                ))
            }
            _ => return Err(mapping(&base.location, SourcePageIssue::Line {})),
        };
        let position = key(&topology.position);
        if let Some(previous) = selected.get(&position) {
            if previous.key != value {
                return Err(mapping(
                    &base.location,
                    SourcePageIssue::TableBorderConflict {
                        first: previous.source.target,
                        second: source.target,
                    },
                ));
            }
        } else {
            selected.insert(position, Edge { key: value, source });
        }
    }
    let mut paints = Vec::new();
    for edge in selected.into_values() {
        cancel(check)?;
        let Some((_, stroke)) = edge.key else {
            continue;
        };
        let cell = geometry.cell(edge.source.target.cell).expect("edge cell");
        let rect = cell.physical;
        let merged = cell.merged;
        let (a, b) = match edge.source.target.edge {
            TableCellEdge::Left => (
                rect.min,
                Point {
                    x: rect.min.x,
                    y: rect.max.y,
                },
            ),
            TableCellEdge::Right => (
                Point {
                    x: rect.max.x,
                    y: rect.min.y,
                },
                rect.max,
            ),
            TableCellEdge::Top => (
                rect.min,
                Point {
                    x: rect.max.x,
                    y: rect.min.y,
                },
            ),
            TableCellEdge::Bottom => (
                Point {
                    x: rect.min.x,
                    y: rect.max.y,
                },
                rect.max,
            ),
            TableCellEdge::TopLeftToBottomRight => (merged.min, merged.max),
            TableCellEdge::BottomLeftToTopRight => (
                Point {
                    x: merged.min.x,
                    y: merged.max.y,
                },
                Point {
                    x: merged.max.x,
                    y: merged.min.y,
                },
            ),
        };
        let mut path = rectangle(rect, cell.source_ordinal, cell.conversion_error_bound);
        path.commands = vec![C::Move { to: a }, C::Line { to: b }];
        path.source_map[0].command_count = 2;
        path.fill = Some(NativePathFill::None);
        path.stroke = Some(true);
        shifted(
            &mut path.commands,
            base.placement.as_ref().expect("table placement").anchor,
            check,
        )?;
        let FillPaintColors::Solid { color } = &edge.source.fill.colors else {
            unreachable!()
        };
        let ColorSample::Resolved { rgba8, .. } = color.outcome else {
            unreachable!()
        };
        let mut binding = base.clone();
        binding.fill = edge.source.fill;
        binding.table_stroke = Some(edge.source.stroke);
        binding.region = Some(SourcePaintRegion {
            bounds: merged,
            coordinate_error_bound: cell.conversion_error_bound,
        });
        paints.push(PreparedPaint {
            binding: Some(binding),
            paths: vec![path],
            fill: None,
            picture_fill: None,
            line: Some((rgba8, stroke)),
        });
    }
    Ok(paints)
}
