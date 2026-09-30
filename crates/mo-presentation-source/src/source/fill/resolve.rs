//! Pure, provenance-preserving native fill inheritance over an immutable index.
mod budget;
mod merge;
mod table;
mod table_border;
mod types;
use super::*;
use crate::{PptxError, cancelled, source::*, value};
use budget::Budget;
pub(in crate::source) use budget::Budget as FillBudget;
pub(in crate::source) use merge::{Input as FillInput, Partial as FillPartial};
use std::collections::{BTreeMap, BTreeSet};
pub(in crate::source) use table::{
    BindIssue as TablePaintBindIssue, Binding as TablePaintBinding, Bindings as TablePaintBindings,
};
pub use types::*;

pub(in crate::source) enum Failure {
    Unresolved(FillUnresolved),
    Abort(PptxError),
}
impl From<PptxError> for Failure {
    fn from(v: PptxError) -> Self {
        Self::Abort(v)
    }
}
impl From<FillUnresolved> for Failure {
    fn from(v: FillUnresolved) -> Self {
        Self::Unresolved(v)
    }
}

struct Reference<'a> {
    ordinal: u32,
    index: u32,
    retained: &'a [u32],
    line: bool,
}
struct Resolver<'a> {
    index: &'a SourceIndex,
    background_part: &'a str,
    view: &'a SourceSurface,
    objects: crate::source::prepared::ObjectBindings<'a>,
    tables: table::Bindings<'a>,
}
fn conflict(message: &str) -> PptxError {
    PptxError::SourceConflict(message.into())
}
fn native_id(target: &FillTarget) -> Option<u32> {
    match target {
        FillTarget::Object { native_id }
        | FillTarget::Line { native_id }
        | FillTarget::Picture { native_id }
        | FillTarget::TableCell { native_id, .. }
        | FillTarget::TableCellBorder { native_id, .. }
        | FillTarget::TableBackground { native_id }
        | FillTarget::TableStyleFill { native_id, .. }
        | FillTarget::TableStyleBorder { native_id, .. } => Some(*native_id),
        _ => None,
    }
}
pub(in crate::source) fn line_input(fill: &line::SourceLineFill) -> (u32, merge::Input<'_>) {
    match fill {
        line::SourceLineFill::None { source_ordinal } => (*source_ordinal, merge::Input::None),
        line::SourceLineFill::Solid {
            source_ordinal,
            color,
        } => (*source_ordinal, merge::Input::Solid(color.as_ref())),
        line::SourceLineFill::Gradient {
            source_ordinal,
            gradient,
        } => (*source_ordinal, merge::Input::Gradient(gradient)),
        line::SourceLineFill::Pattern {
            source_ordinal,
            pattern,
        } => (*source_ordinal, merge::Input::Pattern(pattern)),
    }
}
impl Resolver<'_> {
    fn surface(&self, part: &str) -> Result<&SourceSurface, PptxError> {
        self.index
            .surfaces
            .get(part)
            .ok_or_else(|| conflict("fill surface binding is missing"))
    }
    fn object(&self, owner: &FillOwner) -> Result<&SourceObject, PptxError> {
        native_id(&owner.target)
            .and_then(|id| self.objects.get(&(owner.part.as_str(), id)))
            .ok_or_else(|| conflict("fill object binding is missing"))
    }
    fn declaration(
        &self,
        owner: &FillOwner,
        ordinal: u32,
        budget: &mut Budget<'_>,
    ) -> Result<FillOrigin, PptxError> {
        Ok(FillOrigin::Declaration {
            owner: budget.owner(owner)?,
            source_ordinal: ordinal,
        })
    }
    fn parent_surface(
        &self,
        part: &str,
        budget: &mut Budget<'_>,
    ) -> Result<Option<String>, PptxError> {
        let source = self.surface(part)?;
        let (link, expected) = match source.kind {
            SurfaceKind::Slide => (&source.links.layout, SurfaceKind::Layout),
            SurfaceKind::Layout => (&source.links.master, SurfaceKind::Master),
            SurfaceKind::Master => return Ok(None),
        };
        link.as_ref()
            .map(|part| {
                budget.step()?;
                budget.bytes(part.len())?;
                if self.surface(part)?.kind != expected {
                    return Err(conflict(
                        "fill inheritance does not follow the surface hierarchy",
                    ));
                }
                Ok(part.clone())
            })
            .transpose()
    }
    fn parent_object(
        &self,
        owner: &FillOwner,
        object: &SourceObject,
        budget: &mut Budget<'_>,
    ) -> Result<Option<FillOwner>, Failure> {
        match &object.resolution.placeholder_match {
            SourcePlaceholderMatch::NotPlaceholder
            | SourcePlaceholderMatch::Master
            | SourcePlaceholderMatch::Detached => Ok(None),
            SourcePlaceholderMatch::Matched { target, .. } => {
                if self.parent_surface(&owner.part, budget)?.as_deref()
                    != Some(target.part.as_str())
                {
                    return Err(conflict(
                        "fill placeholder target is not on the linked parent surface",
                    )
                    .into());
                }
                budget.bytes(target.part.len())?;
                let target_kind = match owner.target {
                    FillTarget::Object { .. } => FillTarget::Object {
                        native_id: target.native_id,
                    },
                    FillTarget::Picture { .. } => FillTarget::Picture {
                        native_id: target.native_id,
                    },
                    FillTarget::Line { .. } => FillTarget::Line {
                        native_id: target.native_id,
                    },
                    _ => return Err(conflict("non-object placeholder fill").into()),
                };
                Ok(Some(FillOwner {
                    part: target.part.clone(),
                    target: target_kind,
                }))
            }
            matching => {
                if let SourcePlaceholderMatch::Ambiguous { part, .. } = matching {
                    budget.bytes(part.len())?;
                }
                Err(FillUnresolved::Placeholder {
                    owner: budget.owner(owner)?,
                    matching: matching.clone(),
                }
                .into())
            }
        }
    }
    fn group_parent(
        &self,
        owner: &FillOwner,
        origin: &FillOrigin,
        budget: &mut Budget<'_>,
    ) -> Result<FillOwner, Failure> {
        if native_id(&owner.target).is_none() {
            return Err(FillUnresolved::GroupWithoutParent {
                origin: budget.origin(origin)?,
            }
            .into());
        }
        let object = self.object(owner)?;
        let target = if let Some(id) = object.parent_group {
            if self
                .objects
                .get(&(owner.part.as_str(), id))
                .is_none_or(|o| o.kind != SourceObjectKind::Group)
            {
                return Err(conflict("fill parent is not a native group").into());
            }
            FillTarget::Object { native_id: id }
        } else {
            FillTarget::RootGroup {}
        };
        budget.bytes(owner.part.len())?;
        Ok(FillOwner {
            part: owner.part.clone(),
            target,
        })
    }
    fn theme(
        &self,
        owner: &FillOwner,
        reference: Reference<'_>,
        partial: &mut merge::Partial,
        budget: &mut Budget<'_>,
    ) -> Result<(), Failure> {
        budget.step()?;
        let origin = self.reference_origin(owner, reference.ordinal, budget)?;
        if let Some(ordinal) = reference.retained.first() {
            return Err(FillUnresolved::RetainedContent {
                origin: budget.at(&origin, *ordinal)?,
            }
            .into());
        }
        if reference.line && reference.index == 0 {
            return Ok(());
        }
        if !reference.line && [0, 1000].contains(&reference.index) {
            return partial.merge(merge::Input::None, &[], &origin, Some(owner), budget);
        }
        let view = if matches!(owner.target, FillTarget::Background {}) {
            self.surface(self.background_part)?
        } else {
            self.view
        };
        let binding = view.theme_selection.format.as_ref().ok_or_else(|| {
            FillUnresolved::MissingFormatScheme {
                owner: owner.clone(),
            }
        })?;
        let scheme = self
            .index
            .themes
            .get(&binding.part)
            .and_then(|t| t.format_scheme.as_ref())
            .filter(|s| s.source_ordinal == binding.source_ordinal)
            .ok_or_else(|| conflict("fill format binding differs from inspected source"))?;
        let (entries, offset) = if reference.line {
            (&scheme.lines, reference.index - 1)
        } else if reference.index > 1000 {
            (&scheme.background_fills, reference.index - 1001)
        } else {
            (&scheme.fills, reference.index - 1)
        };
        let entry =
            entries
                .get(offset as usize)
                .ok_or_else(|| FillUnresolved::StyleIndexOutOfRange {
                    owner: owner.clone(),
                    index: reference.index,
                    available: entries.len() as u32,
                })?;
        budget.bytes(binding.part.len())?;
        let origin = FillOrigin::Theme {
            part: binding.part.clone(),
            source_ordinal: entry.source_ordinal,
            via: budget.owner(owner)?,
            reference_ordinal: reference.ordinal,
            style_index: reference.index,
        };
        if reference.line {
            let line = entry
                .line
                .as_ref()
                .filter(|l| l.source_ordinal == entry.source_ordinal)
                .ok_or_else(|| conflict("line fill theme binding differs"))?;
            if let Some(fill) = &line.fill {
                let (ordinal, input) = line_input(fill);
                let at = budget.at(&origin, ordinal)?;
                partial.merge(input, &line.retained_ordinals, &at, Some(owner), budget)?;
            }
        } else {
            let fill = entry
                .fill
                .as_ref()
                .filter(|f| f.source_ordinal == entry.source_ordinal)
                .ok_or_else(|| conflict("theme fill binding differs"))?;
            partial.merge(
                (&fill.definition).into(),
                &fill.retained_ordinals,
                &origin,
                Some(owner),
                budget,
            )?;
        }
        Ok(())
    }
    fn direct(
        &self,
        owner: &FillOwner,
        object: &SourceObject,
        partial: &mut merge::Partial,
        budget: &mut Budget<'_>,
    ) -> Result<(), Failure> {
        let fill = match owner.target {
            FillTarget::Object { .. } => object.fill.as_ref(),
            FillTarget::Picture { .. } => object.picture_fill.as_ref(),
            FillTarget::Line { .. } => {
                if let Some(line) = &object.line
                    && let Some(fill) = &line.fill
                {
                    let (ordinal, input) = line_input(fill);
                    let origin = self.declaration(owner, ordinal, budget)?;
                    partial.merge(input, &line.retained_ordinals, &origin, Some(owner), budget)?;
                }
                return Ok(());
            }
            _ => unreachable!("object target"),
        };
        if let Some(fill) = fill {
            let origin = self.declaration(owner, fill.source_ordinal, budget)?;
            partial.merge(
                (&fill.definition).into(),
                &fill.retained_ordinals,
                &origin,
                Some(owner),
                budget,
            )?;
        }
        Ok(())
    }
    fn redirect(
        &self,
        target: FillOwner,
        origin: FillOrigin,
        redirects: &mut Vec<FillRedirect>,
        budget: &mut Budget<'_>,
    ) -> Result<FillOwner, PptxError> {
        budget.step()?;
        if redirects.len() >= budget.limits.max_group_hops {
            return Err(PptxError::Limit("fill redirect hops"));
        }
        redirects.push(FillRedirect {
            declared_by: origin,
            target: budget.owner(&target)?,
        });
        Ok(target)
    }
    fn resolve(
        &self,
        start: FillOwner,
        budget: &mut Budget<'_>,
    ) -> Result<(EffectiveFill, Vec<FillRedirect>), Failure> {
        if table::is_target(&start.target) {
            return self.table_fill(&start, budget).map(|fill| (fill, vec![]));
        }
        // A picture placeholder can match a p:sp on its layout/master. That
        // ancestor has no picture payload, but its placeholder chain is valid.
        // Only the requested object itself must be a picture for this target.
        if matches!(start.target, FillTarget::Picture { .. })
            && self.object(&start)?.kind != SourceObjectKind::Picture
        {
            return Err(FillUnresolved::UnsupportedTarget { owner: start }.into());
        }
        let mut owner = start;
        let mut seen = BTreeSet::new();
        let mut partial = merge::Partial::default();
        let mut redirects = Vec::new();
        loop {
            budget.step()?;
            if !seen.insert(budget.owner(&owner)?) {
                return Err(conflict("native fill inheritance cycle").into());
            }
            match &owner.target {
                FillTarget::Background {} => {
                    let surface = self.surface(&owner.part)?;
                    if let Some(background) = &surface.background {
                        let origin = self.declaration(&owner, background.source_ordinal, budget)?;
                        if let Some(ordinal) = background.retained_ordinals.first() {
                            return Err(FillUnresolved::RetainedContent {
                                origin: budget.at(&origin, *ordinal)?,
                            }
                            .into());
                        }
                        match &background.definition {
                            SourceBackgroundDefinition::Properties {
                                shade_to_title: Some(true),
                                ..
                            } => {
                                return Err(
                                    FillUnresolved::UnsupportedBackgroundMode { owner }.into()
                                );
                            }
                            SourceBackgroundDefinition::Properties { fill, effects, .. } => {
                                if let Some(effects) = effects {
                                    if let Some(id) = effects.retained_ordinals.first() {
                                        return Err(FillUnresolved::RetainedContent {
                                            origin: budget.at(&origin, *id)?,
                                        }
                                        .into());
                                    }
                                    if !effects.is_explicitly_empty_list() {
                                        return Err(FillUnresolved::EffectEvaluationRequired {
                                            origin: budget.at(&origin, effects.source_ordinal)?,
                                        }
                                        .into());
                                    }
                                }
                                let at = budget.at(&origin, fill.source_ordinal)?;
                                partial.merge(
                                    (&fill.definition).into(),
                                    &fill.retained_ordinals,
                                    &at,
                                    Some(&owner),
                                    budget,
                                )?;
                            }
                            SourceBackgroundDefinition::Reference(reference) => self.theme(
                                &owner,
                                Reference {
                                    ordinal: reference.source_ordinal,
                                    index: reference.index,
                                    retained: &reference.retained_ordinals,
                                    line: false,
                                },
                                &mut partial,
                                budget,
                            )?,
                        }
                        break;
                    }
                    if let Some(parent) = self.parent_surface(&owner.part, budget)? {
                        owner.part = parent;
                        continue;
                    }
                    break;
                }
                FillTarget::RootGroup {} => {
                    if let Some(fill) = &self.surface(&owner.part)?.root_group_fill {
                        let origin = self.declaration(&owner, fill.source_ordinal, budget)?;
                        partial.merge(
                            (&fill.definition).into(),
                            &fill.retained_ordinals,
                            &origin,
                            Some(&owner),
                            budget,
                        )?;
                    }
                    break;
                }
                target => {
                    let object = self.object(&owner)?;
                    if object.kind == SourceObjectKind::GraphicFrame
                        || matches!(target, FillTarget::Line { .. })
                            && object.kind == SourceObjectKind::Group
                    {
                        return Err(FillUnresolved::UnsupportedTarget { owner }.into());
                    }
                    if matches!(target, FillTarget::Object { .. })
                        && let Some(flag) = &object.use_background_fill
                        && flag.value
                    {
                        let origin = self.declaration(&owner, flag.source_ordinal, budget)?;
                        budget.bytes(self.background_part.len())?;
                        let target = FillOwner {
                            part: self.background_part.into(),
                            target: FillTarget::Background {},
                        };
                        owner = self.redirect(target, origin, &mut redirects, budget)?;
                        continue;
                    }
                    self.direct(&owner, object, &mut partial, budget)?;
                    if !partial.complete() {
                        let reference = match target {
                            FillTarget::Object { .. } => {
                                object.fill_reference.as_ref().map(|r| Reference {
                                    ordinal: r.source_ordinal,
                                    index: r.index,
                                    retained: &r.retained_ordinals,
                                    line: false,
                                })
                            }
                            FillTarget::Line { .. } => {
                                object.line_reference.as_ref().map(|r| Reference {
                                    ordinal: r.source_ordinal,
                                    index: r.index,
                                    retained: &r.retained_ordinals,
                                    line: true,
                                })
                            }
                            _ => None,
                        };
                        if let Some(reference) = reference {
                            self.theme(&owner, reference, &mut partial, budget)?;
                        }
                    }
                    if let Some(group) = partial.group() {
                        let origin = budget.origin(group)?;
                        let target = self.group_parent(&owner, &origin, budget)?;
                        owner = self.redirect(target, origin, &mut redirects, budget)?;
                        partial = merge::Partial::default();
                        continue;
                    }
                    if partial.complete() {
                        break;
                    }
                    if let Some(parent) = self.parent_object(&owner, object, budget)? {
                        owner = parent;
                        continue;
                    }
                    break;
                }
            }
        }
        Ok((partial.finish(budget)?, redirects))
    }
}

