//! Property channels share precedence, but never overwrite another property.
use super::Moment;
use crate::{
    Effect, ExactRotation, ExactScale, ExactValue, RotationBasis, RotationComposition,
    TimelineError, Visibility, exact::Ratio,
};
use mo_common::ObjectId;
use std::collections::BTreeMap;

type Winners<T> = BTreeMap<ObjectId, (Moment, usize, T)>;
#[derive(Default)]
pub(super) struct Properties {
    rotations: Winners<(RotationBasis, Ratio)>,
    rotation_additions: Vec<(ObjectId, Moment, usize, Ratio)>,
    scales: Winners<ExactScale>,
    visibility: Winners<Visibility>,
    opacity: Winners<ExactValue>,
    motion: Winners<crate::ExactMotion>,
}
pub(super) struct Values {
    pub rotations: BTreeMap<ObjectId, ExactRotation>,
    pub scales: BTreeMap<ObjectId, ExactScale>,
    pub visibility: BTreeMap<ObjectId, Visibility>,
    pub opacity: BTreeMap<ObjectId, ExactValue>,
    pub motion: BTreeMap<ObjectId, crate::ExactMotion>,
}
fn select<T>(map: &mut Winners<T>, target: &ObjectId, start: &Moment, rank: usize, value: T) {
    if map.get(target).is_none_or(|(previous, order, _)| {
        previous.precedes(start) || (start == previous && rank > *order)
    }) {
        map.insert(target.clone(), (start.clone(), rank, value));
    }
}
fn values<T>(map: Winners<T>) -> BTreeMap<ObjectId, T> {
    map.into_iter()
        .map(|(id, (_, _, value))| (id, value))
        .collect()
}
impl Properties {
    pub fn sample(
        &mut self,
        effect: &Effect,
        progress: &Ratio,
        start: &Moment,
        rank: usize,
        bits: u64,
        path: Option<&crate::motion_path::PathPlan>,
    ) -> Result<(), TimelineError> {
        let interpolate = |from: i64, to: i64| -> Result<ExactValue, TimelineError> {
            Ok(Ratio::integer(from)
                .add(&Ratio::integer(to - from).mul(progress, bits)?, bits)?
                .wire())
        };
        match effect {
            Effect::MotionPath { target, .. } => {
                let value = path.expect("compiled motion path").sample(progress, bits)?;
                select(&mut self.motion, target, start, rank, value);
            }
            Effect::MotionLine { target, from, to } => {
                let axis = |a: &crate::MotionCoordinate, b: &crate::MotionCoordinate| {
                    let a = a.ratio(bits)?;
                    Ok::<_, TimelineError>(
                        a.add(&b.ratio(bits)?.sub(&a, bits)?.mul(progress, bits)?, bits)?
                            .wire(),
                    )
                };
                select(
                    &mut self.motion,
                    target,
                    start,
                    rank,
                    crate::ExactMotion {
                        x: axis(&from.x, &to.x)?,
                        y: axis(&from.y, &to.y)?,
                    },
                );
            }
            Effect::Fade { target, transition } => {
                let (from, to) = match transition {
                    crate::FadeTransition::In => (0, 1),
                    crate::FadeTransition::Out => (1, 0),
                };
                select(
                    &mut self.opacity,
                    target,
                    start,
                    rank,
                    interpolate(from, to)?,
                );
            }
            Effect::SetVisibility { target, value } => {
                select(&mut self.visibility, target, start, rank, *value);
            }
            Effect::Rotation {
                target,
                from,
                to,
                composition,
            } => {
                let angle = Ratio::integer(i64::from(*from)).add(
                    &Ratio::integer(i64::from(*to) - i64::from(*from)).mul(progress, bits)?,
                    bits,
                )?;
                match composition {
                    RotationComposition::Add => {
                        self.rotation_additions
                            .push((target.clone(), start.clone(), rank, angle))
                    }
                    RotationComposition::Absolute | RotationComposition::Layout => {
                        let basis = if *composition == RotationComposition::Absolute {
                            RotationBasis::Absolute
                        } else {
                            RotationBasis::Layout
                        };
                        select(&mut self.rotations, target, start, rank, (basis, angle));
                    }
                }
            }
            Effect::Scale { target, from, to } => {
                select(
                    &mut self.scales,
                    target,
                    start,
                    rank,
                    ExactScale {
                        x: interpolate(i64::from(from.x), i64::from(to.x))?,
                        y: interpolate(i64::from(from.y), i64::from(to.y))?,
                    },
                );
            }
        }
        Ok(())
    }
    pub fn finish(self, bits: u64, check: &dyn Fn() -> bool) -> Result<Values, TimelineError> {
        // Only additions above the latest replacing value survive. Addition is
        // commutative, so no per-frame stack sort or history mutation is needed.
        let mut rotations: BTreeMap<_, _> = self
            .rotations
            .iter()
            .map(|(id, (_, _, value))| (id.clone(), value.clone()))
            .collect();
        for (id, start, rank, offset) in self.rotation_additions {
            crate::cancel(check)?;
            if self
                .rotations
                .get(&id)
                .is_some_and(|(replacement, order, _)| {
                    start.precedes(replacement) || (start == *replacement && rank < *order)
                })
            {
                continue;
            }
            let (_, value) = rotations
                .entry(id)
                .or_insert_with(|| (RotationBasis::Layout, Ratio::integer(0)));
            *value = value.add(&offset, bits)?;
        }
        let rotations = rotations
            .into_iter()
            .map(|(id, (basis, value))| {
                let value = value.wire();
                (
                    id,
                    ExactRotation {
                        numerator: value.numerator,
                        denominator: value.denominator,
                        basis,
                    },
                )
            })
            .collect();
        Ok(Values {
            rotations,
            scales: values(self.scales),
            visibility: values(self.visibility),
            opacity: values(self.opacity),
            motion: values(self.motion),
        })
    }
}
