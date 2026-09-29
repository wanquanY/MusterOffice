//! Explicitly owned, single-entry interval reuse. The host still owns clocks,
//! history persistence and output publication. No frame or future-event cache.
use crate::{
    EvaluatedFrame, EventHistory, InputEvent, PlaybackBinding, TimelineError, TimelinePlan, cancel,
    events::ValidatedEvents, tree,
};
use mo_common::{Digest, RationalTime};
use schemars::{JsonSchema, Schema, SchemaGenerator, json_schema};
use serde::{Deserialize, Serialize};
use std::borrow::Cow;

/// Exact diagnostic work count, represented as canonical uint64 text on all hosts.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct TimelineWorkCount(u64);
impl TimelineWorkCount {
    pub const fn get(self) -> u64 {
        self.0
    }
    fn next(self) -> Result<Self, TimelineError> {
        self.0
            .checked_add(1)
            .map(Self)
            .ok_or(TimelineError::Limit("timeline work count"))
    }
}
impl TryFrom<String> for TimelineWorkCount {
    type Error = &'static str;
    fn try_from(value: String) -> Result<Self, Self::Error> {
        let n = value
            .parse::<u64>()
            .map_err(|_| "expected canonical uint64 work count")?;
        if n.to_string() != value {
            return Err("expected canonical uint64 work count");
        }
        Ok(Self(n))
    }
}
impl From<TimelineWorkCount> for String {
    fn from(value: TimelineWorkCount) -> Self {
        value.0.to_string()
    }
}
impl JsonSchema for TimelineWorkCount {
    fn schema_name() -> Cow<'static, str> {
        "TimelineWorkCount".into()
    }
    fn json_schema(_: &mut SchemaGenerator) -> Schema {
        json_schema!({"type":"string","pattern":"^(0|[1-9][0-9]{0,19})$","not":{"pattern":"[^0-9]"},"x-integer-maximum":"18446744073709551615","description":"Canonical uint64 timeline work count; never wraps."})
    }
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TimelineSamplerInfo {
    pub timeline_sha256: Digest,
    pub cached_binding: Option<PlaybackBinding>,
    /// Successful timeline samples only. Downstream page/raster work may fail.
    pub schedules_built: TimelineWorkCount,
    pub schedules_reused: TimelineWorkCount,
    pub retained_intervals: TimelineWorkCount,
    pub retained_events: TimelineWorkCount,
}
struct EventStamp {
    at: RationalTime,
    event: InputEvent,
}
struct CachedIntervals {
    binding: PlaybackBinding,
    events: Vec<EventStamp>,
    intervals: tree::Intervals,
}
impl CachedIntervals {
    fn matches(
        &self,
        binding: &PlaybackBinding,
        events: &ValidatedEvents<'_>,
        check: &dyn Fn() -> bool,
    ) -> Result<bool, TimelineError> {
        if &self.binding != binding || self.events.len() != events.cursor() as usize {
            return Ok(false);
        }
        for (cached, event) in self.events.iter().zip(events.iter()) {
            cancel(check)?;
            if !cached.at.compare_time(event.at).is_eq() || cached.event != event.event {
                return Ok(false);
            }
        }
        Ok(true)
    }
}
/// Owns its immutable plan so interval schedules cannot be reused with another
/// graph. One consumed event prefix and a bounded activation arena belong to
/// this owner. Restarting plans also key reuse by the exact sample horizon;
/// future feedback is never expanded speculatively or borrowed on a seek.
pub struct TimelineSampler {
    plan: TimelinePlan,
    cached: Option<CachedIntervals>,
    built: TimelineWorkCount,
    reused: TimelineWorkCount,
}
impl TimelineSampler {
    pub fn new(plan: TimelinePlan) -> Self {
        Self {
            plan,
            cached: None,
            built: Default::default(),
            reused: Default::default(),
        }
    }
    pub fn info(&self) -> TimelineSamplerInfo {
        TimelineSamplerInfo {
            timeline_sha256: self.plan.digest().clone(),
            cached_binding: self.cached.as_ref().map(|c| c.binding.clone()),
            schedules_built: self.built,
            schedules_reused: self.reused,
            retained_intervals: TimelineWorkCount(
                self.cached.as_ref().map_or(0, |c| c.intervals.len()) as u64,
            ),
            retained_events: TimelineWorkCount(
                self.cached.as_ref().map_or(0, |c| c.events.len()) as u64
            ),
        }
    }
    /// Releases intervals and event identities, preserving the compiled plan and
    /// lifetime work counts. Owners call this when advancing playback generation.
    pub fn clear(&mut self) {
        self.cached = None;
    }
    pub fn evaluate(
        &mut self,
        binding: &PlaybackBinding,
        at: RationalTime,
        history: Option<&EventHistory>,
        check: &dyn Fn() -> bool,
    ) -> Result<EvaluatedFrame, TimelineError> {
        let events = self.plan.validate_events(binding, at, history, check)?;
        if let Some(cached) = &self.cached
            && cached.intervals.covers(at)
            && cached.matches(binding, &events, check)?
        {
            let reused = self.reused.next()?;
            let frame = tree::sample(
                &self.plan,
                &cached.intervals,
                binding,
                at,
                events.cursor(),
                check,
            )?;
            cancel(check)?;
            self.reused = reused;
            return Ok(frame);
        }
        let built = self.built.next()?;
        let intervals = tree::schedule(&self.plan, &events.inputs(check)?, at, check)?;
        let frame = tree::sample(&self.plan, &intervals, binding, at, events.cursor(), check)?;
        let stamps = events
            .iter()
            .map(|event| {
                cancel(check)?;
                Ok(EventStamp {
                    at: event.at.normalized(),
                    event: event.event.clone(),
                })
            })
            .collect::<Result<_, TimelineError>>()?;
        cancel(check)?;
        // No fallible work after publication. Failed/cancelled samples leave the
        // preceding entry and counters untouched, including backwards seeks.
        self.cached = Some(CachedIntervals {
            binding: binding.clone(),
            events: stamps,
            intervals,
        });
        self.built = built;
        Ok(frame)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn diagnostic_counts_are_exact_and_do_not_wrap() {
        let max = TimelineWorkCount(u64::MAX);
        assert_eq!(
            serde_json::to_string(&max).unwrap(),
            "\"18446744073709551615\""
        );
        assert!(matches!(max.next(), Err(TimelineError::Limit(_))));
        for value in ["00", "01", "-1", "+1", "18446744073709551616", "1\n"] {
            assert!(TimelineWorkCount::try_from(value.to_string()).is_err());
        }
    }
}