pub fn query(
    index: &SourceIndex,
    request: &SourceFillQuery,
    limits: FillResolveLimits,
    check: &dyn Fn() -> bool,
) -> Result<SourceFillStyles, PptxError> {
    query_in_context(index, request, &request.surface, limits, check)
}

/// Resolve declarations on request.surface using the explicitly selected drawing
/// surface's theme, color map and background. Original owners are never rewritten.
/// Page composition validates that these surfaces belong to one source hierarchy.
pub fn query_in_context(
    index: &SourceIndex,
    request: &SourceFillQuery,
    drawing_surface: &str,
    limits: FillResolveLimits,
    check: &dyn Fn() -> bool,
) -> Result<SourceFillStyles, PptxError> {
    query_on_page(
        index,
        request,
        drawing_surface,
        drawing_surface,
        limits,
        check,
    )
}

/// Keep the object's drawing style context independent from the page background
/// sampled by useBgFill. The caller validates the linked page composition.
pub fn query_on_page(
    index: &SourceIndex,
    request: &SourceFillQuery,
    drawing_surface: &str,
    background_surface: &str,
    limits: FillResolveLimits,
    check: &dyn Fn() -> bool,
) -> Result<SourceFillStyles, PptxError> {
    query_on_page_prepared(
        index,
        request,
        drawing_surface,
        background_surface,
        limits,
        check,
    )
    .map(|(styles, _)| styles)
}

