//! One causal agenda for first activation, reactivation and scoped termination.
//! Epochs invalidate delayed triggers when a parent restarts; arena identities
//! invalidate stale ends without discarding the intervals needed by sampling.
use super::*;
use std::{cmp::Ordering, collections::BinaryHeap};
mod condition;
mod lifecycle;
mod navigation;
mod seek;

#[derive(Clone)]
struct Occurred {
    moment: Moment,
    serial: usize,
}

#[derive(Clone, PartialEq, Eq)]
enum Action {
    Begin {
        owner: Option<usize>,
    },
    End {
        interval: usize,
    },
    Turn {
        interval: usize,
    },
    Input {
        event: InputEvent,
    },
    Navigate {
        direction: NavigationDirection,
        owner: usize,
    },
}
impl Action {
    fn priority(&self) -> u8 {
        match self {
            Self::End { .. } => 4,
            Self::Turn { .. } => 3,
            Self::Input { .. } => 2,
            Self::Navigate { .. } => 1,
            Self::Begin { .. } => 0,
        }
    }
}
#[derive(Clone, PartialEq, Eq)]
struct Pending {
    moment: Moment,
    node: usize,
    epoch: usize,
    action: Action,
    serial: usize,
}
impl Ord for Pending {
    fn cmp(&self, other: &Self) -> Ordering {
        other
            .moment
            .compare(&self.moment)
            // Event admission decides begin versus end before either is queued.
            // Termination then closes a scope before further children begin.
            .then_with(|| self.action.priority().cmp(&other.action.priority()))
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
    check: &'a dyn Fn() -> bool,
    intervals: Intervals,
    epochs: Vec<usize>,
    closed: Vec<bool>,
    muted: Vec<bool>,
    resolved: Vec<Vec<bool>>,
    eligible: Vec<bool>,
    remaining: Vec<usize>,
    reported: Vec<bool>,
    child_end: Vec<Moment>,
    declared_bound: Vec<Option<Bound>>,
    automatic_bound: Vec<Option<Bound>>,
    events: Vec<Option<Occurred>>,
    event_serial: usize,
    agenda: BinaryHeap<Pending>,
    steps: usize,
    serial: usize,
    seek_frontier: Option<Moment>,
}
pub(super) fn run(
    plan: &TimelinePlan,
    h: &Hierarchy,
    inputs: &Inputs,
    at: RationalTime,
    check: &dyn Fn() -> bool,
) -> Result<Intervals, TimelineError> {
    let count = plan.timeline.node_count();
    let zero = Moment::at(Ratio::integer(0), 0);
    let mut s = Scheduler {
        plan,
        h,
        check,
        intervals: Intervals::new(plan, at)?,
        epochs: vec![0; count],
        closed: vec![false; count],
        muted: vec![false; count],
        resolved: (0..count)
            .map(|i| vec![false; Entry::at(&plan.timeline, i).ends().len()])
            .collect(),
        eligible: vec![false; count],
        remaining: h.children.iter().map(Vec::len).collect(),
        reported: vec![false; count],
        child_end: vec![zero; count],
        declared_bound: vec![None; count],
        automatic_bound: vec![None; count],
        events: vec![None; count * 2],
        event_serial: 0,
        agenda: BinaryHeap::new(),
        steps: 0,
        serial: 0,
        seek_frontier: None,
    };
    for &i in &h.traversal {
        s.step()?;
        if h.parents[i].is_none() {
            s.enable(i)?;
        }
    }
    for (event, events) in inputs {
        if !h.input_listeners.contains_key(event) {
            continue;
        }
        for (time, sequence) in events {
            s.push(
                usize::MAX,
                Moment::at(time.clone(), *sequence),
                Action::Input {
                    event: event.clone(),
                },
            )?;
        }
    }
    let horizon = Ratio::time(at);
    while let Some(p) = s.agenda.pop() {
        s.step()?;
        if let Action::Input { event } = &p.action {
            s.next_event()?;
            s.dispatch(&h.input_listeners[event], &p.moment, true)?;
            continue;
        }
        if s.closed[p.node] || s.epochs[p.node] != p.epoch {
            continue;
        }
        if plan.restarting && p.moment.time.cmp(&horizon).is_gt() {
            s.agenda.push(p);
            break;
        }
        s.execute(p)?;
    }
    for i in 0..count {
        s.validate_ends(i)?;
    }
    // Future bounds describe the currently known interval, without firing its
    // end or synthesizing future feedback. A different horizon rebuilds it.
    if plan.restarting {
        s.project()?;
    }
    Ok(s.intervals)
}
impl Scheduler<'_> {
    fn point(&self, i: usize, moment: Moment) -> Result<Bound, TimelineError> {
        let local = self
            .intervals
            .local_time(i, &moment, self.bits(), self.check)?;
        Ok(Bound { moment, local })
    }
    fn delay(&self, i: usize, source: &Moment, delay: &Ratio) -> Result<Moment, TimelineError> {
        if !delay.positive() {
            return Ok(source.clone());
        }
        let local = self
            .intervals
            .local_time(i, source, self.bits(), self.check)?
            .add(delay, self.bits())?;
        self.intervals
            .project_time(i, &local, source, self.bits(), self.check)
    }
    fn valid(&self, p: &Pending) -> bool {
        if matches!(p.action, Action::Input { .. }) {
            return true;
        }
        if self.closed[p.node] || self.epochs[p.node] != p.epoch {
            return false;
        }
        match p.action {
            Action::End { interval } => {
                interval == self.intervals.identity(p.node)
                    && self.intervals[p.node].end.is_none()
                    && self.intervals[p.node]
                        .natural
                        .as_ref()
                        .is_some_and(|n| n.moment == p.moment)
            }
            Action::Turn { interval } => {
                interval == self.intervals.identity(p.node)
                    && !self.intervals[p.node].turned
                    && self.intervals[p.node].start.is_some()
                    && self.intervals[p.node].end.is_none()
            }
            Action::Begin { owner } => owner.is_none_or(|id| id == self.intervals.identity(p.node)),
            Action::Navigate { owner, .. } => owner == self.intervals.identity(p.node),
            Action::Input { .. } => true,
        }
    }
    fn execute(&mut self, p: Pending) -> Result<(), TimelineError> {
        if !self.valid(&p) {
            return Ok(());
        }
        match p.action {
            Action::End { .. } => self.finish(p.node, p.moment, false),
            Action::Turn { .. } => self.turn(p.node, p.moment),
            Action::Begin { .. } => self.begin(p.node, p.moment),
            Action::Navigate { direction, .. } => self.navigate(p.node, direction, p.moment),
            Action::Input { .. } => unreachable!("input admission precedes execution"),
        }
    }
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
    fn push(&mut self, node: usize, moment: Moment, action: Action) -> Result<(), TimelineError> {
        self.step()?;
        self.agenda.push(Pending {
            moment,
            node,
            action,
            epoch: if node == usize::MAX {
                0
            } else {
                self.epochs[node]
            },
            serial: self.serial,
        });
        self.serial += 1;
        Ok(())
    }
    fn enable(&mut self, i: usize) -> Result<(), TimelineError> {
        self.step()?;
        if self.closed[i] || self.intervals[i].gate.is_some() {
            return Ok(());
        }
        let mut gate = if let Some(p) = self.h.parents[i] {
            if self.closed[p] || self.intervals[p].end.is_some() {
                return Ok(());
            }
            let Some(start) = &self.intervals[p].start else {
                return Ok(());
            };
            self.intervals[p]
                .child_start
                .as_ref()
                .unwrap_or(start)
                .clone()
        } else {
            Moment::at(Ratio::integer(0), 0)
        };
        if let Some(parent) = self.h.parents[i]
            && Entry::at(&self.plan.timeline, parent)
                .navigation()
                .is_some()
        {
            if self.intervals[parent].position != Some(self.h.positions[i]) {
                return Ok(());
            }
            gate = self.intervals[parent]
                .position_at
                .clone()
                .expect("active sequence cursor");
        } else if let Some(p) = self.h.previous[i] {
            let Some(end) = &self.intervals[p].end else {
                return Ok(());
            };
            if gate.precedes(&end.moment) {
                gate = end.moment.clone();
            }
        }
        self.intervals[i].gate = Some(gate);
        for k in 0..Entry::at(&self.plan.timeline, i).start().conditions().len() {
            self.resolve_start(i, k)?;
        }
        Ok(())
    }
    fn duration(&self, i: usize) -> Result<Option<Ratio>, TimelineError> {
        if let Some(turn) = self.turn_after(i) {
            return Ok(Some(turn.mul(&Ratio::integer(2), self.bits())?));
        }
        Ok(match Entry::at(&self.plan.timeline, i) {
            Entry::Leaf(_) => self.plan.clocks[i].parent_duration().cloned(),
            Entry::Container(c) => match c.duration {
                ContainerDuration::Fixed { duration } => Some(Ratio::time(duration).div(
                    &super::super::container_clock::rate(
                        c.time_transform.unwrap_or_default().speed_milli_percent,
                        self.bits(),
                    )?,
                    self.bits(),
                )?),
                ContainerDuration::Automatic if self.h.children[i].is_empty() => {
                    Some(Ratio::integer(0))
                }
                _ => None,
            },
        })
    }
    fn turn_after(&self, i: usize) -> Option<&Ratio> {
        self.plan.container_clocks[i]
            .as_ref()
            .and_then(|c| c.turn_after())
    }
    fn bound(&mut self, i: usize, moment: Moment) -> Result<(), TimelineError> {
        let bound = self.point(i, moment)?;
        if self.declared_bound[i].as_ref().is_none_or(|b| {
            bound
                .local
                .cmp(&b.local)
                .then_with(|| bound.moment.compare(&b.moment))
                .is_lt()
        }) {
            self.declared_bound[i] = Some(bound);
            self.refresh_bound(i)?;
        }
        Ok(())
    }
    fn refresh_bound(&mut self, i: usize) -> Result<(), TimelineError> {
        let earliest = self.declared_bound[i]
            .iter()
            .chain(self.automatic_bound[i].iter())
            .min_by(|a, b| {
                a.local
                    .cmp(&b.local)
                    .then_with(|| a.moment.compare(&b.moment))
            })
            .cloned();
        let changed = self.intervals[i].natural.as_ref().map(|b| &b.moment)
            != earliest.as_ref().map(|b| &b.moment);
        if changed {
            self.intervals[i].natural = earliest.clone();
            if let Some(bound) = earliest
                && !self.closed[i]
                && self.intervals[i].end.is_none()
            {
                self.push(
                    i,
                    bound.moment,
                    Action::End {
                        interval: self.intervals.identity(i),
                    },
                )?;
            }
        }
        Ok(())
    }
    fn notify(&mut self, i: usize, end: bool) -> Result<(), TimelineError> {
        let event = i * 2 + usize::from(end);
        let moment = if end {
            self.intervals[i].end.as_ref().map(|b| b.moment.clone())
        } else {
            self.intervals[i].start.clone()
        };
        if let Some(moment) = moment {
            let serial = self.next_event()?;
            self.events[event] = Some(Occurred {
                moment: moment.clone(),
                serial,
            });
            self.dispatch(&self.h.listeners[event], &moment, false)?;
        }
        Ok(())
    }
    fn next_event(&mut self) -> Result<usize, TimelineError> {
        self.event_serial = self
            .event_serial
            .checked_add(1)
            .ok_or(TimelineError::Limit("timing event count"))?;
        Ok(self.event_serial)
    }

