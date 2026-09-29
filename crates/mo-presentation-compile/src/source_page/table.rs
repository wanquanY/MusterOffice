//! Native table receivers retain the original object/cell identities. The
//! ordinary fill, stroke, placement, opacity and scene emitter paint them.
mod borders;
use super::prepared::PreparedPaint;
use super::*;
use crate::source_table::{DeclaredTableGeometry, TableGeometryLimits, TablePreparationBudget};
use mo_geometry::{BoundsBudget, Rect};
use mo_presentation_source::source::{prepared::SourcePreparation, table::SourceCellAddress};
use std::collections::{BTreeMap, BTreeSet};

type AdmittedTables = BTreeMap<(String, u32), TableGeometryLimits>;
/// Reserve every visible table before style queries or derived receivers allocate.
/// Query capacity is separate from painted draw capacity: noFill declarations
/// still require bounded source work, even when they ultimately emit no ink.
pub(super) fn admit(
    index: &SourceIndex,
    layers: &[SourcePageLayer],
    retained: Option<&crate::source_table::RetainedTables>,
    check: &dyn Fn() -> bool,
) -> Result<AdmittedTables, SourcePageError> {
    let mut budget = TablePreparationBudget::new(TableGeometryLimits::default());
    let mut queries = mo_raster::MAX_DRAWS;
    let mut admitted = BTreeMap::new();
    for layer in layers {
        let visible: BTreeSet<_> = layer.objects.iter().copied().collect();
        for object in &index.surfaces[&layer.part].objects {
            cancel(check)?;
            if !visible.contains(&object.native_id) {
                continue;
            }
            let Some(table) = &object.table else {
                continue;
            };
            let limits = if let Some(retained) = retained {
                retained.admit(
                    &mo_presentation_source::source::SourceObjectRef {
                        part: layer.part.clone(),
                        native_id: object.native_id,
                    },
                    &mut budget,
                    check,
                )?
            } else {
                budget
                    .admit(table, check)
                    .map_err(crate::source_frame::SourceFrameError::from)?
            };
            // Six declared border sides, at most one cell paint and one table
            // background. Charge physical cells, before merge compaction.
            let count = limits
                .grid
                .max_cells
                .checked_mul(7)
                .and_then(|v| v.checked_add(1))
                .ok_or(RasterError::Limit("page table paint queries"))?;
            queries = queries
                .checked_sub(count)
                .ok_or(RasterError::Limit("page table paint queries"))?;
            if admitted
                .insert((layer.part.clone(), object.native_id), limits)
                .is_some()
            {
                return Err(SourcePageError::Invalid("duplicate table admission"));
            }
        }
    }
    Ok(admitted)
}

pub(super) fn objects(
    index: &SourceIndex,
    q: &SourcePageRequest,
    layers: &[SourcePageLayer],
    transforms: Option<&SourceProperties>,
    preparation: &mut SourcePreparation<'_>,
    check: &dyn Fn() -> bool,
) -> Result<Vec<Object>, SourcePageError> {
    let mut output = Vec::new();
    for layer in layers {
        if layer.objects.is_empty() {
            continue;
        }
        cancel(check)?;
        let placements = source_placements_sampled(
            index,
            &SourcePlacementQuery {
                expected_source_sha256: q.expected_source_sha256.clone(),
                surface: layer.part.clone(),
                objects: layer.objects.clone(),
                profile: SourcePlacementProfile::DrawingmlSourceDraftV1,
            },
            SourcePlacementLimits {
                max_queries: 8192,
                ..Default::default()
            },
            transforms,
            check,
        )?;
        let fills = mo_presentation_source::source::fill::colors::query_in_preparation(
            preparation,
            &SourceFillColorQuery {
                expected_source_sha256: q.expected_source_sha256.clone(),
                surface: layer.part.clone(),
                targets: layer
                    .objects
                    .iter()
                    .map(|id| FillTarget::TableBackground { native_id: *id })
                    .collect(),
                fill_profile: FillProfile::Drawingml2024DraftV1,
                color_profile: ColorProfile::Ecma3762016DraftV1,
                context: q.color_context.clone(),
            },
            &layer.part,
            &q.slide,
            fill_limits(8192),
            check,
        )?;
        if placements.objects.len() != layer.objects.len()
            || fills.targets.len() != layer.objects.len()
        {
            return Err(SourcePageError::Invalid("table page query cardinality"));
        }
        for ((id, placed), fill) in layer
            .objects
            .iter()
            .zip(placements.objects)
            .zip(fills.targets)
        {
            let location = SourcePageLocation {
                part: layer.part.clone(),
                object: Some(*id),
            };
            if placed.native_id != *id
                || fill.target != (FillTarget::TableBackground { native_id: *id })
            {
                return Err(SourcePageError::Invalid("table page query identity"));
            }
            let SourcePlacementOutcome::Resolved { placement } = placed.outcome else {
                let SourcePlacementOutcome::Unresolved { reason } = placed.outcome else {
                    unreachable!()
                };
                return Err(mapping(&location, SourcePageIssue::Placement { reason }));
            };
            output.push(Object {
                binding: SourcePagePaintBinding {
                    location,
                    drawing_surface: layer.part.clone(),
                    fill,
                    picture_fill: None,
                    line: None,
                    placement: Some(*placement),
                    region: None,
                    table_stroke: None,
                },
                geometry: None,
            });
        }
    }
    Ok(output)
}

fn fill_limits(queries: usize) -> FillColorLimits {
    FillColorLimits {
        fills: FillResolveLimits {
            max_queries: queries,
            ..Default::default()
        },
        colors: mo_presentation_source::source::color::ColorLimits {
            // A target can contribute many independent gradient color slots.
            // Final aggregate logical paint work is charged by PreparedPage.
            max_queries: mo_raster::MAX_GRADIENT_INPUT_STOPS,
            ..Default::default()
        },
    }
}

