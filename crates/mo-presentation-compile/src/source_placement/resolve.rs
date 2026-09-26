use super::{Context, Failure, types::*};
use mo_common::Emu;
use mo_pptx::source::{
    SourceObject, SourceObjectKind, SourceObjectRef, SourcePlaceholderMatch, SourceResolvedValue,
    SourceTransform,
};
use mo_presentation_model::{Point, Size};
const ZERO: Point = Point {
    x: Emu::new(0),
    y: Emu::new(0),
};
const SIZE: Size = Size {
    width: Emu::new(0),
    height: Emu::new(0),
};
fn declared<T>(value: T, owner: SourceObjectRef) -> TransformValue<T> {
    TransformValue {
        value,
        source: TransformValueSource::Declaration { object: owner },
    }
}
fn defaulted<T: Copy>(value: Option<T>, fallback: T, owner: &SourceObjectRef) -> TransformValue<T> {
    match value {
        Some(value) => declared(value, owner.clone()),
        None => TransformValue {
            value: fallback,
            source: TransformValueSource::Default {
                object: owner.clone(),
            },
        },
    }
}
fn reason(owner: &SourceObjectRef, cause: PlacementCause) -> Failure {
    Failure::Unresolved(PlacementUnresolved {
        object: owner.clone(),
        cause,
    })
}
fn retained(owner: &SourceObjectRef, t: Option<&SourceTransform>) -> Result<(), Failure> {
    if let Some(n) = t.and_then(|t| t.retained_ordinals.first()) {
        return Err(reason(
            owner,
            PlacementCause::RetainedTransform { source_ordinal: *n },
        ));
    }
    Ok(())
}
impl<'a> Context<'a, '_> {
    fn inherited<T: Clone>(
        &mut self,
        value: Option<&SourceResolvedValue<T>>,
        owner: &SourceObjectRef,
        missing: PlacementCause,
    ) -> Result<TransformValue<T>, Failure> {
        let value = value.ok_or_else(|| reason(owner, missing))?;
        let declared_object = self.object(&value.declared_by)?;
        retained(&value.declared_by, declared_object.transform.as_ref())?;
        Ok(declared(value.value.clone(), value.declared_by.clone()))
    }
    fn orientation(
        &mut self,
        owner: &SourceObjectRef,
    ) -> Result<(SourceObjectRef, Option<&'a SourceTransform>), Failure> {
        let mut at = owner.clone();
        for _ in 0..=self.work.limits.max_depth {
            self.work.step()?;
            let object = self.object(&at)?;
            retained(&at, object.transform.as_ref())?;
            if object.transform.is_some() {
                return Ok((at, object.transform.as_ref()));
            }
            match &object.resolution.placeholder_match {
                SourcePlaceholderMatch::Matched { target, .. } => at = target.clone(),
                SourcePlaceholderMatch::NotPlaceholder
                | SourcePlaceholderMatch::Master
                | SourcePlaceholderMatch::Detached => return Ok((at, None)),
                status => {
                    return Err(reason(
                        &at,
                        PlacementCause::Inheritance {
                            status: status.clone(),
                        },
                    ));
                }
            }
        }
        Err(SourcePlacementError::Limit("placeholder depth").into())
    }
    pub fn resolve(
        &mut self,
        owner: &SourceObjectRef,
        object: &SourceObject,
    ) -> Result<ResolvedNativeTransform, Failure> {
        self.work.step()?;
        let t = object.transform.as_ref();
        retained(owner, t)?;
        let group = object.kind == SourceObjectKind::Group;
        let (origin, size) = if group {
            (
                defaulted(t.and_then(|t| t.origin), ZERO, owner),
                defaulted(t.and_then(|t| t.size), SIZE, owner),
            )
        } else {
            (
                self.inherited(
                    object.resolution.origin.as_ref(),
                    owner,
                    PlacementCause::MissingOrigin,
                )?,
                self.inherited(
                    object.resolution.size.as_ref(),
                    owner,
                    PlacementCause::MissingSize,
                )?,
            )
        };
        let (selected, orientation) = if group {
            (owner.clone(), t)
        } else {
            self.orientation(owner)?
        };
        let child_origin = group.then(|| defaulted(t.and_then(|t| t.child_origin), ZERO, owner));
        let child_size = group.then(|| defaulted(t.and_then(|t| t.child_size), SIZE, owner));
        for (name, values, lower) in [
            (
                "origin",
                [origin.value.x.get(), origin.value.y.get()],
                -27_273_042_329_600,
            ),
            ("size", [size.value.width.get(), size.value.height.get()], 0),
            (
                "childOrigin",
                child_origin
                    .as_ref()
                    .map(|v| [v.value.x.get(), v.value.y.get()])
                    .unwrap_or([0, 0]),
                -27_273_042_329_600,
            ),
            (
                "childSize",
                child_size
                    .as_ref()
                    .map(|v| [v.value.width.get(), v.value.height.get()])
                    .unwrap_or([0, 0]),
                0,
            ),
        ] {
            if values
                .iter()
                .any(|v| !(lower..=27_273_042_316_900).contains(v))
            {
                return Err(reason(
                    owner,
                    PlacementCause::InvalidCoordinate { field: name.into() },
                ));
            }
        }
        Ok(ResolvedNativeTransform {
            origin,
            size,
            rotation: defaulted(orientation.and_then(|t| t.rotation), 0, &selected),
            flip_horizontal: defaulted(
                orientation.and_then(|t| t.flip_horizontal),
                false,
                &selected,
            ),
            flip_vertical: defaulted(orientation.and_then(|t| t.flip_vertical), false, &selected),
            child_origin,
            child_size,
            graphic_frame_orientation_ignored: object.kind == SourceObjectKind::GraphicFrame,
        })
    }
}
