//! Entry admission and ancestor lifecycle transitions. Completion and restart
//! share subtree termination, but only completion advances a sequence/parent.
use super::*;
impl Scheduler<'_> {
    pub(super) fn begin(&mut self, i: usize, moment: Moment) -> Result<(), TimelineError> {
        self.step()?;
        let entry = Entry::at(&self.plan.timeline, i);
        if let Some(parent) = self.h.parents[i]
            && let Some(nav) = Entry::at(&self.plan.timeline, parent).navigation()
            && !nav.concurrent
            && self.intervals[parent].position != Some(self.h.positions[i])
        {
            return Ok(());
        }
        if let Some(previous) = &self.intervals[i].start {
            if !previous.precedes(&moment) || entry.restart().is_never() {
                return Ok(());
            }
            let active = self.intervals[i].end.is_none();
            if active && entry.restart() == RestartMode::WhenNotActive {
                return Ok(());
            }
            self.validate_ends(i)?;
            if active {
                self.finish(i, moment.clone(), true)?;
            }
            let gate = self.intervals[i].gate.clone();
            self.intervals
                .replace(i, self.intervals.parent(i), self.plan.limits.max_intervals)?;
            self.intervals[i].gate = gate;
            self.intervals[i].reset = Some(self.event_serial);
            self.resolved[i].fill(false);
            self.eligible[i] = false;
            self.declared_bound[i] = None;
            self.automatic_bound[i] = None;
        }
        self.intervals[i].origin =
            Some(
                self.intervals
                    .local_time(i, &moment, self.bits(), self.check)?,
            );
        self.intervals[i].start = Some(moment.clone());
        self.intervals[i].child_start = Some(moment.clone());
        if entry.navigation().is_some() {
            self.intervals[i].position = Some(0);
            self.intervals[i].position_at = Some(moment.clone());
        }
        self.remaining[i] = self.h.children[i].len();
        self.child_end[i] = moment.clone();
        if self.reported[i] {
            if let Some(parent) = self.h.parents[i] {
                self.remaining[parent] += 1;
                self.automatic_bound[parent] = None;
                self.refresh_bound(parent)?;
            }
            self.reported[i] = false;
        }
        if let Some(d) = self.duration(i)? {
            self.bound(i, self.delay(i, &moment, &d)?)?;
        }
        if let Some(turn) = self.turn_after(i) {
            let at = self.delay(i, &moment, turn)?;
            self.push(
                i,
                at,
                Action::Turn {
                    interval: self.intervals.identity(i),
                },
            )?;
        }
        self.reset_children(i)?;
        for k in 0..entry.ends().len() {
            self.resolve_end(i, k)?;
        }
        self.notify(i, false)?;
        self.resolve_navigation(i)?;
        for &child in &self.h.children[i] {
            self.enable(child)?;
        }
        Ok(())
    }
    pub(super) fn reset_children(&mut self, i: usize) -> Result<(), TimelineError> {
        self.reset_descendants(i, None)
    }
    fn reset_descendants(&mut self, i: usize, leg: Option<&Moment>) -> Result<(), TimelineError> {
        let mut stack: Vec<_> = self.h.children[i].iter().rev().copied().collect();
        while let Some(child) = stack.pop() {
            self.step()?;
            let parent = self.h.parents[child].map(|p| self.intervals.identity(p));
            if leg.is_some() || self.intervals.parent(child) != parent {
                self.intervals
                    .replace(child, parent, self.plan.limits.max_intervals)?;
                self.intervals[child].reset = Some(self.event_serial);
                self.intervals[child].reset_at = leg.cloned();
                self.epochs[child] += 1;
                self.closed[child] = false;
                self.muted[child] = false;
                self.resolved[child].fill(false);
                self.eligible[child] = false;
                self.reported[child] = false;
                self.remaining[child] = self.h.children[child].len();
                self.declared_bound[child] = None;
                self.automatic_bound[child] = None;
                self.events[child * 2] = None;
                self.events[child * 2 + 1] = None;
                stack.extend(self.h.children[child].iter().rev().copied());
            }
        }
        Ok(())
    }
    pub(super) fn turn(&mut self, i: usize, moment: Moment) -> Result<(), TimelineError> {
        self.step()?;
        // Child end actions and their dependent cancellations precede the turn
        // at the same instant. The container itself has not ended or restarted.
        self.intervals[i].turned = true;
        self.intervals[i].child_start = Some(moment.clone());
        self.remaining[i] = self.h.children[i].len();
        self.child_end[i] = moment.clone();
        self.reset_descendants(i, Some(&moment))?;
        for &child in &self.h.children[i] {
            self.enable(child)?;
        }
        Ok(())
    }
    pub(super) fn finish(
        &mut self,
        i: usize,
        moment: Moment,
        restarting: bool,
    ) -> Result<(), TimelineError> {
        // Close descendants before publishing any end. An end from a clipped
        // child cannot activate a sibling in the subtree being terminated.
        let mut stack = vec![i];
        let mut stopped = Vec::new();
        while let Some(node) = stack.pop() {
            self.step()?;
            if self.closed[node] {
                continue;
            }
            if node != i {
                self.closed[node] = true;
            }
            if self.intervals[node].start.is_some() && self.intervals[node].end.is_none() {
                self.intervals[node].end = Some(self.point(node, moment.clone())?);
                stopped.push(node);
            }
            stack.extend(self.h.children[node].iter().rev().copied());
        }
        for &node in stopped.iter().rev() {
            self.notify(node, true)?;
            if !restarting {
                self.advance_finished(node, &moment)?;
            }
        }
        for node in stopped.into_iter().rev() {
            self.validate_ends(node)?;
            if restarting && node == i {
                continue;
            }
            if let Some(parent) = self.h.parents[node]
                && !self.reported[node]
                && !self.intervals[node].clipped()
            {
                self.reported[node] = true;
                self.remaining[parent] -= 1;
                let endpoint = &self.intervals[node]
                    .end
                    .as_ref()
                    .expect("ended child")
                    .moment;
                if self.child_end[parent].precedes(endpoint) {
                    self.child_end[parent] = endpoint.clone();
                }
                if self.remaining[parent] == 0
                    && self.turn_after(parent).is_none()
                    && matches!(
                        Entry::at(&self.plan.timeline, parent),
                        Entry::Container(TimingContainer {
                            duration: ContainerDuration::Automatic,
                            ..
                        })
                    )
                {
                    self.automatic_bound[parent] =
                        Some(self.point(parent, self.child_end[parent].clone())?);
                    self.refresh_bound(parent)?;
                }
            }
        }
        Ok(())
    }
}