pub(super) fn rectangle(rect: Rect, ordinal: u32, error: Fixed) -> CompiledNativePath {
    let origin = GeometryOrigin::document(ordinal);
    let commands = vec![
        C::Move { to: rect.min },
        C::Line {
            to: Point {
                x: rect.max.x,
                y: rect.min.y,
            },
        },
        C::Line { to: rect.max },
        C::Line {
            to: Point {
                x: rect.min.x,
                y: rect.max.y,
            },
        },
        C::Close,
    ];
    CompiledNativePath {
        origin,
        source_map: vec![NativePathSpan {
            origin,
            first_command: 0,
            command_count: 5,
        }],
        commands,
        fill: None,
        stroke: Some(false),
        extrusion_ok: None,
        numeric_error_bound: Point { x: error, y: error },
        curve_error_bound: ZERO,
        coordinate_error_bound: Point { x: error, y: error },
        arc_segments: 0,
    }
}

#[allow(clippy::too_many_arguments)]
fn filled_receiver(
    binding: &SourcePagePaintBinding,
    ordinal: u32,
    images_enabled: bool,
    size: mo_presentation_model::Size,
    tolerance: Fixed,
    bounds: &mut BoundsBudget,
    check: &dyn Fn() -> bool,
) -> Result<PreparedPaint, SourcePageError> {
    let region = binding.region.as_ref().expect("native receiver region");
    let path = rectangle(region.bounds, ordinal, region.coordinate_error_bound);
    let mut paths = vec![path];
    let radial = gradient_circle::LayoutContext {
        page_size: size,
        paths: Some(&paths),
        tolerance,
        budget: bounds,
    }
    .layout(&binding.fill, &binding.location, check)?;
    let fill = paint::fill(
        &binding.fill,
        &binding.location,
        images_enabled,
        binding.placement.as_ref(),
        size,
        radial.as_ref(),
        Some(region),
        check,
    )?;
    if fill.is_some() {
        shifted(
            &mut paths[0].commands,
            binding.placement.as_ref().expect("placement").anchor,
            check,
        )?;
    } else {
        paths.clear();
    }
    Ok(PreparedPaint {
        binding: Some(binding.clone()),
        paths,
        fill,
        picture_fill: None,
        line: None,
    })
}

#[allow(clippy::too_many_arguments)]
pub(super) fn prepare(
    source: &mut SourcePreparation<'_>,
    q: &SourcePageRequest,
    binding: &mut SourcePagePaintBinding,
    images_enabled: bool,
    tolerance: Fixed,
    bounds: &mut BoundsBudget,
    geometry: &DeclaredTableGeometry<'_>,
    check: &dyn Fn() -> bool,
) -> Result<Vec<PreparedPaint>, SourcePageError> {
    let native_id = binding.location.object.expect("table object");
    let index = source.index();
    let grid = geometry.grid();
    let table = grid.table();
    let columns = geometry.columns();
    let rows = geometry.rows();
    let x0 = columns.first().expect("table columns");
    let x1 = columns.last().expect("table columns");
    let y0 = rows.first().expect("table rows");
    let y1 = rows.last().expect("table rows");
    let error = [x0, x1, y0, y1]
        .iter()
        .map(|v| v.conversion_error_bound)
        .max()
        .unwrap();
    binding.region = Some(SourcePaintRegion {
        bounds: Rect {
            min: Point {
                x: x0.value.min(x1.value),
                y: y0.value,
            },
            max: Point {
                x: x0.value.max(x1.value),
                y: y1.value,
            },
        },
        coordinate_error_bound: error,
    });
    let size = index
        .page_size
        .ok_or(SourcePageError::Invalid("table page size"))?;
    let mut background = filled_receiver(
        binding,
        table.source_ordinal,
        images_enabled,
        size,
        tolerance,
        bounds,
        check,
    )?;
    background.binding = None;
    let mut paints = vec![background];
    let regions = grid.regions();
    let fills = mo_presentation_source::source::fill::colors::query_in_preparation(
        source,
        &SourceFillColorQuery {
            expected_source_sha256: q.expected_source_sha256.clone(),
            surface: binding.location.part.clone(),
            targets: regions
                .iter()
                .map(|r| FillTarget::TableCell {
                    native_id,
                    cell: r.origin,
                })
                .collect(),
            fill_profile: FillProfile::Drawingml2024DraftV1,
            color_profile: ColorProfile::Ecma3762016DraftV1,
            context: q.color_context.clone(),
        },
        &binding.location.part,
        &q.slide,
        fill_limits(regions.len()),
        check,
    )?;
    if fills.targets.len() != regions.len() {
        return Err(SourcePageError::Invalid("table cell fill cardinality"));
    }
    for (region, fill) in regions.iter().zip(fills.targets) {
        cancel(check)?;
        let cell = geometry.cell(region.origin).expect("compiled native cell");
        if fill.target
            != (FillTarget::TableCell {
                native_id,
                cell: region.origin,
            })
        {
            return Err(SourcePageError::Invalid("table cell fill identity"));
        }
        let mut cell_binding = binding.clone();
        cell_binding.fill = fill;
        cell_binding.region = Some(SourcePaintRegion {
            bounds: cell.merged,
            coordinate_error_bound: cell.conversion_error_bound,
        });
        paints.push(filled_receiver(
            &cell_binding,
            cell.source_ordinal,
            images_enabled,
            size,
            tolerance,
            bounds,
            check,
        )?);
    }
    paints.extend(borders::prepare(source, q, binding, geometry, check)?);
    Ok(paints)
}
