//! Chart geometry uses the same device-coordinate budget as ordinary paths.
use super::*;
use crate::source_placement::*;

pub(super) fn for_layer(
    index: &SourceIndex,
    q: &SourcePageRequest,
    layer: &SourcePageLayer,
    properties: Option<&SourceProperties>,
    bindings: &BTreeMap<(String, u32), SourceChartBinding>,
    check: &dyn Fn() -> bool,
) -> Result<BTreeMap<u32, Fixed>, SourcePageError> {
    let objects: Vec<_> = layer
        .objects
        .iter()
        .copied()
        .filter(|id| bindings.contains_key(&(layer.part.clone(), *id)))
        .collect();
    if objects.is_empty() {
        return Ok(BTreeMap::new());
    }
    let placements = source_placements_sampled(
        index,
        &SourcePlacementQuery {
            expected_source_sha256: q.expected_source_sha256.clone(),
            surface: layer.part.clone(),
            objects,
            profile: SourcePlacementProfile::DrawingmlSourceDraftV1,
        },
        Default::default(),
        properties,
        check,
    )?;
    placements
        .objects
        .into_iter()
        .map(|placed| {
            let placement = match placed.outcome {
                SourcePlacementOutcome::Resolved { placement } => placement,
                SourcePlacementOutcome::Unresolved { reason } => {
                    return Err(SourcePageError::Mapping {
                        location: SourcePageLocation {
                            part: layer.part.clone(),
                            object: Some(placed.native_id),
                        },
                        issue: Box::new(SourcePageIssue::Placement { reason }),
                    });
                }
            };
            let tolerance = crate::coordinate_budget::local_tolerance(
                std::iter::once((&placement.affine, &placement.uncertainty)),
                &q.viewport,
                check,
            )?;
            Ok((placed.native_id, tolerance))
        })
        .collect()
}
