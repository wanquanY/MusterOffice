//! Validate the complete host log before selecting a consumed prefix. Reuse must
//! never hide invalid future input, omitted coverage or conflicting duplicates.
use crate::{
    EventHistory, InputEvent, PlaybackBinding, PlaybackEvent, TimelineError, TimelinePlan, cancel,
    exact::Ratio,
};
use mo_common::{ObjectId, RationalTime};
use std::collections::BTreeMap;

pub(crate) type Clicks = BTreeMap<Option<ObjectId>, Vec<(Ratio, u32)>>;
pub(crate) struct ValidatedEvents<'a> {
    prefix: &'a [PlaybackEvent],
    cursor: u32,
}
impl ValidatedEvents<'_> {
    pub(crate) fn cursor(&self) -> u32 {
        self.cursor
    }
    pub(crate) fn iter(&self) -> impl Iterator<Item = &PlaybackEvent> {
        let mut previous = 0;
        self.prefix.iter().filter(move |event| {
            let unique = event.sequence != previous;
            previous = event.sequence;
            unique
        })
    }
    pub(crate) fn clicks(&self, check: &dyn Fn() -> bool) -> Result<Clicks, TimelineError> {
        let mut clicks = Clicks::new();
        for event in self.iter() {
            cancel(check)?;
            match &event.event {
                InputEvent::Click { target } => {
                    clicks
                        .entry(target.clone())
                        .or_default()
                        .push((Ratio::time(event.at), event.sequence));
                }
            }
        }
        Ok(clicks)
    }
}
impl TimelinePlan {
    pub(crate) fn validate_events<'a>(
        &self,
        binding: &PlaybackBinding,
        at: RationalTime,
        history: Option<&'a EventHistory>,
        check: &dyn Fn() -> bool,
    ) -> Result<ValidatedEvents<'a>, TimelineError> {
        cancel(check)?;
        if at.ticks.get() < 0 {
            return Err(crate::invalid(None, "negative presentation time"));
        }
        if self.interactive && history.is_none() {
            return Err(TimelineError::MissingEventHistory);
        }
        let Some(history) = history else {
            return Ok(ValidatedEvents {
                prefix: &[],
                cursor: 0,
            });
        };
        if &history.binding != binding {
            return Err(TimelineError::EventHistory(
                "binding does not match session/revision/generation",
            ));
        }
        if history.through.compare_time(at).is_lt() {
            return Err(TimelineError::MissingEventHistory);
        }
        if history.events.len() > self.limits.max_events {
            return Err(TimelineError::Limit("event log count"));
        }
        let mut previous: Option<&PlaybackEvent> = None;
        let mut cursor = 0;
        let mut end = 0;
        for (i, event) in history.events.iter().enumerate() {
            cancel(check)?;
            if event.generation != binding.generation {
                return Err(TimelineError::EventHistory("stale event generation"));
            }
            if event.at.ticks.get() < 0 || event.at.compare_time(history.through).is_gt() {
                return Err(TimelineError::EventHistory("event outside covered time"));
            }
            if let Some(p) = previous {
                if event == p {
                    continue;
                }
                if p.sequence.checked_add(1) != Some(event.sequence)
                    || event.at.compare_time(p.at).is_lt()
                {
                    return Err(TimelineError::EventHistory(
                        "non-contiguous, conflicting or unordered event",
                    ));
                }
            } else if event.sequence != 1 {
                return Err(TimelineError::EventHistory(
                    "complete prefix must start at sequence one",
                ));
            }
            previous = Some(event);
            if event.at.compare_time(at).is_le() {
                cursor = event.sequence;
                end = i + 1;
            }
        }
        Ok(ValidatedEvents {
            prefix: &history.events[..end],
            cursor,
        })
    }
}
