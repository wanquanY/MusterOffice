//! Advance one activation's clock while host time stands still. Internal
//! timers run in order, and cross-scope notifications use their actual host
//! instant. Infinite behavior contributes its first cycle, never a fake end.
use super::*;

impl Scheduler<'_> {
    pub(super) fn seek_natural(
        &mut self,
        root: usize,
        mut at: Moment,
    ) -> Result<Moment, TimelineError> {
        let owner = self.intervals.identity(root);
        let mut scope = vec![false; self.h.parents.len()];
        let mut stack = vec![root];
        while let Some(i) = stack.pop() {
            self.step()?;
            scope[i] = true;
            stack.extend(self.h.children[i].iter().copied());
        }
        loop {
            self.step()?;
            if owner != self.intervals.identity(root)
                || self.intervals[root].end.is_some()
                || self.closed[root]
            {
                break;
            }
            let target = self.seek_target(root, &scope, &at)?;
            let mut pending = Vec::new();
            let mut chosen: Option<usize> = None;
            while let Some(p) = self.agenda.pop() {
                self.step()?;
                if !self.valid(&p) {
                    continue;
                }
                let eligible = !matches!(p.action, Action::Input { .. })
                    && (scope[p.node] && !target.precedes(&p.moment)
                        || !scope[p.node] && !at.precedes(&p.moment));
                if eligible && chosen.is_none() {
                    chosen = Some(pending.len());
                }
                pending.push(p);
            }
            let Some(index) = chosen else {
                self.agenda.extend(pending);
                if target.time.cmp(&at.time).is_gt() {
                    let delta = target.time.sub(&at.time, self.bits())?;
                    at = self.advance_clock(root, &scope, &at, &delta)?;
                    continue;
                }
                break;
            };
            let p = pending.swap_remove(index);
            self.agenda.extend(pending);
            if p.moment.time.cmp(&at.time).is_gt() {
                let delta = p.moment.time.sub(&at.time, self.bits())?;
                // Include the selected timer in the same projection as its
                // matching bound, then let ordinary validity checks admit it.
                self.agenda.push(p);
                at = self.advance_clock(root, &scope, &at, &delta)?;
            } else {
                if at.precedes(&p.moment) {
                    at = p.moment.clone();
                }
                self.execute(p)?;
                if let Some(frontier) = &self.seek_frontier
                    && frontier.time.cmp(&at.time).is_eq()
                    && frontier.sequence == at.sequence
                    && at.precedes(frontier)
                {
                    at = frontier.clone();
                }
            }
        }
        Ok(at)
    }

    fn seek_target(
        &mut self,
        root: usize,
        scope: &[bool],
        at: &Moment,
    ) -> Result<Moment, TimelineError> {
        let mut targets: Vec<Option<Moment>> = vec![None; scope.len()];
        // A known delayed activation extends a container's natural position;
        // future host inputs and unknown event-dependent starts do not.
        for p in &self.agenda {
            cancel(self.check)?;
            if !matches!(p.action, Action::Input { .. })
                && scope[p.node]
                && self.valid(p)
                && matches!(p.action, Action::Begin { .. })
                && self.intervals[p.node].start.is_none()
                && targets[p.node]
                    .as_ref()
                    .is_none_or(|old| p.moment.precedes(old))
            {
                targets[p.node] = Some(p.moment.clone());
            }
        }
        for &i in self.h.traversal.iter().rev() {
            self.step()?;
            if !scope[i] || self.closed[i] {
                continue;
            }
            let interval = &self.intervals[i];
            if let Some(end) = &interval.end {
                targets[i] = Some(end.moment.clone());
                continue;
            }
            if interval.start.is_none() {
                continue;
            }
            let target = match Entry::at(&self.plan.timeline, i) {
                Entry::Leaf(_) => {
                    let clock = &self.plan.clocks[i];
                    let duration = match clock.parent_duration() {
                        Some(d) => d.clone(),
                        None => clock.first_cycle_duration(self.bits())?,
                    };
                    let local = interval
                        .origin
                        .as_ref()
                        .expect("active clock")
                        .add(&duration, self.bits())?;
                    Some(
                        self.intervals
                            .project_time(i, &local, at, self.bits(), self.check)?,
                    )
                }
                Entry::Container(_) if self.turn_after(i).is_some() => {
                    interval.natural.as_ref().map(|n| n.moment.clone())
                }
                Entry::Container(_) => self.h.children[i]
                    .iter()
                    .filter_map(|&c| targets[c].clone())
                    .max_by(|a, b| a.compare(b))
                    .or_else(|| interval.natural.as_ref().map(|n| n.moment.clone())),
            };
            targets[i] = match (target, &interval.natural) {
                (Some(target), Some(end)) if end.moment.precedes(&target) => {
                    Some(end.moment.clone())
                }
                (target, _) => target,
            };
        }
        Ok(targets[root]
            .take()
            .filter(|t| at.precedes(t))
            .unwrap_or_else(|| at.clone()))
    }

    fn advance_clock(
        &mut self,
        root: usize,
        scope: &[bool],
        at: &Moment,
        delta: &Ratio,
    ) -> Result<Moment, TimelineError> {
        let mut projected = Vec::new();
        let mut untouched = Vec::new();
        while let Some(p) = self.agenda.pop() {
            self.step()?;
            if !self.valid(&p) {
                continue;
            }
            if !matches!(p.action, Action::Input { .. }) && scope[p.node] {
                let local =
                    self.intervals
                        .local_time(p.node, &p.moment, self.bits(), self.check)?;
                projected.push((p, local));
            } else {
                untouched.push(p);
            }
        }
        let at = Moment {
            order: at.order.add(delta, self.bits())?,
            ..at.clone()
        };
        self.intervals.jump(
            root,
            clocks::Jump {
                at: at.clone(),
                delta: delta.clone(),
            },
        );
        self.seek_frontier = Some(at.clone());
        self.agenda.extend(untouched);
        for (mut p, local) in projected {
            self.step()?;
            p.moment = self
                .intervals
                .project_time(p.node, &local, &at, self.bits(), self.check)?;
            self.agenda.push(p);
        }
        for (i, inside) in scope.iter().enumerate() {
            self.step()?;
            if !inside || self.intervals[i].end.is_some() {
                continue;
            }
            let bits = self.bits();
            for bound in [&mut self.declared_bound[i], &mut self.automatic_bound[i]]
                .into_iter()
                .flatten()
            {
                bound.moment =
                    self.intervals
                        .project_time(i, &bound.local, &at, bits, self.check)?;
            }
            if let Some(mut bound) = self.intervals[i].natural.clone() {
                bound.moment =
                    self.intervals
                        .project_time(i, &bound.local, &at, bits, self.check)?;
                self.intervals[i].natural = Some(bound);
            }
        }
        Ok(at)
    }
}
