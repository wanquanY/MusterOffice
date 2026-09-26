//! Provenance-preserving line property resolution over an immutable source index.
mod budget;
mod merge;
mod types;
use crate::{PptxError, source::*, value};
use budget::Budget;
use std::collections::BTreeMap;
pub use types::*;

enum Failure {
    Unresolved(LineUnresolved),
    Abort(PptxError),
}
impl From<PptxError> for Failure {
    fn from(v: PptxError) -> Self {
        Self::Abort(v)
    }
}
impl From<LineUnresolved> for Failure {
    fn from(v: LineUnresolved) -> Self {
        Self::Unresolved(v)
    }
}

struct Resolver<'a> {
    index: &'a SourceIndex,
    surface: &'a SourceSurface,
    objects: BTreeMap<(&'a str, u32), &'a SourceObject>,
}
impl Resolver<'_> {
    fn object(&self, reference: &SourceObjectRef) -> Result<&SourceObject, PptxError> {
        self.objects
            .get(&(reference.part.as_str(), reference.native_id))
            .copied()
            .ok_or_else(|| {
                PptxError::SourceConflict(
                    "line object binding is not in the inspected source".into(),
                )
            })
    }
    fn resolve(
        &self,
        target: SourceObjectRef,
        budget: &mut Budget<'_>,
    ) -> Result<EffectiveLine, Failure> {
        let mut current = target;
        let mut partial = merge::Partial::default();
        let mut placeholder = None;
        let mut placeholder_seen = false;
        // Each parent hop must move slide -> layout -> master. A mutated public
        // index cannot turn the bounded native hierarchy into a cycle.
        for _ in 0..3 {
            budget.step()?;
            let object = self.object(&current)?;
            if !matches!(
                object.kind,
                SourceObjectKind::Shape | SourceObjectKind::Picture | SourceObjectKind::Connector
            ) {
                return Err(LineUnresolved::UnsupportedObject { object: current }.into());
            }
            budget.bytes(
                current
                    .part
                    .len()
                    .checked_mul(64)
                    .ok_or(PptxError::Limit("line origin bytes"))?,
            )?;
            let reference = object.line_reference.as_ref();
            let local_placeholder = if let Some(reference) = reference {
                budget.step()?;
                let origin = LineOrigin::Object {
                    object: current.clone(),
                    source_ordinal: reference.source_ordinal,
                };
                if !reference.retained_ordinals.is_empty() {
                    return Err(LineUnresolved::RetainedContent {
                        origin: origin.at(reference.retained_ordinals[0]),
                    }
                    .into());
                }
                if let Some(color) = &reference.color {
                    budget.color(color)?;
                    Some(merge::color(color, &origin))
                } else {
                    None
                }
            } else {
                None
            };
            if !placeholder_seen && reference.is_some() {
                placeholder = local_placeholder.clone();
                placeholder_seen = true;
            }
            if let Some(line) = &object.line {
                budget.line(line)?;
                partial.merge(
                    line,
                    &LineOrigin::Object {
                        object: current.clone(),
                        source_ordinal: line.source_ordinal,
                    },
                    local_placeholder.as_ref(),
                )?;
            }
            if partial.complete() {
                return Ok(partial.finish(placeholder));
            }
            // Zero denotes no style lookup in this explicit draft profile. It
            // is not an invented noFill and does not erase parent declarations.
            if let Some(reference) = reference
                && reference.index != 0
            {
                let binding = self
                    .surface
                    .theme_selection
                    .format
                    .as_ref()
                    .ok_or_else(|| LineUnresolved::MissingFormatScheme {
                        object: current.clone(),
                    })?;
                let scheme = self
                    .index
                    .themes
                    .get(&binding.part)
                    .and_then(|t| t.format_scheme.as_ref())
                    .filter(|s| s.source_ordinal == binding.source_ordinal)
                    .ok_or_else(|| {
                        PptxError::SourceConflict(
                            "line format binding differs from inspected source".into(),
                        )
                    })?;
                let entry = scheme
                    .lines
                    .get((reference.index - 1) as usize)
                    .ok_or_else(|| LineUnresolved::StyleIndexOutOfRange {
                        object: current.clone(),
                        index: reference.index,
                        available: scheme.lines.len() as u32,
                    })?;
                let line = entry
                    .line
                    .as_ref()
                    .filter(|l| l.source_ordinal == entry.source_ordinal)
                    .ok_or_else(|| {
                        PptxError::SourceConflict(
                            "line style binding differs from inspected source".into(),
                        )
                    })?;
                budget.bytes(
                    binding
                        .part
                        .len()
                        .checked_mul(64)
                        .ok_or(PptxError::Limit("line origin bytes"))?,
                )?;
                budget.line(line)?;
                partial.merge(
                    line,
                    &LineOrigin::Theme {
                        part: binding.part.clone(),
                        source_ordinal: entry.source_ordinal,
                        via: current.clone(),
                        reference_ordinal: reference.source_ordinal,
                        style_index: reference.index,
                    },
                    local_placeholder.as_ref(),
                )?;
            }
            if partial.complete() {
                return Ok(partial.finish(placeholder));
            }
            match &object.resolution.placeholder_match {
                SourcePlaceholderMatch::NotPlaceholder
                | SourcePlaceholderMatch::Master
                | SourcePlaceholderMatch::Detached => return Ok(partial.finish(placeholder)),
                SourcePlaceholderMatch::Matched { target, .. } => {
                    let child = self.index.surfaces.get(&current.part).map(|s| s.kind);
                    let parent = self.index.surfaces.get(&target.part).map(|s| s.kind);
                    if !matches!(
                        (child, parent),
                        (Some(SurfaceKind::Slide), Some(SurfaceKind::Layout))
                            | (Some(SurfaceKind::Layout), Some(SurfaceKind::Master))
                    ) {
                        return Err(PptxError::SourceConflict(
                            "line inheritance does not follow the surface hierarchy".into(),
                        )
                        .into());
                    }
                    current = target.clone();
                }
                matching => {
                    return Err(LineUnresolved::Placeholder {
                        object: current,
                        matching: matching.clone(),
                    }
                    .into());
                }
            }
        }
        Err(PptxError::SourceConflict("line inheritance exceeds native hierarchy".into()).into())
    }
}

