//! Container-owned simple-time filters. The first admitted domain consists of
//! coincident finite subtrees: all descendants start together and have the same
//! active endpoint. Easing fixes both endpoints, so every observable begin/end,
//! ancestor cutoff, restart and navigation seek keeps its existing schedule.
//! Reversal is also admitted in that domain: reversing [0, D] within [0, D]
//! preserves the physical activation interval. Unequal/delayed child intervals
//! require reverse interval selection and are not admitted by this proof.
//! Auto-reversal schedules two such legs with separate descendant activations.
//! Descendant turn deadlines under ancestor easing need nonlinear projection;
//! they cannot use the endpoint-preservation proof of a single leg.
//!
//! Positive speed is an affine activation clock handled by the scheduler for
//! arbitrary subtrees. Unequal spans, delayed children and internal conditions
//! under easing need nonlinear deadline projection (including irrational
//! inverse times). Reject those at
//! compilation rather than flattening filters or rounding event chronology.
use super::*;
use crate::{
    clock::{BehaviorClock, Easing},
    exact::Ratio,
};

#[derive(Debug, Clone)]
pub(crate) struct ContainerClock {
    duration: Ratio,
    easing: Option<Easing>,
    backwards: bool,
    /// A container's second leg is a real descendant activation, not a leaf
    /// sample trick. This deadline is in the container's incoming parent clock.
    turn_after: Option<Ratio>,
}
impl ContainerClock {
    pub(crate) fn turn_after(&self) -> Option<&Ratio> {
        self.turn_after.as_ref()
    }
    pub(crate) fn sample(
        &self,
        elapsed: &Ratio,
        parent_backwards: bool,
        ended: bool,
        bits: u64,
    ) -> Result<(Ratio, bool), TimelineError> {
        let span = if self.turn_after.is_some() {
            self.duration.mul(&Ratio::integer(2), bits)?
        } else {
            self.duration.clone()
        };
        let mut elapsed = if self.backwards {
            span.sub(elapsed, bits)?
        } else {
            elapsed.clone()
        };
        let mut backwards = parent_backwards ^ self.backwards;
        if self.turn_after.is_some() {
            let side = elapsed.cmp(&self.duration);
            // At the cusp, an active second leg takes its outgoing side. A
            // cutoff at the same instant retains the side it arrived from.
            let reverse = side.is_gt() || side.is_eq() && (backwards == ended);
            if reverse {
                elapsed = span.sub(&elapsed, bits)?;
                backwards = !backwards;
            }
        }
        let elapsed = match &self.easing {
            Some(easing) => easing
                .apply(&elapsed.div(&self.duration, bits)?, bits)?
                .mul(&self.duration, bits)?,
            None => elapsed,
        };
        Ok((elapsed, backwards))
    }
}

pub(super) fn compile(
    t: &Timeline,
    h: &Hierarchy,
    leaves: &[BehaviorClock],
    bits: u64,
    check: &dyn Fn() -> bool,
) -> Result<Vec<Option<Box<ContainerClock>>>, TimelineError> {
    let mut spans: Vec<Option<Ratio>> = vec![None; t.node_count()];
    let mut has_turns = vec![false; t.node_count()];
    let mut clocks = vec![None; t.node_count()];
    for &i in h.traversal.iter().rev() {
        cancel(check)?;
        let Entry::Container(c) = Entry::at(t, i) else {
            spans[i] = leaves[i].parent_duration().cloned();
            continue;
        };
        crate::clock::validate_transform(c.time_transform, &c.id)?;
        let transform = c.time_transform.unwrap_or_default();
        let descendant_turns = h.children[i].iter().any(|&child| has_turns[child]);
        has_turns[i] = transform.auto_reverse || descendant_turns;
        let mut common = None;
        let mut coincident =
            c.navigation.is_none() && (c.kind == ContainerKind::Parallel || c.children.len() <= 1);
        for &child in &h.children[i] {
            cancel(check)?;
            let e = Entry::at(t, child);
            // One zero-offset begin has no second begin instance within an
            // activation, irrespective of its restart admission policy.
            coincident &= e.ends().is_empty()
                && matches!(e.start().conditions(), [TimeCondition::At { offset }] if offset.ticks.get() == 0);
            if let Some(span) = &spans[child] {
                if let Some(previous) = &common {
                    coincident &= span.cmp(previous).is_eq();
                } else {
                    common = Some(span.clone());
                }
            } else {
                coincident = false;
            }
        }
        let common = common.unwrap_or_else(|| Ratio::integer(0));
        coincident &= match c.duration {
            ContainerDuration::Automatic => true,
            ContainerDuration::Fixed { duration } => common.cmp(&Ratio::time(duration)).is_eq(),
            ContainerDuration::Indefinite => false,
        };
        let leg_span = coincident
            .then(|| common.div(&rate(transform.speed_milli_percent, bits)?, bits))
            .transpose()?;
        spans[i] = leg_span
            .as_ref()
            .map(|span| {
                span.mul(
                    &Ratio::integer(if transform.auto_reverse { 2 } else { 1 }),
                    bits,
                )
            })
            .transpose()?;
        let easing = Easing::new(transform, bits)?;
        let backwards = transform.speed_milli_percent < 0;
        if easing.is_some() && descendant_turns {
            return Err(invalid(
                Some(&c.id),
                "descendant reversal turns under easing require nonlinear deadline projection",
            ));
        }
        if easing.is_some() || backwards || transform.auto_reverse {
            if !coincident || !common.positive() {
                return Err(invalid(
                    Some(&c.id),
                    "container easing or reversal requires a coincident finite subtree; general filtered interval projection is not implemented",
                ));
            }
            clocks[i] = Some(Box::new(ContainerClock {
                duration: common,
                easing,
                backwards,
                turn_after: transform
                    .auto_reverse
                    .then(|| leg_span.expect("coincident span")),
            }));
        }
    }
    Ok(clocks)
}

pub(crate) fn rate(speed: i32, bits: u64) -> Result<Ratio, TimelineError> {
    // The causal agenda advances in elapsed time. For admitted reversal clocks,
    // endpoint reflection belongs to the container's simple-time filter.
    Ratio::new(i64::from(speed).abs().into(), 100_000.into(), bits)
}
