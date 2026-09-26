use super::*;
impl Scheduler<'_> {
    pub(super) fn condition(
        &self,
        i: usize,
        condition: &TimeCondition,
        dependency: Option<usize>,
        reference: &Moment,
        start_condition: bool,
    ) -> Result<Option<Moment>, TimelineError> {
        let bits = self.bits();
        match condition {
            TimeCondition::At { offset } => {
                let gate = self.intervals[i].gate.as_ref().expect("enabled scope");
                Ok(Some(Moment {
                    time: gate.time.add(&Ratio::time(*offset), bits)?,
                    sequence: gate.sequence,
                }))
            }
            TimeCondition::After { event, delay, .. } => {
                let source = &self.intervals[dependency.expect("compiled dependency")];
                let value = match event {
                    NodeEvent::Begin => source.start.clone(),
                    NodeEvent::End => source.end.as_ref().map(|time| Moment {
                        time: time.clone(),
                        sequence: source.end_sequence,
                    }),
                };
                value
                    .filter(|m| {
                        !start_condition
                            || m.time.cmp(&reference.time).is_gt()
                            || (m.time.cmp(&reference.time).is_eq()
                                && m.sequence >= reference.sequence)
                    })
                    .map(|m| {
                        Ok(Moment {
                            time: m.time.add(&Ratio::time(*delay), bits)?,
                            sequence: m.sequence.max(reference.sequence),
                        })
                    })
                    .transpose()
            }
            TimeCondition::Click { target, delay } => {
                let clicks = self
                    .clicks
                    .get(target)
                    .map(Vec::as_slice)
                    .unwrap_or_default();
                let next = clicks.partition_point(|(time, sequence)| {
                    time.cmp(&reference.time).is_lt() || *sequence <= reference.sequence
                });
                clicks
                    .get(next)
                    .map(|(time, sequence)| {
                        Ok(Moment {
                            time: time.add(&Ratio::time(*delay), bits)?,
                            sequence: *sequence,
                        })
                    })
                    .transpose()
            }
        }
    }
}