/// Native/WASM hosts inspect actual source bytes before invoking this pure query.
/// Returned styles are derived; source author records and the package are untouched.
pub fn query(
    index: &SourceIndex,
    request: &SourceLineQuery,
    limits: LineResolveLimits,
    check: &dyn Fn() -> bool,
) -> Result<SourceLineStyles, PptxError> {
    query_in_context(index, request, &request.surface, limits, check)
}

/// Resolve declarations on request.surface using the explicitly selected drawing
/// surface's theme, color map and background. Original owners are never rewritten.
/// Page composition validates that these surfaces belong to one source hierarchy.
pub fn query_in_context(
    index: &SourceIndex,
    request: &SourceLineQuery,
    drawing_surface: &str,
    limits: LineResolveLimits,
    check: &dyn Fn() -> bool,
) -> Result<SourceLineStyles, PptxError> {
    crate::cancelled(check)?;
    if index.source_sha256 != request.expected_source_sha256 {
        return Err(PptxError::SourceConflict(
            "line query source digest differs".into(),
        ));
    }
    if request.objects.len() > limits.max_queries {
        return Err(PptxError::Limit("line queries"));
    }
    if !index.surfaces.contains_key(&request.surface) {
        return Err(value(
            "lineQuery.surface",
            "surface is not in the inspected source",
        ));
    }
    let surface = index.surfaces.get(drawing_surface).ok_or_else(|| {
        value(
            "lineQuery.surface",
            "surface is not in the inspected source",
        )
    })?;
    let mut budget = Budget {
        limits,
        check,
        steps: 0,
        values: 0,
        lexical_bytes: 0,
    };
    let mut objects = BTreeMap::new();
    for (part, surface) in &index.surfaces {
        for object in &surface.objects {
            budget.step()?;
            objects.insert((part.as_str(), object.native_id), object);
        }
    }
    let resolver = Resolver {
        index,
        surface,
        objects,
    };
    let mut results = Vec::with_capacity(request.objects.len());
    for id in &request.objects {
        let target = SourceObjectRef {
            part: request.surface.clone(),
            native_id: *id,
        };
        if !resolver
            .objects
            .contains_key(&(request.surface.as_str(), *id))
        {
            return Err(value(
                "lineQuery.objects",
                "object is not in the requested surface",
            ));
        }
        let outcome = match resolver.resolve(target, &mut budget) {
            Ok(line) => LineOutcome::Resolved {
                line: Box::new(line),
            },
            Err(Failure::Unresolved(reason)) => LineOutcome::Unresolved { reason },
            Err(Failure::Abort(error)) => return Err(error),
        };
        results.push(SourceLineResult {
            native_id: *id,
            outcome,
        });
    }
    Ok(SourceLineStyles {
        source_sha256: index.source_sha256.clone(),
        surface: request.surface.clone(),
        profile: request.profile,
        objects: results,
    })
}
