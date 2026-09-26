//! Bind local radial geometry to the immutable source and receiving shape.
use super::*;
use crate::source_placement::*;
use mo_pptx::source::{
    SourceIndex,
    fill::resolve::*,
    geometry::evaluate::{
        GeometryLimits, GeometryOutcome, GeometryProfile, GeometryUnresolved, SourceGeometryQuery,
    },
};
use schemars::JsonSchema;
use serde::Serialize;
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SourceRadialLayoutPlan {
    pub source_sha256: mo_common::Digest,
    pub surface: String,
    pub target: FillTarget,
    /// Original inheritance/provenance remains available to later painting.
    pub style: FillOutcome,
    /// Local geometry has not yet applied this placement or stationary policy.
    pub placement: Option<NativePlacement>,
    pub layout: NativeRadialLayout,
}
#[derive(Debug, thiserror::Error)]
pub enum SourceRadialLayoutError {
    #[error("source radial digest conflict")]
    SourceConflict,
    #[error("source radial layout: {0}")]
    Invalid(&'static str),
    #[error("source radial layout limit: {0}")]
    Limit(&'static str),
    #[error("source radial layout at {target:?}: {error}")]
    At {
        target: FillTarget,
        error: Box<SourceRadialLayoutError>,
    },
    #[error("source radial fill unresolved: {0:?}")]
    FillRequired(Box<FillOutcome>),
    #[error("source radial geometry unresolved: {0:?}")]
    GeometryRequired(Box<GeometryUnresolved>),
    #[error("source radial placement unresolved: {0:?}")]
    PlacementRequired(Box<PlacementUnresolved>),
    #[error(transparent)]
    Source(#[from] mo_pptx::PptxError),
    #[error(transparent)]
    Placement(#[from] SourcePlacementError),
    #[error(transparent)]
    Layout(#[from] RadialLayoutError),
}
/// Atomic native library query. No file I/O, decoder, shaper or renderer calls.
/// This is local circle geometry, not a final page or an Office acceptance claim.
pub fn layout_source(
    index: &SourceIndex,
    query: &SourceFillQuery,
    options: RadialLayoutOptions,
    limits: RadialLayoutLimits,
    check: &dyn Fn() -> bool,
) -> Result<Vec<SourceRadialLayoutPlan>, SourceRadialLayoutError> {
    layout_source_with_basis(
        index,
        query,
        options,
        limits,
        CircleFocusBasis::CircumscribedSquare,
        check,
    )
}
pub fn layout_source_with_basis(
    index: &SourceIndex,
    query: &SourceFillQuery,
    options: RadialLayoutOptions,
    limits: RadialLayoutLimits,
    basis: CircleFocusBasis,
    check: &dyn Fn() -> bool,
) -> Result<Vec<SourceRadialLayoutPlan>, SourceRadialLayoutError> {
    use SourceRadialLayoutError as E;
    cancel(check)?;
    if index.source_sha256 != query.expected_source_sha256 {
        return Err(E::SourceConflict);
    }
    if !index.surfaces.contains_key(&query.surface) {
        return Err(E::Invalid("source surface"));
    }
    if query.targets.len() > limits.max_targets {
        return Err(E::Limit("targets"));
    }
    if options.coordinate_tolerance.raw() < 1024 {
        return Err(E::Invalid("local tolerance below 1024 Q32 units"));
    }
    let mut seen = BTreeSet::new();
    let mut ids = Vec::new();
    for target in &query.targets {
        cancel(check)?;
        if !seen.insert(target.clone()) {
            return Err(E::Invalid("duplicate target"));
        }
        match target {
            FillTarget::Object { native_id } => ids.push(*native_id),
            FillTarget::Background {} => (),
            _ => {
                return Err(E::Invalid(
                    "standalone object or background target required",
                ));
            }
        }
    }
    let styles = mo_pptx::source::fill::resolve::query(
        index,
        query,
        FillResolveLimits {
            max_queries: limits.max_targets,
            ..Default::default()
        },
        check,
    )?;
    let (mut geometries, mut placements) = if ids.is_empty() {
        (BTreeMap::new(), BTreeMap::new())
    } else {
        let geometry = mo_pptx::source::geometry::evaluate::query(
            index,
            &SourceGeometryQuery {
                expected_source_sha256: query.expected_source_sha256.clone(),
                surface: query.surface.clone(),
                objects: ids.clone(),
                profile: GeometryProfile::Drawingml2016PresetsDraftV2,
            },
            GeometryLimits {
                max_queries: limits.max_targets,
                ..Default::default()
            },
            check,
        )?;
        let placements = source_placements(
            index,
            &SourcePlacementQuery {
                expected_source_sha256: query.expected_source_sha256.clone(),
                surface: query.surface.clone(),
                objects: ids.clone(),
                profile: SourcePlacementProfile::DrawingmlSourceDraftV1,
            },
            SourcePlacementLimits {
                max_queries: limits.max_targets,
                ..Default::default()
            },
            check,
        )?;
        (
            geometry
                .objects
                .into_iter()
                .map(|v| (v.native_id, v.outcome))
                .collect(),
            placements
                .objects
                .into_iter()
                .map(|v| (v.native_id, v.outcome))
                .collect(),
        )
    };
    if styles.targets.len() != query.targets.len()
        || geometries.len() != ids.len()
        || placements.len() != ids.len()
    {
        return Err(E::Invalid("query cardinality"));
    }
    let quarter = Fixed::from_raw(options.coordinate_tolerance.raw() / 4);
    let mut compiler = NativePathCompiler::new(
        NativePathOptions {
            profile: NativePathProfile::DrawingmlPolarArcsDraftV1,
            coordinate_tolerance: quarter,
        },
        limits.paths,
        check,
    )
    .map_err(RadialLayoutError::from)?;
    let mut budget = BoundsBudget::new(limits.max_bounds_steps);
    let mut plans = Vec::with_capacity(styles.targets.len());
    for (requested, item) in query.targets.iter().zip(styles.targets) {
        cancel(check)?;
        if requested != &item.target {
            return Err(E::Invalid("fill target binding"));
        }
        let mut compute = || -> Result<_, E> {
            let FillOutcome::Resolved { fill, redirects } = &item.outcome else {
                return Err(E::FillRequired(Box::new(item.outcome.clone())));
            };
            let EffectiveFill::Gradient { gradient, .. } = fill.as_ref() else {
                return Err(E::Invalid("resolved gradient required"));
            };
            if redirects
                .iter()
                .any(|r| matches!(r.target.target, FillTarget::Background {}))
                && matches!(requested, FillTarget::Object { .. })
            {
                return Err(E::Invalid(
                    "background redirect requires page background space",
                ));
            }
            let (layout, placement) = match *requested {
                FillTarget::Background {} => {
                    let size = index.page_size.ok_or(E::Invalid("source page extent"))?;
                    (
                        layout_background_with_basis(gradient, size, basis, check)?,
                        None,
                    )
                }
                FillTarget::Object { native_id } => {
                    let geometry = geometries
                        .remove(&native_id)
                        .ok_or(E::Invalid("geometry binding"))?;
                    let GeometryOutcome::Resolved { geometry } = geometry else {
                        let GeometryOutcome::Unresolved { reason } = geometry else {
                            unreachable!()
                        };
                        return Err(E::GeometryRequired(Box::new(reason)));
                    };
                    let placement = placements
                        .remove(&native_id)
                        .ok_or(E::Invalid("placement binding"))?;
                    let SourcePlacementOutcome::Resolved { placement } = placement else {
                        let SourcePlacementOutcome::Unresolved { reason } = placement else {
                            unreachable!()
                        };
                        return Err(E::PlacementRequired(Box::new(reason)));
                    };
                    if geometry.extent.value != placement.source_size {
                        return Err(E::Invalid("geometry/placement extent binding"));
                    }
                    let paths = compiler
                        .compile(&geometry)
                        .map_err(RadialLayoutError::from)?;
                    (
                        layout_paths_with_basis(
                            gradient,
                            &paths,
                            RadialLayoutOptions {
                                coordinate_tolerance: quarter,
                            },
                            &mut budget,
                            basis,
                            check,
                        )?,
                        Some(*placement),
                    )
                }
                _ => unreachable!("preflight targets"),
            };
            Ok(SourceRadialLayoutPlan {
                source_sha256: index.source_sha256.clone(),
                surface: query.surface.clone(),
                target: item.target.clone(),
                style: item.outcome.clone(),
                placement,
                layout,
            })
        };
        plans.push(compute().map_err(|error| E::At {
            target: item.target,
            error: Box::new(error),
        })?);
    }
    cancel(check)?;
    Ok(plans)
}
