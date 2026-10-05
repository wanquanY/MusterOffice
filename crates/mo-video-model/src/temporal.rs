use mo_common::RationalTime;
use mo_time::{ExactRatio, TimeError};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FrameRate {
    pub numerator: u32,
    pub denominator: u32,
}
impl FrameRate {
    pub fn valid(self) -> bool {
        (1..=1_000_000_000).contains(&self.numerator)
            && (1..=1_000_000_000).contains(&self.denominator)
    }
    pub fn frame_time(self, index: i64, bits: u64) -> Result<ExactRatio, TimeError> {
        if !self.valid() {
            return Err(TimeError::Denominator);
        }
        ExactRatio::frame_time(index, self.numerator, self.denominator, bits)
    }
}

/// Half-open [start,end). Source ranges may begin before zero; timeline placement may not.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TimeRange {
    pub start: RationalTime,
    pub end: RationalTime,
}
impl TimeRange {
    pub fn valid(self) -> bool {
        self.start.compare_time(self.end).is_lt()
    }
    pub fn contains(self, time: RationalTime) -> bool {
        self.start.compare_time(time).is_le() && time.compare_time(self.end).is_lt()
    }
    pub fn contains_boundary(self, time: RationalTime) -> bool {
        self.start.compare_time(time).is_le() && time.compare_time(self.end).is_le()
    }
    pub fn intersects(self, other: Self) -> bool {
        self.start.compare_time(other.end).is_lt() && other.start.compare_time(self.end).is_lt()
    }
    pub fn duration(self, bits: u64) -> Result<ExactRatio, TimeError> {
        ExactRatio::from_time(self.end).sub(&ExactRatio::from_time(self.start), bits)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TimeAnchor {
    pub local: RationalTime,
    pub source: RationalTime,
}

/// Source-time anchors are the authority; a separate mutable speed curve is forbidden.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TimeMap {
    pub anchors: Vec<TimeAnchor>,
}

/// A one-sided sample at an exact instant. Reversal flips the side rather than inventing an epsilon.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum SampleSide {
    After,
    Before,
}
impl SampleSide {
    pub fn reversed(self) -> Self {
        match self {
            Self::After => Self::Before,
            Self::Before => Self::After,
        }
    }
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TimeSample {
    pub time: ExactRatio,
    pub side: SampleSide,
}
impl TimeSample {
    pub fn after(time: RationalTime) -> Self {
        Self {
            time: ExactRatio::from_time(time),
            side: SampleSide::After,
        }
    }
    pub fn covers(&self, range: TimeRange) -> bool {
        let start = ExactRatio::from_time(range.start).compare(&self.time);
        let end = self.time.compare(&ExactRatio::from_time(range.end));
        match self.side {
            SampleSide::After => start.is_le() && end.is_lt(),
            SampleSide::Before => start.is_lt() && end.is_le(),
        }
    }
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MappedTime {
    pub sample: TimeSample,
    pub rate: ExactRatio,
}
impl TimeMap {
    /// Edit-time boundary lookup. This endpoint must not be used to request a media sample.
    pub fn map_boundary(
        &self,
        local: RationalTime,
        bits: u64,
    ) -> Result<Option<ExactRatio>, TimeError> {
        if let Some(last) = self.anchors.last()
            && last.local.compare_time(local).is_eq()
        {
            let time = ExactRatio::from_time(last.source);
            time.check_budget(bits)?;
            return Ok(Some(time));
        }
        self.map(local, bits)
    }
    /// Piecewise linear, reverse and hold mapping. Caller supplies a validated map.
    /// Half-open lookup deliberately does not return a sample for the final boundary.
    pub fn map(&self, local: RationalTime, bits: u64) -> Result<Option<ExactRatio>, TimeError> {
        self.sample(&TimeSample::after(local), bits)
            .map(|mapped| mapped.map(|m| m.sample.time))
    }
    /// Caller supplies a validated map. This preserves arbitrary exact intermediate time across
    /// nested sequences and returns the source-side limit and local derivative used by media I/O.
    pub fn sample(&self, local: &TimeSample, bits: u64) -> Result<Option<MappedTime>, TimeError> {
        local.time.check_budget(bits)?;
        let right = self.anchors.partition_point(|a| match local.side {
            SampleSide::After => ExactRatio::from_time(a.local).compare(&local.time).is_le(),
            SampleSide::Before => ExactRatio::from_time(a.local).compare(&local.time).is_lt(),
        });
        if right == 0 || right == self.anchors.len() {
            return Ok(None);
        }
        let a = self.anchors[right - 1];
        let b = self.anchors[right];
        let elapsed = local.time.sub(&ExactRatio::from_time(a.local), bits)?;
        let duration = ExactRatio::from_time(b.local).sub(&ExactRatio::from_time(a.local), bits)?;
        let span = ExactRatio::from_time(b.source).sub(&ExactRatio::from_time(a.source), bits)?;
        let rate = span.div(&duration, bits)?;
        let time = ExactRatio::from_time(a.source).add(&rate.mul(&elapsed, bits)?, bits)?;
        let side = match rate.compare(&ExactRatio::integer(0)) {
            std::cmp::Ordering::Greater => local.side,
            std::cmp::Ordering::Less => local.side.reversed(),
            std::cmp::Ordering::Equal => SampleSide::After,
        };
        Ok(Some(MappedTime {
            sample: TimeSample { time, side },
            rate,
        }))
    }
}
