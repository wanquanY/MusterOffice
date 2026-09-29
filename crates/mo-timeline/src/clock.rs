//! Compiled behavior clock shared by flat graphs and hierarchical trees.
//! Scheduling uses parent duration; effects consume exact filtered simple time.
use crate::{
    RepeatCount, RepeatDuration, TimeTransform, TimelineError, TimingNode, exact::Ratio, invalid,
};

#[derive(Debug, Clone)]
pub(crate) struct BehaviorClock {
    cycle: Ratio,
    active: Option<Ratio>,
    rate: Ratio,
    parent_duration: Option<Ratio>,
    backwards: bool,
    auto_reverse: bool,
    easing: Option<Easing>,
}
#[derive(Debug, Clone)]
pub(crate) struct Easing {
    acceleration: Ratio,
    deceleration: Ratio,
    run_rate: Ratio,
}
pub(crate) fn validate(node: &TimingNode) -> Result<(), TimelineError> {
    if node.duration.ticks.get() <= 0 || node.repeat_milli == RepeatCount::Finite(0) {
        return Err(invalid(
            Some(&node.id),
            "positive duration and repeat count required",
        ));
    }
    if matches!(node.repeat_duration, Some(RepeatDuration::Finite(t)) if t.ticks.get() < 0) {
        return Err(invalid(Some(&node.id), "negative repeat duration"));
    }
    validate_transform(node.time_transform, &node.id)
}
pub(crate) fn validate_transform(
    transform: Option<TimeTransform>,
    id: &mo_common::TimingNodeId,
) -> Result<(), TimelineError> {
    let Some(t) = transform else {
        return Ok(());
    };
    if t.speed_milli_percent == 0 {
        return Err(invalid(Some(id), "time transform speed must be nonzero"));
    }
    if u64::from(t.acceleration_milli_percent) + u64::from(t.deceleration_milli_percent) > 100_000 {
        return Err(invalid(
            Some(id),
            "acceleration and deceleration must sum to at most 100000",
        ));
    }
    Ok(())
}
impl BehaviorClock {
    pub(crate) fn new(node: &TimingNode, bits: u64) -> Result<Self, TimelineError> {
        validate(node)?;
        let t = node.time_transform.unwrap_or_default();
        let cycle = Ratio::time(node.duration)
            .mul(&Ratio::integer(if t.auto_reverse { 2 } else { 1 }), bits)?;
        let count_bound = match node.repeat_milli {
            RepeatCount::Finite(count) => {
                Some(cycle.mul(&Ratio::new(count.into(), 1000.into(), bits)?, bits)?)
            }
            RepeatCount::Indefinite => None,
        };
        let duration_bound = match node.repeat_duration {
            Some(RepeatDuration::Finite(t)) => Some(Ratio::time(t)),
            None | Some(RepeatDuration::Indefinite) => None,
        };
        let active = match (count_bound, duration_bound) {
            (Some(a), Some(b)) => Some(if a.cmp(&b).is_gt() { b } else { a }),
            (a, b) => a.or(b),
        };
        // Promote before abs so the full signed native speed range is exact.
        let rate = Ratio::new(
            i64::from(t.speed_milli_percent).abs().into(),
            100_000.into(),
            bits,
        )?;
        let parent_duration = active.as_ref().map(|a| a.div(&rate, bits)).transpose()?;
        Ok(Self {
            cycle,
            active,
            rate,
            parent_duration,
            backwards: t.speed_milli_percent < 0,
            auto_reverse: t.auto_reverse,
            easing: Easing::new(t, bits)?,
        })
    }
    pub(crate) fn parent_duration(&self) -> Option<&Ratio> {
        self.parent_duration.as_ref()
    }
    pub(crate) fn first_cycle_duration(&self, bits: u64) -> Result<Ratio, TimelineError> {
        self.cycle.div(&self.rate, bits)
    }
    pub(crate) fn needs_endpoint(&self, has_end_conditions: bool) -> bool {
        self.backwards && (self.active.is_none() || has_end_conditions)
    }
    pub(crate) fn sample(
        &self,
        elapsed: &Ratio,
        ended: bool,
        parent_backwards: bool,
        entering: bool,
        cutoff: Option<&Ratio>,
        bits: u64,
    ) -> Result<(String, Ratio), TimelineError> {
        let local = elapsed.mul(&self.rate, bits)?;
        let local = if self.backwards {
            // Finite declarations keep their own reverse origin when clipped.
            // An indefinite repeat can only reverse if its resolved parent
            // interval supplies an endpoint; never invent a large repeat count.
            let active = match cutoff {
                Some(endpoint) => endpoint.mul(&self.rate, bits)?,
                None => self
                    .active
                    .as_ref()
                    .ok_or_else(|| {
                        invalid(
                            None,
                            "indefinite reverse behavior requires a resolved finite endpoint",
                        )
                    })?
                    .clone(),
            };
            active.sub(&local, bits)?
        } else {
            local
        };
        let position = local.div(&self.cycle, bits)?;
        let mut iteration = &position.n / &position.d;
        let mut progress = position.fraction(bits)?;
        // Endpoint sides depend on the cascaded direction, not only this leaf's
        // speed. Two reversals restore forward fill; a reversed ancestor enters
        // this leaf at its upper endpoint even though elapsed is not zero.
        let backwards = self.backwards ^ parent_backwards;
        if !progress.positive()
            && position.positive()
            && ((!backwards && ended) || (backwards && entering))
        {
            iteration -= 1;
            progress = Ratio::integer(1);
        }
        if self.auto_reverse {
            progress = progress.mul(&Ratio::integer(2), bits)?;
            if progress.cmp(&Ratio::integer(1)).is_gt() {
                progress = Ratio::integer(2).sub(&progress, bits)?;
            }
        }
        if let Some(easing) = &self.easing {
            progress = easing.apply(&progress, bits)?;
        }
        Ok((iteration.to_string(), progress))
    }
}
impl Easing {
    pub(crate) fn new(t: TimeTransform, bits: u64) -> Result<Option<Self>, TimelineError> {
        if t.acceleration_milli_percent == 0 && t.deceleration_milli_percent == 0 {
            return Ok(None);
        }
        let acceleration = Ratio::new(t.acceleration_milli_percent.into(), 100_000.into(), bits)?;
        let deceleration = Ratio::new(t.deceleration_milli_percent.into(), 100_000.into(), bits)?;
        let run_rate = Ratio::integer(2).div(
            &Ratio::integer(2)
                .sub(&acceleration, bits)?
                .sub(&deceleration, bits)?,
            bits,
        )?;
        Ok(Some(Self {
            acceleration,
            deceleration,
            run_rate,
        }))
    }
    pub(crate) fn apply(&self, p: &Ratio, bits: u64) -> Result<Ratio, TimelineError> {
        let two = Ratio::integer(2);
        if p.cmp(&self.acceleration).is_lt() {
            p.mul(p, bits)?
                .mul(&self.run_rate, bits)?
                .div(&self.acceleration.mul(&two, bits)?, bits)
        } else if p
            .cmp(&Ratio::integer(1).sub(&self.deceleration, bits)?)
            .is_gt()
        {
            let remaining = Ratio::integer(1).sub(p, bits)?;
            Ratio::integer(1).sub(
                &remaining
                    .mul(&remaining, bits)?
                    .mul(&self.run_rate, bits)?
                    .div(&self.deceleration.mul(&two, bits)?, bits)?,
                bits,
            )
        } else {
            p.sub(&self.acceleration.div(&two, bits)?, bits)?
                .mul(&self.run_rate, bits)
        }
    }
}
