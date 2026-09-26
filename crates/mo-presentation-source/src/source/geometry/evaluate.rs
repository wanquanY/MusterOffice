//! Native guide and coordinate evaluation, separate from source declarations,
//! path-space scaling, tessellation and page compilation.
mod builtin;
mod math;
mod presets;
mod records;
mod session;
mod types;
use super::*;
use crate::{PptxError, source::*, value};
use session::{Budget, Session};
use std::collections::BTreeMap;
pub use types::*;

enum Failure {
    Unresolved(GeometryUnresolved),
    Abort(PptxError),
}
impl From<GeometryUnresolved> for Failure {
    fn from(v: GeometryUnresolved) -> Self {
        Self::Unresolved(v)
    }
}
impl From<PptxError> for Failure {
    fn from(v: PptxError) -> Self {
        Self::Abort(v)
    }
}

fn evaluate(
    object: &SourceObject,
    budget: &mut Budget<'_>,
    catalog: &mut presets::Catalog,
) -> Result<EvaluatedGeometry, Failure> {
    budget.step()?;
    if !matches!(
        object.kind,
        SourceObjectKind::Shape | SourceObjectKind::Picture | SourceObjectKind::Connector
    ) {
        return Err(GeometryUnresolved::UnsupportedObject.into());
    }
    let g = object
        .geometry
        .as_ref()
        .ok_or(GeometryUnresolved::MissingDeclaration)?;
    if let Some(ordinal) = g.retained_ordinals.first() {
        return Err(GeometryUnresolved::RetainedContent {
            origin: GeometryOrigin::document(*ordinal),
        }
        .into());
    }
    let size = object
        .resolution
        .size
        .as_ref()
        .ok_or(GeometryUnresolved::MissingExtent)?;
    let w = size.value.width.get();
    let h = size.value.height.get();
    if !(0..=27_273_042_316_900).contains(&w) || !(0..=27_273_042_316_900).contains(&h) {
        return Err(GeometryUnresolved::InvalidExtent.into());
    }
    let (custom, preset, overrides) = match &g.definition {
        SourceGeometryDefinition::Custom(c) => (c.as_ref(), None, None),
        SourceGeometryDefinition::Preset {
            preset,
            adjustments,
        } => (
            catalog.get(*preset, budget)?,
            Some(*preset),
            adjustments.as_ref(),
        ),
    };
    let mut s = Session::new(w as f64, h as f64, budget);
    s.preset = preset;
    let mut adjustments = s.guides(custom.adjustments.as_ref(), true)?;
    if preset.is_some() {
        s.document_guides = true;
        adjustments.extend(s.guides(overrides, true)?);
        s.document_guides = false;
    }
    let guides = s.guides(custom.guides.as_ref(), false)?;
    let mut handles = Vec::new();
    if let Some(list) = &custom.handles {
        for h in &list.entries {
            handles.push(s.handle(h)?);
        }
    }
    let mut connections = Vec::new();
    if let Some(list) = &custom.connections {
        for c in &list.entries {
            connections.push(s.connection(c)?);
        }
    }
    let text_rect = custom.text_rect.as_ref().map(|r| s.rect(r)).transpose()?;
    let mut paths = Vec::new();
    for p in &custom.paths.entries {
        paths.push(s.path(p)?);
    }
    Ok(EvaluatedGeometry {
        source_ordinal: g.source_ordinal,
        extent: size.clone(),
        adjustments,
        guides,
        handles,
        connections,
        text_rect,
        paths,
    })
}

pub fn query(
    index: &SourceIndex,
    request: &SourceGeometryQuery,
    limits: GeometryLimits,
    check: &dyn Fn() -> bool,
) -> Result<SourceGeometryValues, PptxError> {
    crate::cancelled(check)?;
    if index.source_sha256 != request.expected_source_sha256 {
        return Err(PptxError::SourceConflict(
            "geometry query source digest differs".into(),
        ));
    }
    if request.objects.len() > limits.max_queries {
        return Err(PptxError::Limit("geometry queries"));
    }
    let surface = index.surfaces.get(&request.surface).ok_or_else(|| {
        value(
            "geometryQuery.surface",
            "surface is not in inspected source",
        )
    })?;
    let mut budget = Budget {
        limits,
        check,
        steps: 0,
        bytes: 0,
        values: 0,
    };
    budget.lexical(&request.surface)?;
    let mut lookup = BTreeMap::new();
    for object in &surface.objects {
        budget.step()?;
        lookup.insert(object.native_id, object);
    }
    let mut objects = Vec::new();
    let mut catalog = presets::Catalog::default();
    for id in &request.objects {
        budget.step()?;
        let object = lookup.get(id).ok_or_else(|| {
            value(
                "geometryQuery.objects",
                "object is not in requested surface",
            )
        })?;
        let outcome = match evaluate(object, &mut budget, &mut catalog) {
            Ok(geometry) => GeometryOutcome::Resolved {
                geometry: Box::new(geometry),
            },
            Err(Failure::Unresolved(reason)) => GeometryOutcome::Unresolved { reason },
            Err(Failure::Abort(error)) => return Err(error),
        };
        objects.push(GeometryResult {
            native_id: *id,
            outcome,
        });
    }
    Ok(SourceGeometryValues {
        source_sha256: index.source_sha256.clone(),
        surface: request.surface.clone(),
        profile: request.profile,
        objects,
    })
}
