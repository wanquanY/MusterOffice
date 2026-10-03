use super::*;
pub(crate) struct Object {
    pub(crate) binding: SourcePagePaintBinding,
    pub(super) geometry: Option<Box<EvaluatedGeometry>>,
}
fn drawings(
    index: &SourceIndex,
    q: &SourcePageRequest,
    layers: &[SourcePageLayer],
    transforms: Option<&crate::source_placement::SourceProperties>,
    check: &dyn Fn() -> bool,
) -> Result<Vec<Object>, SourcePageError> {
    let mut objects = vec![];
    for layer in layers {
        cancel(check)?;
        let ids = &layer.objects;
        if ids.is_empty() {
            continue;
        }
        let placements = crate::source_placement::source_placements_sampled(
            index,
            &SourcePlacementQuery {
                expected_source_sha256: q.expected_source_sha256.clone(),
                surface: layer.part.clone(),
                objects: ids.clone(),
                profile: SourcePlacementProfile::DrawingmlSourceDraftV1,
            },
            SourcePlacementLimits {
                max_queries: 8192,
                ..Default::default()
            },
            transforms,
            check,
        )?;
        let geometry = mo_presentation_source::source::geometry::evaluate::query(
            index,
            &SourceGeometryQuery {
                expected_source_sha256: q.expected_source_sha256.clone(),
                surface: layer.part.clone(),
                objects: ids.clone(),
                profile: GeometryProfile::Drawingml2016PresetsDraftV2,
            },
            GeometryLimits {
                max_queries: 8192,
                ..Default::default()
            },
            check,
        )?;
        let picture_ids: std::collections::BTreeSet<u32> = index.surfaces[&layer.part]
            .objects
            .iter()
            .filter(|o| o.kind == mo_presentation_source::source::SourceObjectKind::Picture)
            .map(|o| o.native_id)
            .collect();
        let fills = mo_presentation_source::source::fill::colors::query_on_page(
            index,
            &SourceFillColorQuery {
                expected_source_sha256: q.expected_source_sha256.clone(),
                surface: layer.part.clone(),
                targets: ids
                    .iter()
                    .map(|id| FillTarget::Object { native_id: *id })
                    .chain(
                        ids.iter()
                            .filter(|id| picture_ids.contains(id))
                            .map(|id| FillTarget::Picture { native_id: *id }),
                    )
                    .collect(),
                fill_profile: FillProfile::Drawingml2024DraftV1,
                color_profile: ColorProfile::Ecma3762016DraftV1,
                context: q.color_context.clone(),
            },
            &layer.part,
            &q.slide,
            FillColorLimits {
                fills: FillResolveLimits {
                    max_queries: 16384,
                    ..Default::default()
                },
                ..Default::default()
            },
            check,
        )?;
        let lines = mo_presentation_source::source::line::colors::query_in_context(
            index,
            &SourceLineColorQuery {
                expected_source_sha256: q.expected_source_sha256.clone(),
                surface: layer.part.clone(),
                objects: ids.clone(),
                line_profile: LineProfile::Drawingml2024DraftV1,
                color_profile: ColorProfile::Ecma3762016DraftV1,
                context: q.color_context.clone(),
            },
            &layer.part,
            LineColorLimits {
                lines: LineResolveLimits {
                    max_queries: 16384,
                    ..Default::default()
                },
                colors: mo_presentation_source::source::color::ColorLimits {
                    max_queries: 16384,
                    ..Default::default()
                },
            },
            check,
        )?;
        let mut fills: std::collections::BTreeMap<_, _> = fills
            .targets
            .into_iter()
            .map(|fill| (fill.target.clone(), fill))
            .collect();
        if [
            placements.objects.len(),
            geometry.objects.len(),
            lines.objects.len(),
        ]
        .iter()
        .any(|n| *n != ids.len())
        {
            return Err(SourcePageError::Invalid("source query cardinality"));
        }
        for (((id, p), g), line) in ids
            .iter()
            .zip(placements.objects)
            .zip(geometry.objects)
            .zip(lines.objects)
        {
            cancel(check)?;
            let location = SourcePageLocation {
                part: layer.part.clone(),
                object: Some(*id),
            };
            let fill = fills
                .remove(&FillTarget::Object { native_id: *id })
                .ok_or(SourcePageError::Invalid("source fill cardinality"))?;
            let picture_fill = if picture_ids.contains(id) {
                Some(
                    fills
                        .remove(&FillTarget::Picture { native_id: *id })
                        .ok_or(SourcePageError::Invalid("source picture fill cardinality"))?,
                )
            } else {
                None
            };
            if p.native_id != *id
                || g.native_id != *id
                || line.native_id != *id
                || fill.target != (FillTarget::Object { native_id: *id })
            {
                return Err(SourcePageError::Invalid("source query object binding"));
            }
            let SourcePlacementOutcome::Resolved { placement } = p.outcome else {
                let SourcePlacementOutcome::Unresolved { reason } = p.outcome else {
                    unreachable!()
                };
                return Err(mapping(&location, SourcePageIssue::Placement { reason }));
            };
            let GeometryOutcome::Resolved { geometry } = g.outcome else {
                let GeometryOutcome::Unresolved { reason } = g.outcome else {
                    unreachable!()
                };
                return Err(mapping(&location, SourcePageIssue::Geometry { reason }));
            };
            objects.push(Object {
                binding: SourcePagePaintBinding {
                    location,
                    drawing_surface: layer.part.clone(),
                    fill,
                    picture_fill,
                    line: Some(line),
                    placement: Some(*placement),
                    region: None,
                    table_stroke: None,
                },
                geometry: Some(geometry),
            });
        }
        if !fills.is_empty() {
            return Err(SourcePageError::Invalid("unbound source fills"));
        }
    }
    Ok(objects)
}