    fn validate_ends(&mut self, i: usize) -> Result<(), TimelineError> {
        self.step()?;
        if self.intervals[i].start.is_some()
            && !self.resolved[i].is_empty()
            && self.resolved[i].iter().all(|v| *v)
            && !self.eligible[i]
        {
            return Err(invalid(
                Some(Entry::at(&self.plan.timeline, i).id()),
                "all resolved end conditions precede activation",
            ));
        }
        Ok(())
    }
    fn project(&mut self) -> Result<(), TimelineError> {
        // The heap is ordered. Only the earliest known future begin of an
        // unstarted entry is exposed; subsequent restarts stay unevaluated.
        while let Some(p) = self.agenda.pop() {
            self.step()?;
            if self.closed[p.node] || self.epochs[p.node] != p.epoch {
                continue;
            }
            if let Action::Begin { owner } = p.action
                && owner.is_none_or(|id| id == self.intervals.identity(p.node))
                && self.intervals[p.node].start.is_none()
            {
                let mut parent = self.h.parents[p.node];
                let mut excluded = false;
                while let Some(i) = parent {
                    self.step()?;
                    if self.intervals[i]
                        .end
                        .as_ref()
                        .or(self.intervals[i].natural.as_ref())
                        .is_some_and(|end| !p.moment.precedes(&end.moment))
                    {
                        excluded = true;
                        break;
                    }
                    parent = self.h.parents[i];
                }
                if excluded {
                    continue;
                }
                self.intervals[p.node].origin =
                    Some(
                        self.intervals
                            .local_time(p.node, &p.moment, self.bits(), self.check)?,
                    );
                self.intervals[p.node].start = Some(p.moment.clone());
                if let Some(d) = self.duration(p.node)? {
                    self.bound(p.node, self.delay(p.node, &p.moment, &d)?)?;
                }
                for k in 0..Entry::at(&self.plan.timeline, p.node).ends().len() {
                    self.resolve_end(p.node, k)?;
                }
            }
        }
        for &i in &self.h.traversal {
            self.step()?;
            if self.intervals[i].start.is_none() || self.intervals[i].end.is_some() {
                continue;
            }
            let parent = self.h.parents[i].and_then(|p| self.intervals[p].end.clone());
            let natural = self.intervals[i].natural.clone();
            self.intervals[i].end = match (natural, parent) {
                (Some(n), Some(p)) => Some(if p.moment.precedes(&n.moment) {
                    self.point(i, p.moment)?
                } else {
                    n
                }),
                (Some(n), None) => Some(n),
                (None, Some(p)) => Some(self.point(i, p.moment)?),
                (None, None) => None,
            };
        }
        Ok(())
    }
}
