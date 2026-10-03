//! Activation identity is an arena index, scoped to its exact parent activation.
//! Historical children cannot leak into a later activation of the same parent.
use super::*;
use std::ops::{Index, IndexMut};

pub(super) struct Record {
    pub(super) parent: Option<usize>,
    /// This activation's outgoing speed magnitude, applied only to descendants. Its own
    /// begin/end conditions are still measured in the receiving parent clock.
    /// The signed declaration is retained; the admitted container filter owns
    /// reversal while scheduling uses the magnitude of this value.
    pub(super) child_speed: i32,
    pub(super) interval: Interval,
}
pub(crate) struct Intervals {
    pub(super) presentation_step: Option<PresentationStepReceipt>,
    records: Vec<Record>,
    current: Vec<usize>,
    by_node: Vec<Vec<usize>>,
    horizon: Option<RationalTime>,
    has_jumps: bool,
    has_rates: bool,
}
pub(super) struct Selection<'a> {
    records: &'a [Record],
    ids: Vec<Option<usize>>,
    waiting: Interval,
    has_jumps: bool,
    has_rates: bool,
}
impl Selection<'_> {
    pub(super) fn len(&self) -> usize {
        self.ids.len()
    }
    pub(super) fn local_time(
        &self,
        i: usize,
        time: &Ratio,
        point: Option<&Moment>,
        bits: u64,
        check: &dyn Fn() -> bool,
    ) -> Result<Ratio, TimelineError> {
        if !self.has_jumps && !self.has_rates {
            return Ok(time.clone());
        }
        clocks::local_time(self.records, self.ids[i], time, point, bits, check)
    }
}
impl Index<usize> for Selection<'_> {
    type Output = Interval;
    fn index(&self, i: usize) -> &Interval {
        self.ids[i].map_or(&self.waiting, |id| &self.records[id].interval)
    }
}
impl Index<usize> for Intervals {
    type Output = Interval;
    fn index(&self, i: usize) -> &Interval {
        &self.records[self.current[i]].interval
    }
}
impl IndexMut<usize> for Intervals {
    fn index_mut(&mut self, i: usize) -> &mut Interval {
        &mut self.records[self.current[i]].interval
    }
}
impl Intervals {
    pub(super) fn local_time(
        &self,
        i: usize,
        point: &Moment,
        bits: u64,
        check: &dyn Fn() -> bool,
    ) -> Result<Ratio, TimelineError> {
        if !self.has_jumps && !self.has_rates {
            return Ok(point.time.clone());
        }
        clocks::local_time(
            &self.records,
            Some(self.current[i]),
            &point.time,
            Some(point),
            bits,
            check,
        )
    }
    pub(super) fn project_time(
        &self,
        i: usize,
        local: &Ratio,
        reference: &Moment,
        bits: u64,
        check: &dyn Fn() -> bool,
    ) -> Result<Moment, TimelineError> {
        if !self.has_jumps && !self.has_rates {
            return Ok(Moment {
                time: local.clone(),
                ..reference.clone()
            });
        }
        clocks::project_time(
            &self.records,
            self.current[i],
            local,
            reference,
            bits,
            check,
        )
    }
    pub(super) fn jump(&mut self, i: usize, jump: clocks::Jump) {
        self.has_jumps = true;
        self[i].jumps.push(jump);
    }
    pub(super) fn new(plan: &TimelinePlan, at: RationalTime) -> Result<Self, TimelineError> {
        let n = plan.timeline.node_count();
        if n > plan.limits.max_intervals {
            return Err(TimelineError::Limit("timing interval count"));
        }
        Ok(Self {
            presentation_step: None,
            records: plan
                .hierarchy
                .parents
                .iter()
                .enumerate()
                .map(|(i, &parent)| Record {
                    parent,
                    child_speed: match Entry::at(&plan.timeline, i) {
                        Entry::Container(c) => {
                            c.time_transform.unwrap_or_default().speed_milli_percent
                        }
                        Entry::Leaf(_) => 100_000,
                    },
                    interval: Interval::default(),
                })
                .collect(),
            current: (0..n).collect(),
            by_node: (0..n).map(|i| vec![i]).collect(),
            horizon: plan.restarting.then_some(at.normalized()),
            has_jumps: false,
            has_rates: plan.timeline.tree.as_ref().is_some_and(|t| {
                t.containers
                    .iter()
                    .any(|c| c.time_transform.unwrap_or_default().speed_milli_percent != 100_000)
            }),
        })
    }
    pub(crate) fn len(&self) -> usize {
        self.records.len()
    }
    pub(crate) fn covers(&self, at: RationalTime) -> bool {
        self.horizon.is_none_or(|h| h.compare_time(at).is_eq())
    }
    pub(super) fn identity(&self, i: usize) -> usize {
        self.current[i]
    }
    pub(super) fn replace(
        &mut self,
        i: usize,
        parent: Option<usize>,
        limit: usize,
    ) -> Result<(), TimelineError> {
        if self.records.len() >= limit {
            return Err(TimelineError::Limit("timing interval count"));
        }
        let id = self.records.len();
        self.records.push(Record {
            parent,
            child_speed: self.records[self.current[i]].child_speed,
            interval: Interval::default(),
        });
        self.by_node[i].push(id);
        self.current[i] = id;
        Ok(())
    }
    pub(super) fn parent(&self, i: usize) -> Option<usize> {
        self.records[self.current[i]].parent
    }
    pub(super) fn select(
        &self,
        plan: &TimelinePlan,
        at: RationalTime,
        check: &dyn Fn() -> bool,
    ) -> Result<Selection<'_>, TimelineError> {
        let h = &plan.hierarchy;
        let mut ids = vec![None; h.parents.len()];
        let time = Ratio::time(at);
        for &i in &h.traversal {
            let parent = h.parents[i].and_then(|p| ids[p]);
            let mut best = None;
            for &id in &self.by_node[i] {
                cancel(check)?;
                let record = &self.records[id];
                if record.parent != parent {
                    continue;
                }
                let candidate = &record.interval;
                let replace = best.is_none_or(|old: usize| {
                    let old = &self.records[old].interval;
                    match (
                        old.reset_at.as_ref().or(old.start.as_ref()),
                        candidate.reset_at.as_ref().or(candidate.start.as_ref()),
                    ) {
                        (None, _) => true,
                        (_, None) => false,
                        (Some(a), Some(b)) => {
                            if b.time.cmp(&time).is_le() {
                                !b.precedes(a) || a.time.cmp(&time).is_gt()
                            } else {
                                a.time.cmp(&time).is_gt() && b.precedes(a)
                            }
                        }
                    }
                });
                if replace {
                    best = Some(id);
                }
            }
            ids[i] = best;
        }
        Ok(Selection {
            records: &self.records,
            ids,
            waiting: Interval::default(),
            has_jumps: self.has_jumps,
            has_rates: self.has_rates,
        })
    }
}
