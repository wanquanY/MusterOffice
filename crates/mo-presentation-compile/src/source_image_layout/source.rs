//! Bind the native local image calculation to source target identity and the
//! opaque decoded results. Page composition consumes the placement separately.
use super::*;
use crate::source_placement::*;
use mo_image::DecodedImage;
use mo_pptx::source::{SourceIndex, fill::resolve::FillTarget, images::*};
use schemars::JsonSchema;
use serde::Serialize;
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ImageSourceLayoutPlan {
    pub target: FillTarget,
    pub resource: u32,
    /// None is a surface background. This is the shape placement, not yet the
    /// image paint placement when rotateWithShape is false.
    pub placement: Option<NativePlacement>,
    pub layout: NativeImageLayout,
}
#[derive(Debug, thiserror::Error)]
pub enum ImageSourceLayoutError {
    #[error("source image layout binding: {0}")]
    Invalid(&'static str),
    #[error("source image layout limit: {0}")]
    Limit(&'static str),
    #[error("source image layout unresolved target {target:?}: {outcome:?}")]
    Unresolved {
        target: FillTarget,
        outcome: Box<SourceImageOutcome>,
    },
    #[error("source image layout at {target:?}: {error}")]
    At {
        target: FillTarget,
        error: Box<ImageSourceLayoutError>,
    },
    #[error("source image layout placement: {0:?}")]
    PlacementRequired(Box<PlacementUnresolved>),
    #[error(transparent)]
    Placement(#[from] SourcePlacementError),
    #[error(transparent)]
    Layout(#[from] ImageLayoutError),
}
/// `catalog` and `index` are computed native source records from the same
/// package, not untrusted deserialized host replacements. DecodedImage can only
/// be constructed by the validated decoder; each resource digest is rebound.
/// All targets must resolve. No partial target list escapes on any failure.
pub fn layout_source(
    index: &SourceIndex,
    catalog: &SourceImageResources,
    decoded: &[DecodedImage],
    check: &dyn Fn() -> bool,
) -> Result<Vec<ImageSourceLayoutPlan>, ImageSourceLayoutError> {
    use ImageSourceLayoutError as E;
    cancel(check)?;
    if catalog.source_sha256 != index.source_sha256
        || !index.surfaces.contains_key(&catalog.surface)
    {
        return Err(E::Invalid("source digest or surface"));
    }
    if catalog.targets.len() > 4096 || catalog.resources.len() > 4096 {
        return Err(E::Limit("targets or resources"));
    }
    if catalog.resources.len() != decoded.len() {
        return Err(E::Invalid("decoded cardinality"));
    }
    let mut bytes = 0usize;
    for (resource, pixels) in catalog.resources.iter().zip(decoded) {
        cancel(check)?;
        if resource.sha256 != pixels.info().source_sha256 {
            return Err(E::Invalid("decoded source digest"));
        }
        bytes = bytes
            .checked_add(pixels.pixels().len())
            .filter(|n| *n <= mo_image::MAX_PIXEL_BYTES)
            .ok_or(E::Limit("decoded resources"))?;
    }
    let mut ids = BTreeSet::new();
    for item in &catalog.targets {
        cancel(check)?;
        if !matches!(item.outcome, SourceImageOutcome::Available { .. }) {
            return Err(E::Unresolved {
                target: item.target.clone(),
                outcome: Box::new(item.outcome.clone()),
            });
        }
        match item.target {
            FillTarget::Object { native_id }
            | FillTarget::Picture { native_id }
            | FillTarget::Line { native_id } => {
                ids.insert(native_id);
            }
            FillTarget::Background {} => (),
            FillTarget::RootGroup {} => {
                return Err(E::Invalid("root group has no standalone image paint box"));
            }
        }
    }
    let placements = if ids.is_empty() {
        BTreeMap::new()
    } else {
        source_placements(
            index,
            &SourcePlacementQuery {
                expected_source_sha256: index.source_sha256.clone(),
                surface: catalog.surface.clone(),
                objects: ids.iter().copied().collect(),
                profile: SourcePlacementProfile::DrawingmlSourceDraftV1,
            },
            SourcePlacementLimits {
                max_queries: 4096,
                ..Default::default()
            },
            check,
        )?
        .objects
        .into_iter()
        .map(|p| (p.native_id, p.outcome))
        .collect()
    };
    if placements.len() != ids.len() {
        return Err(E::Invalid("placement cardinality"));
    }
    let mut plans = Vec::with_capacity(catalog.targets.len());
    for item in &catalog.targets {
        cancel(check)?;
        let SourceImageOutcome::Available { resource, binding } = &item.outcome else {
            unreachable!("preflight resolved targets");
        };
        let result = (|| {
            let image = decoded
                .get(*resource as usize)
                .ok_or(E::Invalid("resource index"))?;
            let placement = match item.target {
                FillTarget::Object { native_id }
                | FillTarget::Picture { native_id }
                | FillTarget::Line { native_id } => {
                    match placements
                        .get(&native_id)
                        .ok_or(E::Invalid("placement identity"))?
                    {
                        SourcePlacementOutcome::Resolved { placement } => {
                            Some(placement.as_ref().clone())
                        }
                        SourcePlacementOutcome::Unresolved { reason } => {
                            return Err(E::PlacementRequired(Box::new(reason.clone())));
                        }
                    }
                }
                FillTarget::Background {} => None,
                FillTarget::RootGroup {} => unreachable!("root preflight"),
            };
            let size = match &placement {
                Some(p) => p.source_size,
                None => index.page_size.ok_or(E::Invalid("background page size"))?,
            };
            let layout = layout(&binding.image, image.info(), size, check)?;
            Ok(ImageSourceLayoutPlan {
                target: item.target.clone(),
                resource: *resource,
                placement,
                layout,
            })
        })();
        plans.push(result.map_err(|error| E::At {
            target: item.target.clone(),
            error: Box::new(error),
        })?);
    }
    cancel(check)?;
    Ok(plans)
}