#[allow(clippy::too_many_arguments)]
pub(super) fn objects(
    index: &SourceIndex,
    q: &SourcePageRequest,
    layers: &[SourcePageLayer],
    transforms: Option<&crate::source_placement::SourceProperties>,
    preparation: Option<&mut mo_presentation_source::source::prepared::SourcePreparation<'_>>,
    charts: &crate::source_chart_page::Charts,
    check: &dyn Fn() -> bool,
) -> Result<Vec<Object>, SourcePageError> {
    use std::collections::{BTreeMap, BTreeSet};
    let mut ordinary = layers.to_vec();
    let mut tables = layers.to_vec();
    for (i, layer) in layers.iter().enumerate() {
        cancel(check)?;
        let ids: BTreeSet<_> = index.surfaces[&layer.part]
            .objects
            .iter()
            .filter(|o| o.table.is_some())
            .map(|o| o.native_id)
            .collect();
        ordinary[i]
            .objects
            .retain(|id| !ids.contains(id) && !charts.contains_key(&(layer.part.clone(), *id)));
        tables[i].objects.retain(|id| ids.contains(id));
    }
    let ordinary = drawings(index, q, &ordinary, transforms, check)?;
    let tables = if tables.iter().all(|l| l.objects.is_empty()) {
        Vec::new()
    } else {
        super::table::objects(
            index,
            q,
            &tables,
            transforms,
            preparation.ok_or(SourcePageError::Invalid("missing table preparation"))?,
            check,
        )?
    };
    let mut by_id = BTreeMap::new();
    for object in ordinary
        .into_iter()
        .chain(tables)
        .chain(chart_objects(index, q, layers, transforms, charts, check)?)
    {
        let key = (
            object.binding.location.part.clone(),
            object.binding.location.object,
        );
        if by_id.insert(key, object).is_some() {
            return Err(SourcePageError::Invalid("duplicate page object"));
        }
    }
    let mut output = Vec::with_capacity(by_id.len());
    for layer in layers {
        for id in &layer.objects {
            cancel(check)?;
            output.push(
                by_id
                    .remove(&(layer.part.clone(), Some(*id)))
                    .ok_or(SourcePageError::Invalid("page object ordering"))?,
            );
        }
    }
    if !by_id.is_empty() {
        return Err(SourcePageError::Invalid("unbound page objects"));
    }
    Ok(output)
}

fn chart_objects(
    index: &SourceIndex,
    q: &SourcePageRequest,
    layers: &[SourcePageLayer],
    transforms: Option<&SourceProperties>,
    charts: &crate::source_chart_page::Charts,
    check: &dyn Fn() -> bool,
) -> Result<Vec<Object>, SourcePageError> {
    let mut output = Vec::new();
    for layer in layers {
        let ids: Vec<_> = layer
            .objects
            .iter()
            .copied()
            .filter(|id| charts.contains_key(&(layer.part.clone(), *id)))
            .collect();
        if ids.is_empty() {
            continue;
        }
        let placements = source_placements_sampled(
            index,
            &SourcePlacementQuery {
                expected_source_sha256: q.expected_source_sha256.clone(),
                surface: layer.part.clone(),
                objects: ids,
                profile: SourcePlacementProfile::DrawingmlSourceDraftV1,
            },
            Default::default(),
            transforms,
            check,
        )?;
        for placed in placements.objects {
            let location = SourcePageLocation {
                part: layer.part.clone(),
                object: Some(placed.native_id),
            };
            let placement = match placed.outcome {
                SourcePlacementOutcome::Resolved { placement } => *placement,
                SourcePlacementOutcome::Unresolved { reason } => {
                    return Err(mapping(&location, SourcePageIssue::Placement { reason }));
                }
            };
            output.push(Object {
                binding: SourcePagePaintBinding {
                    location,
                    drawing_surface: layer.part.clone(),
                    fill: SourceFillColorResult {
                        target: FillTarget::Object {
                            native_id: placed.native_id,
                        },
                        style: FillOutcome::Resolved {
                            fill: Box::new(EffectiveFill::None {
                                declared_by: FillOrigin::ProfileDefault {},
                            }),
                            redirects: vec![],
                        },
                        colors: FillPaintColors::None {},
                        context_override: None,
                    },
                    picture_fill: None,
                    line: None,
                    placement: Some(placement),
                    region: None,
                    table_stroke: None,
                },
                geometry: None,
            });
        }
    }
    Ok(output)
}
