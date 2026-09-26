//! Digest-bound native surface placement. Source projection resolves package
//! identity and placeholder matches; this stage resolves transforms and shares
//! the authored DrawingML arithmetic without constructing a lossy author model.
mod resolve;
mod types;
use crate::{
    CompileError,
    placement_core::{Engine, Frame, State},
};
use mo_common::Emu;
use mo_pptx::source::{
    SourceIndex, SourceObject, SourceObjectKind, SourceObjectRef, SourceSurface,
};
use mo_presentation_model::Point;
use std::collections::{BTreeMap, BTreeSet};
/// Only produced by a bound native timeline sample inside this crate.
pub(crate) type SourceRotations = BTreeMap<(String, u32), crate::angle::Angle>;
pub use types::*;
struct Work<'a> {
    limits: SourcePlacementLimits,
    check: &'a dyn Fn() -> bool,
    steps: usize,
    indexed: usize,
}
impl Work<'_> {
    fn step(&mut self) -> Result<(), SourcePlacementError> {
        if (self.check)() {
            return Err(SourcePlacementError::Cancelled);
        }
        self.steps = self
            .steps
            .checked_add(1)
            .ok_or(SourcePlacementError::Limit("work"))?;
        if self.steps > self.limits.max_steps {
            return Err(SourcePlacementError::Limit("work"));
        }
        Ok(())
    }
}
struct Context<'a, 'b> {
    index: &'a SourceIndex,
    lookup: BTreeMap<&'a str, BTreeMap<u32, &'a SourceObject>>,
    work: Work<'b>,
}
impl<'a> Context<'a, '_> {
    fn surface(&self, part: &str) -> Result<&'a SourceSurface, SourcePlacementError> {
        self.index
            .surfaces
            .get(part)
            .ok_or(SourcePlacementError::Invalid("surface reference"))
    }
    fn object(
        &mut self,
        owner: &SourceObjectRef,
    ) -> Result<&'a SourceObject, SourcePlacementError> {
        self.work.step()?;
        if !self.lookup.contains_key(owner.part.as_str()) {
            let (part, surface) = self
                .index
                .surfaces
                .get_key_value(&owner.part)
                .ok_or(SourcePlacementError::Invalid("surface reference"))?;
            let mut objects = BTreeMap::new();
            for object in &surface.objects {
                self.work.step()?;
                self.work.indexed = self
                    .work
                    .indexed
                    .checked_add(1)
                    .ok_or(SourcePlacementError::Limit("indexed objects"))?;
                if self.work.indexed > self.work.limits.max_indexed_objects {
                    return Err(SourcePlacementError::Limit("indexed objects"));
                }
                if objects.insert(object.native_id, object).is_some() {
                    return Err(SourcePlacementError::Invalid("duplicate object identity"));
                }
            }
            self.lookup.insert(part.as_str(), objects);
        }
        self.lookup[owner.part.as_str()]
            .get(&owner.native_id)
            .copied()
            .ok_or(SourcePlacementError::Invalid("object reference"))
    }
}
enum Failure {
    Unresolved(PlacementUnresolved),
    Abort(SourcePlacementError),
}
impl From<SourcePlacementError> for Failure {
    fn from(error: SourcePlacementError) -> Self {
        Self::Abort(error)
    }
}
struct Cached {
    depth: u32,
    outcome: SourcePlacementOutcome,
    children: Option<State>,
}
fn frame(t: &ResolvedNativeTransform) -> Frame {
    let mut source_size = t
        .child_size
        .as_ref()
        .map(|v| v.value)
        .unwrap_or(t.size.value);
    if t.child_size.is_some() {
        // MS-OE376 chExt: no scaling on a zero or omitted child-size axis. The
        // rotation center still belongs to the full unscaled target rectangle.
        if source_size.width.get() == 0 {
            source_size.width = t.size.value.width;
        }
        if source_size.height.get() == 0 {
            source_size.height = t.size.value.height;
        }
    }
    Frame {
        origin: t.origin.value,
        target_size: t.size.value,
        source_size,
        source_origin: t.child_origin.as_ref().map(|v| v.value).unwrap_or(Point {
            x: Emu::new(0),
            y: Emu::new(0),
        }),
        rotation: crate::angle::Angle::integer(if t.graphic_frame_orientation_ignored {
            0
        } else {
            i64::from(t.rotation.value)
        }),
        flips: if t.graphic_frame_orientation_ignored {
            [false; 2]
        } else {
            [t.flip_horizontal.value, t.flip_vertical.value]
        },
    }
}
fn place(
    engine: &mut Engine,
    t: ResolvedNativeTransform,
    parent: &State,
    owner: &SourceObjectRef,
    rotation: Option<&crate::angle::Angle>,
    check: &dyn Fn() -> bool,
) -> Result<(NativePlacement, Option<State>), Failure> {
    let mut f = frame(&t);
    if let Some(rotation) = rotation {
        if t.graphic_frame_orientation_ignored {
            return Err(SourcePlacementError::Invalid(
                "graphic-frame animated orientation not implemented",
            )
            .into());
        }
        f.rotation = rotation.clone();
    }
    let result = engine
        .place(&f, parent, t.child_size.is_some(), check)
        .map_err(|error| match error {
            CompileError::Cancelled => Failure::Abort(SourcePlacementError::Cancelled),
            CompileError::Limit(reason) => Failure::Abort(SourcePlacementError::Limit(reason)),
            CompileError::Range => Failure::Unresolved(PlacementUnresolved {
                object: owner.clone(),
                cause: PlacementCause::NumericRange,
            }),
            _ => Failure::Abort(SourcePlacementError::Invalid("coordinate computation")),
        })?;
    Ok((
        NativePlacement {
            transform: t,
            source_size: f.source_size,
            source_origin: f.source_origin,
            anchor: result.anchor,
            affine: result.affine,
            uncertainty: result.uncertainty,
        },
        result.children,
    ))
}
pub fn source_placements(
    index: &SourceIndex,
    request: &SourcePlacementQuery,
    limits: SourcePlacementLimits,
    check: &dyn Fn() -> bool,
) -> Result<SourcePlacements, SourcePlacementError> {
    source_placements_sampled(index, request, limits, None, check)
}
pub(crate) fn source_placements_sampled(
    index: &SourceIndex,
    request: &SourcePlacementQuery,
    limits: SourcePlacementLimits,
    rotations: Option<&SourceRotations>,
    check: &dyn Fn() -> bool,
) -> Result<SourcePlacements, SourcePlacementError> {
    let mut ctx = Context {
        index,
        lookup: BTreeMap::new(),
        work: Work {
            limits,
            check,
            steps: 0,
            indexed: 0,
        },
    };
    ctx.work.step()?;
    if index.source_sha256 != request.expected_source_sha256 {
        return Err(SourcePlacementError::SourceConflict);
    }
    if request.objects.len() > limits.max_queries {
        return Err(SourcePlacementError::Limit("queries"));
    }
    let surface = ctx.surface(&request.surface)?;
    let mut wanted = BTreeSet::new();
    for id in &request.objects {
        let mut at = Some(*id);
        let mut depth = 0u32;
        while let Some(id) = at {
            ctx.work.step()?;
            if depth > limits.max_depth {
                return Err(SourcePlacementError::Limit("group depth"));
            }
            let object = ctx.object(&SourceObjectRef {
                part: request.surface.clone(),
                native_id: id,
            })?;
            if depth > 0 && object.kind != SourceObjectKind::Group {
                return Err(SourcePlacementError::Invalid("parent is not a group"));
            }
            if !wanted.insert(id) {
                break;
            }
            at = object.parent_group;
            depth += 1;
        }
    }
    let mut engine = Engine::new();
    let mut cache: BTreeMap<u32, Cached> = BTreeMap::new();
    for object in &surface.objects {
        ctx.work.step()?;
        if !wanted.contains(&object.native_id) {
            continue;
        }
        let owner = SourceObjectRef {
            part: request.surface.clone(),
            native_id: object.native_id,
        };
        let parent = object
            .parent_group
            .map(|id| {
                cache.get(&id).ok_or(SourcePlacementError::Invalid(
                    "parent must precede its descendants",
                ))
            })
            .transpose()?;
        let depth = parent.map_or(0, |p| p.depth + 1);
        if depth > limits.max_depth {
            return Err(SourcePlacementError::Limit("group depth"));
        }
        if let Some(Cached {
            outcome: SourcePlacementOutcome::Unresolved { reason },
            ..
        }) = parent
        {
            cache.insert(
                object.native_id,
                Cached {
                    depth,
                    outcome: SourcePlacementOutcome::Unresolved {
                        reason: reason.clone(),
                    },
                    children: None,
                },
            );
            continue;
        }
        let identity = State::identity();
        let state = match parent {
            Some(p) => p.children.as_ref().ok_or(SourcePlacementError::Invalid(
                "missing group coordinate state",
            ))?,
            None => &identity,
        };
        let result = ctx.resolve(&owner, object).and_then(|t| {
            place(
                &mut engine,
                t,
                state,
                &owner,
                rotations.and_then(|r| r.get(&(owner.part.clone(), owner.native_id))),
                check,
            )
        });
        let (outcome, children) = match result {
            Ok((placement, children)) => (
                SourcePlacementOutcome::Resolved {
                    placement: Box::new(placement),
                },
                children,
            ),
            Err(Failure::Unresolved(reason)) => {
                (SourcePlacementOutcome::Unresolved { reason }, None)
            }
            Err(Failure::Abort(error)) => return Err(error),
        };
        cache.insert(
            object.native_id,
            Cached {
                depth,
                outcome,
                children,
            },
        );
    }
    let mut objects = Vec::new();
    for id in &request.objects {
        ctx.work.step()?;
        let c = cache
            .get(id)
            .ok_or(SourcePlacementError::Invalid("missing requested placement"))?;
        objects.push(NativeObjectPlacement {
            native_id: *id,
            parent_group: ctx
                .object(&SourceObjectRef {
                    part: request.surface.clone(),
                    native_id: *id,
                })?
                .parent_group,
            depth: c.depth,
            outcome: c.outcome.clone(),
        });
    }
    ctx.work.step()?;
    Ok(SourcePlacements {
        source_sha256: index.source_sha256.clone(),
        surface: request.surface.clone(),
        profile: request.profile,
        root_transform_ignored: surface.root_group_transform.is_some(),
        objects,
        unique_angles: engine.unique_angles(),
    })
}
