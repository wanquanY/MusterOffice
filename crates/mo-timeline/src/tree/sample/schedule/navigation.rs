//! A sequence owns its cursor; advancing an old concurrent child cannot move it.
//! Backward movement resets records, not wall time or the host's event history.
use super::*;
impl Scheduler<'_> {
    pub(super) fn resolve_navigation(&mut self, i: usize) -> Result<(), TimelineError> {
        let entry = Entry::at(&self.plan.timeline, i);
        if entry.navigation().is_none() {
            return Ok(());
        }
        let start = self.intervals[i].start.clone().expect("active sequence");
        for (index, atom) in entry
            .conditions()
            .filter(|(k, _)| matches!(k, ConditionIndex::Next(_) | ConditionIndex::Previous(_)))
        {
            self.step()?;
            if let Some(moment) = self.condition(
                i,
                atom,
                self.h.dependencies[i].get(&index).copied(),
                &start,
                true,
            )? {
                let direction = if matches!(index, ConditionIndex::Next(_)) {
                    NavigationDirection::Next
                } else {
                    NavigationDirection::Previous
                };
                self.push(
                    i,
                    moment,
                    Action::Navigate {
                        direction,
                        owner: self.intervals.identity(i),
                    },
                )?;
            }
        }
        Ok(())
    }
    pub(super) fn advance_finished(
        &mut self,
        child: usize,
        moment: &Moment,
    ) -> Result<(), TimelineError> {
        if let Some(parent) = self.h.parents[child]
            && Entry::at(&self.plan.timeline, parent)
                .navigation()
                .is_some()
        {
            if self.closed[parent]
                || self.intervals[parent].end.is_some()
                || self.intervals[parent].position != Some(self.h.positions[child])
            {
                return Ok(());
            }
            let position = self.h.positions[child] + 1;
            self.move_cursor(parent, position, moment);
            if let Some(&next) = self.h.children[parent].get(position) {
                self.enable(next)?;
            }
        } else if let Some(next) = self.h.next[child] {
            self.enable(next)?;
        }
        Ok(())
    }
    fn move_cursor(&mut self, i: usize, position: usize, moment: &Moment) {
        self.intervals[i].position = Some(position);
        self.intervals[i].position_at = Some(moment.clone());
    }
    fn presentation_sequence(&self, i: usize) -> bool {
        matches!(
            Entry::at(&self.plan.timeline, i),
            Entry::Container(TimingContainer {
                presentation: Some(PresentationRole::MainSequence),
                ..
            })
        )
    }
    /// One cursor decision shared by input admission and actual navigation.
    /// Delayed actions recompute against their owning activation when executed.
    pub(super) fn navigation_position(
        &mut self,
        i: usize,
        direction: NavigationDirection,
    ) -> Result<Option<usize>, TimelineError> {
        if self.closed[i] || self.intervals[i].end.is_some() || self.intervals[i].start.is_none() {
            return Ok(None);
        }
        let current = self.intervals[i].position.expect("sequence cursor");
        let children = &self.h.children[i];
        if direction == NavigationDirection::Next {
            return Ok(children.get(current).map(|&child| {
                if self.intervals[child].start.is_none() {
                    current
                } else {
                    current + 1
                }
            }));
        }
        let active_current = self.presentation_sequence(i)
            && children
                .get(current)
                .is_some_and(|&child| self.intervals[child].start.is_some());
        let Some(mut previous) = (if active_current {
            Some(current)
        } else {
            current.checked_sub(1)
        }) else {
            return Ok(None);
        };
        let nav = Entry::at(&self.plan.timeline, i)
            .navigation()
            .expect("sequence navigation");
        if nav.previous_action == PreviousAction::SkipTimed {
            while previous > 0 {
                self.step()?;
                if Entry::at(&self.plan.timeline, children[previous])
                    .start()
                    .conditions()
                    .iter()
                    .all(|condition| matches!(condition, TimeCondition::Never {}))
                {
                    break;
                }
                previous -= 1;
            }
        }
        Ok(Some(previous))
    }
    pub(super) fn navigate(
        &mut self,
        i: usize,
        direction: NavigationDirection,
        mut moment: Moment,
    ) -> Result<(), TimelineError> {
        self.step()?;
        if self.closed[i] || self.intervals[i].end.is_some() || self.intervals[i].start.is_none() {
            return Ok(());
        }
        if let Some((prior, at)) = &self.intervals[i].last_navigation
            && at == &moment
        {
            if *prior != direction {
                return Err(invalid(
                    Some(Entry::at(&self.plan.timeline, i).id()),
                    "conflicting sequence directions at one event",
                ));
            }
            return Ok(());
        }
        self.intervals[i].last_navigation = Some((direction, moment.clone()));
        let nav = Entry::at(&self.plan.timeline, i)
            .navigation()
            .expect("sequence navigation");
        let current = self.intervals[i].position.expect("sequence cursor");
        let children = &self.h.children[i];
        let Some(destination) = self.navigation_position(i, direction)? else {
            return Ok(());
        };
        match direction {
            NavigationDirection::Next => {
                let child = children[current];
                let next = destination;
                self.move_cursor(i, next, &moment);
                if next != current
                    && nav.next_action == NextAction::Seek
                    && self.intervals[child].end.is_none()
                {
                    let owner = self.intervals.identity(i);
                    moment = self.seek_natural(child, moment)?;
                    if self.intervals.identity(i) != owner
                        || self.closed[i]
                        || self.intervals[i].end.is_some()
                    {
                        return Ok(());
                    }
                    self.move_cursor(i, next, &moment);
                }
                if next != current && !nav.concurrent && self.intervals[child].end.is_none() {
                    self.finish(child, moment.clone(), true)?;
                    // A navigation cutoff completes this slot even before its
                    // natural duration. It does not complete untouched slots.
                    if !self.reported[child] {
                        self.reported[child] = true;
                        self.remaining[i] -= 1;
                        self.child_end[i] = moment.clone();
                    }
                }
                if let Some(&child) = children.get(next) {
                    self.enable(child)?;
                    self.begin(child, moment)?;
                } else if self.remaining[i] == 0
                    && matches!(
                        Entry::at(&self.plan.timeline, i),
                        Entry::Container(TimingContainer {
                            duration: ContainerDuration::Automatic,
                            ..
                        })
                    )
                {
                    self.automatic_bound[i] = Some(self.point(i, moment)?);
                    self.refresh_bound(i)?;
                }
            }
            NavigationDirection::Previous => {
                let presentation = self.presentation_sequence(i);
                let previous = destination;
                // Close every affected branch before any end notification, then
                // reset them all after notifications. Old events cannot leak
                // into an already reset sibling during the same command.
                for &child in &children[previous..] {
                    self.close_for_reset(child)?;
                }
                for &child in &children[previous..] {
                    self.finish(child, moment.clone(), true)?;
                }
                for &child in &children[previous..] {
                    self.reset_slot(child, &moment)?;
                }
                self.automatic_bound[i] = None;
                self.refresh_bound(i)?;
                self.move_cursor(i, previous, &moment);
                let child = children[previous];
                self.enable(child)?;
                if !presentation {
                    self.begin(child, moment)?;
                }
            }
        }
        Ok(())
    }
    fn close_for_reset(&mut self, root: usize) -> Result<(), TimelineError> {
        let mut stack = vec![root];
        while let Some(node) = stack.pop() {
            self.step()?;
            self.muted[node] = true;
            stack.extend(self.h.children[node].iter().copied());
        }
        Ok(())
    }
    fn reset_slot(&mut self, i: usize, moment: &Moment) -> Result<(), TimelineError> {
        self.step()?;
        if self.reported[i]
            && let Some(parent) = self.h.parents[i]
        {
            self.remaining[parent] += 1;
        }
        self.intervals
            .replace(i, self.intervals.parent(i), self.plan.limits.max_intervals)?;
        self.intervals[i].reset = Some(self.event_serial);
        self.intervals[i].reset_at = Some(moment.clone());
        self.epochs[i] += 1;
        self.closed[i] = false;
        self.muted[i] = false;
        self.resolved[i].fill(false);
        self.eligible[i] = false;
        self.reported[i] = false;
        self.remaining[i] = self.h.children[i].len();
        self.declared_bound[i] = None;
        self.automatic_bound[i] = None;
        self.events[i * 2] = None;
        self.events[i * 2 + 1] = None;
        self.reset_children(i)
    }
}
