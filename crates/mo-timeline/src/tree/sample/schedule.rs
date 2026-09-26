//! Causal once-activation intervals. The agenda consumes actual events; parent
//! termination clips its subtree instead of creating child-begin dependencies.
use super::*;
use std::{cmp::Ordering, collections::BinaryHeap};
mod condition;

#[derive(Clone, PartialEq, Eq)]
struct Pending {
    moment: Moment,
    node: usize,
    end: bool,
    serial: usize,
}
impl Ord for Pending {
    fn cmp(&self, other: &Self) -> Ordering {
        other
            .moment
            .time
            .cmp(&self.moment.time)
            .then_with(|| other.moment.sequence.cmp(&self.moment.sequence))
            .then_with(|| self.end.cmp(&other.end))
            .then_with(|| other.serial.cmp(&self.serial))
    }
}
impl PartialOrd for Pending {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}
struct Scheduler<'a> {
    plan: &'a TimelinePlan,
    h: &'a Hierarchy,
    clicks: &'a Clicks,
    check: &'a dyn Fn() -> bool,
    intervals: Vec<Interval>,
    queued: Vec<bool>,
    closed: Vec<bool>,
    resolved: Vec<Vec<bool>>,
    eligible: Vec<bool>,
    remaining: Vec<usize>,
    child_end: Vec<Moment>,
    agenda: BinaryHeap<Pending>,
    steps: usize,
    serial: usize,
}
pub(super) fn run(
    plan: &TimelinePlan,
    h: &Hierarchy,
    clicks: &Clicks,
    check: &dyn Fn() -> bool,
) -> Result<Vec<Interval>, TimelineError> {
    let count = plan.timeline.node_count();
    let mut s = Scheduler {
        plan,
        h,
        clicks,
        check,
        intervals: vec![
            Interval {
                gate: None,
                start: None,
                natural: None,
                end: None,
                end_sequence: 0,
                natural_sequence: 0
            };
            count
        ],
        queued: vec![false; count],
        closed: vec![false; count],
        resolved: (0..count)
            .map(|i| vec![false; Entry::at(&plan.timeline, i).ends().len()])
            .collect(),
        eligible: vec![false; count],
        remaining: h.children.iter().map(Vec::len).collect(),
        child_end: vec![
            Moment {
                time: Ratio::integer(0),
                sequence: 0
            };
            count
        ],
        agenda: BinaryHeap::new(),
        steps: 0,
        serial: 0,
    };
    for &i in &h.traversal {
        s.step()?;
        if h.parents[i].is_none() {
            s.enable(i)?;
        }
    }
    while let Some(p) = s.agenda.pop() {
        s.step()?;
        if s.closed[p.node] {
            continue;
        }
        if p.end {
            let interval = &s.intervals[p.node];
            if interval
                .natural
                .as_ref()
                .is_some_and(|n| n.cmp(&p.moment.time).is_eq())
                && interval.natural_sequence == p.moment.sequence
            {
                s.finish(p.node, p.moment)?;
            }
        } else {
            s.begin(p.node, p.moment)?;
        }
    }
    for i in 0..count {
        s.step()?;
        if s.intervals[i].start.is_some()
            && !s.resolved[i].is_empty()
            && s.resolved[i].iter().all(|v| *v)
            && !s.eligible[i]
        {
            return Err(invalid(
                Some(Entry::at(&plan.timeline, i).id()),
                "all resolved end conditions precede activation",
            ));
        }
    }
    Ok(s.intervals)
}
impl Scheduler<'_> {
    fn step(&mut self) -> Result<(), TimelineError> {
        cancel(self.check)?;
        if self.steps >= self.plan.limits.max_schedule_steps {
            return Err(TimelineError::Limit("timing schedule work"));
        }
        self.steps += 1;
        Ok(())
    }
    fn bits(&self) -> u64 {
        self.plan.limits.max_exact_bits
    }
    fn push(&mut self, node: usize, moment: Moment, end: bool) -> Result<(), TimelineError> {
        self.step()?;
        self.agenda.push(Pending {
            moment,
            node,
            end,
            serial: self.serial,
        });
        self.serial += 1;
        Ok(())
    }
    fn enable(&mut self, i: usize) -> Result<(), TimelineError> {
        self.step()?;
        if self.closed[i] || self.queued[i] || self.intervals[i].start.is_some() {
            return Ok(());
        }
        let mut gate = if let Some(p) = self.h.parents[i] {
            if self.closed[p] {
                return Ok(());
            }
            let Some(start) = &self.intervals[p].start else {
                return Ok(());
            };
            start.clone()
        } else {
            Moment {
                time: Ratio::integer(0),
                sequence: 0,
            }
        };
        if let Some(p) = self.h.previous[i] {
            let Some(end) = &self.intervals[p].end else {
                return Ok(());
            };
            if end.cmp(&gate.time).is_gt() {
                gate.time = end.clone();
            }
            gate.sequence = gate.sequence.max(self.intervals[p].end_sequence);
        }
        self.intervals[i].gate = Some(gate.clone());
        let entry = Entry::at(&self.plan.timeline, i);
        let candidate = self.condition(i, entry.start(), self.h.dependencies[i], &gate, true)?;
        if let Some(candidate) = candidate {
            self.queued[i] = true;
            self.push(i, candidate, false)?;
        }
        Ok(())
    }
    fn begin(&mut self, i: usize, moment: Moment) -> Result<(), TimelineError> {
        if self.intervals[i].start.is_some() {
            return Ok(());
        }
        self.intervals[i].start = Some(moment.clone());
        let entry = Entry::at(&self.plan.timeline, i);
        let duration = match entry {
            Entry::Leaf(_) => self.plan.clocks[i].parent_duration().cloned(),
            Entry::Container(c) => match c.duration {
                ContainerDuration::Fixed { duration } => Some(Ratio::time(duration)),
                ContainerDuration::Automatic if self.h.children[i].is_empty() => {
                    Some(Ratio::integer(0))
                }
                _ => None,
            },
        };
        self.child_end[i] = moment.clone();
        if let Some(d) = duration {
            self.bound(
                i,
                Moment {
                    time: moment.time.add(&d, self.bits())?,
                    sequence: moment.sequence,
                },
            )?;
        }
        for k in 0..entry.ends().len() {
            self.resolve_end(i, k)?;
        }
        self.notify(i, false)?;
        for &child in &self.h.children[i] {
            self.enable(child)?;
        }
        Ok(())
    }
    fn bound(&mut self, i: usize, moment: Moment) -> Result<(), TimelineError> {
        let interval = &mut self.intervals[i];
        let replace = interval.natural.as_ref().is_none_or(|n| {
            moment.time.cmp(n).is_lt()
                || (moment.time.cmp(n).is_eq() && moment.sequence < interval.natural_sequence)
        });
        if replace {
            interval.natural = Some(moment.time.clone());
            interval.natural_sequence = moment.sequence;
            if !self.closed[i] {
                self.push(i, moment, true)?;
            }
        }
        Ok(())
    }
    fn resolve_end(&mut self, i: usize, k: usize) -> Result<(), TimelineError> {
        self.step()?;
        if self.resolved[i][k] {
            return Ok(());
        }
        let Some(start) = self.intervals[i].start.clone() else {
            return Ok(());
        };
        let condition = &Entry::at(&self.plan.timeline, i).ends()[k];
        let candidate =
            self.condition(i, condition, self.h.end_dependencies[i][k], &start, false)?;
        if let Some(mut candidate) = candidate {
            self.resolved[i][k] = true;
            if candidate.time.cmp(&start.time).is_ge() {
                self.eligible[i] = true;
                candidate.sequence = candidate.sequence.max(start.sequence);
                self.bound(i, candidate)?;
            }
        }
        Ok(())
    }
    fn notify(&mut self, i: usize, end: bool) -> Result<(), TimelineError> {
        for &(dependent, condition) in &self.h.listeners[i * 2 + usize::from(end)] {
            self.step()?;
            if let Some(k) = condition {
                self.resolve_end(dependent, k)?;
            } else {
                self.enable(dependent)?;
            }
        }
        Ok(())
    }
    fn finish(&mut self, i: usize, moment: Moment) -> Result<(), TimelineError> {
        // Mark the entire stopped scope before notifying dependencies. An end
        // emitted by a clipped child may start an outside sibling, never restart
        // another child in the scope that is being terminated.
        let mut stack = vec![i];
        let mut stopped = Vec::new();
        while let Some(node) = stack.pop() {
            self.step()?;
            if self.closed[node] {
                continue;
            }
            self.closed[node] = true;
            if let Some(start) = &self.intervals[node].start {
                self.intervals[node].end_sequence = moment.sequence.max(start.sequence);
                self.intervals[node].end = Some(moment.time.clone());
                stopped.push(node);
            }
            stack.extend(self.h.children[node].iter().rev().copied());
        }
        // Settle all same-event end references before classifying an end as
        // natural or clipped. A child can explicitly end on its parent's end.
        for &node in stopped.iter().rev() {
            self.notify(node, true)?;
            if let Some(next) = self.h.next[node] {
                self.enable(next)?;
            }
        }
        // Children report completion before their automatic parent is considered.
        for node in stopped.into_iter().rev() {
            if let Some(parent) = self.h.parents[node]
                && !self.intervals[node].clipped()
            {
                self.remaining[parent] -= 1;
                let endpoint = self.intervals[node].end.as_ref().expect("ended child");
                if endpoint.cmp(&self.child_end[parent].time).is_gt() {
                    self.child_end[parent].time = endpoint.clone();
                }
                self.child_end[parent].sequence = self.child_end[parent]
                    .sequence
                    .max(self.intervals[node].end_sequence);
                if self.remaining[parent] == 0
                    && matches!(
                        Entry::at(&self.plan.timeline, parent),
                        Entry::Container(TimingContainer {
                            duration: ContainerDuration::Automatic,
                            ..
                        })
                    )
                {
                    self.bound(parent, self.child_end[parent].clone())?;
                }
            }
        }
        Ok(())
    }
}