pub(in crate::source) fn query_on_page_prepared<'a>(
    index: &'a SourceIndex,
    request: &SourceFillQuery,
    drawing_surface: &str,
    background_surface: &str,
    limits: FillResolveLimits,
    check: &dyn Fn() -> bool,
) -> Result<(SourceFillStyles, TablePaintBindings<'a>), PptxError> {
    query_on_page_with_preparation(
        index,
        request,
        drawing_surface,
        background_surface,
        limits,
        None,
        check,
    )
}

/// Reuses an immutable source's object index, native table grids and styles.
pub fn query_in_preparation(
    preparation: &mut crate::source::prepared::SourcePreparation<'_>,
    request: &SourceFillQuery,
    drawing_surface: &str,
    background_surface: &str,
    limits: FillResolveLimits,
    check: &dyn Fn() -> bool,
) -> Result<SourceFillStyles, PptxError> {
    query_on_page_with_preparation(
        preparation.index(),
        request,
        drawing_surface,
        background_surface,
        limits,
        Some(preparation),
        check,
    )
    .map(|(styles, _)| styles)
}

pub(in crate::source) fn query_on_page_with_preparation<'a>(
    index: &'a SourceIndex,
    request: &SourceFillQuery,
    drawing_surface: &str,
    background_surface: &str,
    limits: FillResolveLimits,
    preparation: Option<&mut crate::source::prepared::SourcePreparation<'a>>,
    check: &dyn Fn() -> bool,
) -> Result<(SourceFillStyles, TablePaintBindings<'a>), PptxError> {
    cancelled(check)?;
    if index.source_sha256 != request.expected_source_sha256 {
        return Err(conflict("fill query source digest differs"));
    }
    if request.targets.len() > limits.max_queries {
        return Err(PptxError::Limit("fill queries"));
    }
    if !index.surfaces.contains_key(&request.surface)
        || !index.surfaces.contains_key(background_surface)
    {
        return Err(value(
            "fillQuery.surface",
            "surface is not in inspected source",
        ));
    }
    let view = index
        .surfaces
        .get(drawing_surface)
        .ok_or_else(|| value("fillQuery.surface", "surface is not in inspected source"))?;
    let mut budget = Budget::new(limits, check);
    let objects = if let Some(prepared) = preparation.as_ref() {
        if !std::ptr::eq(prepared.index(), index) {
            return Err(conflict("fill preparation source differs"));
        }
        let objects = prepared.bindings()?;
        budget.steps(objects.len())?;
        objects
    } else {
        crate::source::prepared::ObjectBindings::new(index, &mut || budget.step())?
    };
    let tables = table::prepare(
        index,
        &request.surface,
        &request.targets,
        &objects,
        preparation,
        &mut budget,
    )?;
    let resolver = Resolver {
        index,
        background_part: index
            .surfaces
            .get_key_value(background_surface)
            .expect("validated background surface")
            .0
            .as_str(),
        view,
        objects,
        tables,
    };
    let mut targets = Vec::with_capacity(request.targets.len());
    for target in &request.targets {
        budget.step()?;
        if let Some(id) = native_id(target)
            && !resolver
                .objects
                .contains_key(&(request.surface.as_str(), id))
        {
            return Err(value(
                "fillQuery.targets",
                "object is not in requested surface",
            ));
        }
        budget.bytes(request.surface.len())?;
        let owner = FillOwner {
            part: request.surface.clone(),
            target: target.clone(),
        };
        let outcome = match resolver.resolve(owner, &mut budget) {
            Ok((fill, redirects)) => FillOutcome::Resolved {
                fill: Box::new(fill),
                redirects,
            },
            Err(Failure::Unresolved(reason)) => FillOutcome::Unresolved { reason },
            Err(Failure::Abort(error)) => return Err(error),
        };
        targets.push(SourceFillResult {
            target: target.clone(),
            outcome,
        });
    }
    Ok((
        SourceFillStyles {
            source_sha256: index.source_sha256.clone(),
            surface: request.surface.clone(),
            profile: request.profile,
            targets,
        },
        resolver.tables,
    ))
}
