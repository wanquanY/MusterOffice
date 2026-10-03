//! Dispatch each actual event once, resolving begin versus end from the same
//! pre-event state. Delay changes execution time, never event sensitivity.
use super::*;
impl Scheduler<'_> {
    pub(super) fn resolve_start(&mut self, i: usize, k: usize) -> Result<(), TimelineError> {
        self.step()?;
        if self.closed[i] || self.intervals[i].start.is_some() {
            return Ok(());
        }
        let Some(gate) = self.intervals[i].gate.clone() else {
            return Ok(());
        };
        let condition = &Entry::at(&self.plan.timeline, i).start().conditions()[k];
        if let Some(candidate) = self.condition(
            i,
            condition,
            self.h.dependencies[i]
                .get(&ConditionIndex::Start(k))
                .copied(),
            &gate,
            true,
        )? {
            let owner = matches!(condition, TimeCondition::After { .. })
                .then(|| self.intervals.identity(i));
            self.push(i, candidate, Action::Begin { owner })?;
        }
        Ok(())
    }
    pub(super) fn resolve_end(&mut self, i: usize, k: usize) -> Result<(), TimelineError> {
        self.step()?;
        let Some(start) = self.intervals[i].start.clone() else {
            return Ok(());
        };
        let condition = &Entry::at(&self.plan.timeline, i).ends()[k];
        if let Some(mut candidate) = self.condition(
            i,
            condition,
            self.h.dependencies[i].get(&ConditionIndex::End(k)).copied(),
            &start,
            false,
        )? {
            self.resolved[i][k] = true;
            if !candidate.precedes(&start) {
                self.eligible[i] = true;
                candidate.sequence = candidate.sequence.max(start.sequence);
                self.bound(i, candidate)?;
            }
        }
        Ok(())
    }
    pub(super) fn dispatch(
        &mut self,
        listeners: &[(usize, ConditionIndex)],
        source: &Moment,
        external: bool,
        presentation_step: bool,
    ) -> Result<bool, TimelineError> {
        // Compile appends listeners in node order, with starts before ends.
        // No global node scan is necessary, including long condition lists.
        let mut position = 0;
        let mut consumed = false;
        while position < listeners.len() {
            self.step()?;
            let i = listeners[position].0;
            if self.muted[i] {
                while position < listeners.len() && listeners[position].0 == i {
                    self.step()?;
                    position += 1;
                }
                continue;
            }
            let entry = Entry::at(&self.plan.timeline, i);
            let active = self.intervals[i].start.is_some() && self.intervals[i].end.is_none();
            let begin_allowed = !self.closed[i]
                && self.intervals[i].gate.as_ref().is_some_and(|gate| {
                    !source.precedes(gate) && (!external || source.sequence > gate.sequence)
                })
                && if active {
                    entry.restart() == RestartMode::Always
                } else {
                    self.intervals[i].start.is_none() || !entry.restart().is_never()
                };
            let end_allowed = self.intervals[i].start.is_some()
                && self.intervals[i]
                    .end
                    .as_ref()
                    .is_none_or(|end| end.moment == *source);
            let mut accepted_begin = false;
            let mut accepted_end = false;
            let mut navigation: Option<(NavigationDirection, Moment)> = None;
            while position < listeners.len() && listeners[position].0 == i {
                self.step()?;
                let condition = listeners[position].1;
                position += 1;
                match condition {
                    ConditionIndex::Start(k) if begin_allowed => {
                        let atom = &entry.start().conditions()[k];
                        let moment = self.delayed(i, source, atom)?;
                        self.push(
                            i,
                            moment,
                            Action::Begin {
                                owner: Some(self.intervals.identity(i)),
                            },
                        )?;
                        accepted_begin = true;
                        consumed = true;
                    }
                    ConditionIndex::End(k) if end_allowed && !accepted_begin => {
                        let atom = &entry.ends()[k];
                        let moment = self.delayed(i, source, atom)?;
                        self.resolved[i][k] = true;
                        self.eligible[i] = true;
                        self.bound(i, moment)?;
                        accepted_end = true;
                        consumed = true;
                    }
                    index @ (ConditionIndex::Next(_) | ConditionIndex::Previous(_))
                        if active
                            && !self.closed[i]
                            && !accepted_begin
                            && !accepted_end
                            && self.intervals[i].start.as_ref().is_some_and(|start| {
                                !source.precedes(start)
                                    && (!external || source.sequence > start.sequence)
                            }) =>
                    {
                        let direction = if matches!(index, ConditionIndex::Next(_)) {
                            NavigationDirection::Next
                        } else {
                            NavigationDirection::Previous
                        };
                        // A sequence at its boundary cannot consume a slideshow
                        // step merely because an onNext/onPrev listener exists.
                        // Raw event scheduling retains its original semantics.
                        if presentation_step && self.navigation_position(i, direction)?.is_none() {
                            continue;
                        }
                        let moment = self.delayed(i, source, entry.condition(index))?;
                        if let Some((prior, at)) = &navigation
                            && *prior != direction
                            && *at == moment
                        {
                            return Err(invalid(
                                Some(entry.id()),
                                "one event requests conflicting sequence directions",
                            ));
                        }
                        if navigation
                            .as_ref()
                            .is_none_or(|(_, at)| moment.precedes(at))
                        {
                            navigation = Some((direction, moment));
                        }
                    }
                    _ => (),
                }
            }
            if let Some((direction, moment)) = navigation {
                self.push(
                    i,
                    moment,
                    Action::Navigate {
                        direction,
                        owner: self.intervals.identity(i),
                    },
                )?;
                consumed = true;
            }
        }
        Ok(consumed)
    }
    fn delayed(
        &self,
        i: usize,
        source: &Moment,
        atom: &TimeCondition,
    ) -> Result<Moment, TimelineError> {
        let delay = match atom {
            TimeCondition::After { delay, .. }
            | TimeCondition::Click { delay, .. }
            | TimeCondition::Navigation { delay, .. } => delay,
            _ => unreachable!("event listener"),
        };
        self.delay(i, source, &Ratio::time(*delay))
    }
    pub(super) fn condition(
        &self,
        i: usize,
        condition: &TimeCondition,
        dependency: Option<usize>,
        reference: &Moment,
        start_condition: bool,
    ) -> Result<Option<Moment>, TimelineError> {
        match condition {
            TimeCondition::Never {}
            | TimeCondition::Click { .. }
            | TimeCondition::Navigation { .. } => Ok(None),
            TimeCondition::At { offset } => {
                let gate = self.intervals[i].gate.as_ref().expect("enabled scope");
                Ok(Some(self.delay(i, gate, &Ratio::time(*offset))?))
            }
            TimeCondition::After { event, delay, .. } => {
                let source = dependency.expect("compiled dependency") * 2 + event.edge_index();
                self.events[source]
                    .as_ref()
                    .filter(|event| !start_condition || !event.moment.precedes(reference))
                    .filter(|event| {
                        self.intervals[i]
                            .reset
                            .is_none_or(|reset| event.serial > reset)
                    })
                    .map(|event| {
                        let m = &event.moment;
                        let mut moment = self.delay(i, m, &Ratio::time(*delay))?;
                        if !moment.precedes(reference) {
                            moment.sequence = moment.sequence.max(reference.sequence);
                        }
                        Ok(moment)
                    })
                    .transpose()
            }
        }
    }
}
