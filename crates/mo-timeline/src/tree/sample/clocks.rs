//! Affine clocks owned by activation identities, with exact subtree seeks.
//! Jumps use unscaled host-time distance so their causal order remains common
//! across differently paced branches. Rates and origins map that coordinate to
//! the receiving parent's simple time; no synthetic host input is introduced.
use super::{intervals::Record, *};

#[derive(Clone)]
pub(super) struct Jump {
    pub(super) at: Moment,
    pub(super) delta: Ratio,
}
impl Jump {
    fn before(&self, bits: u64) -> Result<Moment, TimelineError> {
        Ok(Moment {
            order: self.at.order.sub(&self.delta, bits)?,
            ..self.at.clone()
        })
    }
    fn contribution(
        &self,
        time: &Ratio,
        point: Option<&Moment>,
        bits: u64,
    ) -> Result<Ratio, TimelineError> {
        if let Some(point) = point {
            if !point.precedes(&self.at) {
                return Ok(self.delta.clone());
            }
            let before = self.before(bits)?;
            if before.precedes(point) {
                return point.order.sub(&before.order, bits);
            }
        } else if time.cmp(&self.at.time).is_ge() {
            return Ok(self.delta.clone());
        }
        Ok(Ratio::integer(0))
    }
}
pub(super) fn local_time(
    records: &[Record],
    mut id: Option<usize>,
    time: &Ratio,
    point: Option<&Moment>,
    bits: u64,
    check: &dyn Fn() -> bool,
) -> Result<Ratio, TimelineError> {
    let (rate, offset) = affine(records, id, bits, check)?;
    let mut local = time.clone();
    while let Some(i) = id {
        cancel(check)?;
        for jump in &records[i].interval.jumps {
            cancel(check)?;
            local = local.add(&jump.contribution(time, point, bits)?, bits)?;
        }
        id = records[i].parent;
    }
    local.mul(&rate, bits)?.add(&offset, bits)
}
pub(super) fn project_time(
    records: &[Record],
    id: usize,
    local: &Ratio,
    reference: &Moment,
    bits: u64,
    check: &dyn Fn() -> bool,
) -> Result<Moment, TimelineError> {
    let (rate, offset) = affine(records, Some(id), bits, check)?;
    let local = local.sub(&offset, bits)?.div(&rate, bits)?;
    let mut id = Some(id);
    let mut jumps = Vec::new();
    while let Some(i) = id {
        cancel(check)?;
        for jump in &records[i].interval.jumps {
            cancel(check)?;
            jumps.push(jump);
        }
        id = records[i].parent;
    }
    jumps.sort_by(|a, b| a.at.compare(&b.at));
    let mut offset = Ratio::integer(0);
    for jump in jumps {
        cancel(check)?;
        let left = jump.at.time.add(&offset, bits)?;
        let right = left.add(&jump.delta, bits)?;
        if local.cmp(&left).is_ge() && local.cmp(&right).is_le() {
            let mut moment = jump.before(bits)?;
            moment.order = moment.order.add(&local.sub(&left, bits)?, bits)?;
            return Ok(moment);
        }
        if local.cmp(&left).is_lt() {
            break;
        }
        offset = offset.add(&jump.delta, bits)?;
    }
    let time = local.sub(&offset, bits)?;
    // Zero-delay causality retains its exact position; a future host instant
    // starts a fresh within-instant clock coordinate.
    Ok(Moment {
        order: if time.cmp(&reference.time).is_eq() {
            reference.order.clone()
        } else {
            Ratio::integer(0)
        },
        time,
        sequence: reference.sequence,
    })
}

/// incoming(child) = origin(parent) + abs(speed(parent)) *
/// (incoming(parent) - origin(parent)). Compose from the inner activation out,
/// preserving each activation's anchor, including after restart and seek.
/// For admitted coincident reversal, this is chronological elapsed time;
/// simple-time reflection preserves interval endpoints and is sampled separately.
fn affine(
    records: &[Record],
    id: Option<usize>,
    bits: u64,
    check: &dyn Fn() -> bool,
) -> Result<(Ratio, Ratio), TimelineError> {
    let mut rate = Ratio::integer(1);
    let mut offset = Ratio::integer(0);
    let mut parent = id.and_then(|i| records[i].parent);
    while let Some(i) = parent {
        cancel(check)?;
        let record = &records[i];
        if record.child_speed != 100_000 {
            let speed = super::super::container_clock::rate(record.child_speed, bits)?;
            let origin =
                record.interval.origin.as_ref().ok_or_else(|| {
                    invalid(None, "paced parent clock requires an activation origin")
                })?;
            offset = offset.add(
                &rate
                    .mul(&Ratio::integer(1).sub(&speed, bits)?, bits)?
                    .mul(origin, bits)?,
                bits,
            )?;
            rate = rate.mul(&speed, bits)?;
        }
        parent = record.parent;
    }
    Ok((rate, offset))
}
