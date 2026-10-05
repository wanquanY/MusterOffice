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
            return Ok(Some(ExactRatio::from_time(last.source)));
        }
        self.map(local, bits)
    }
    /// Piecewise linear, reverse and hold mapping. Caller supplies a validated map.
    /// Half-open lookup deliberately does not return a sample for the final boundary.
    pub fn map(&self, local: RationalTime, bits: u64) -> Result<Option<ExactRatio>, TimeError> {
        let right = self
            .anchors
            .partition_point(|a| a.local.compare_time(local).is_le());
        if right == 0 || right == self.anchors.len() {
            return Ok(None);
        }
        let a = self.anchors[right - 1];
        let b = self.anchors[right];
        let elapsed = ExactRatio::from_time(local).sub(&ExactRatio::from_time(a.local), bits)?;
        let duration = ExactRatio::from_time(b.local).sub(&ExactRatio::from_time(a.local), bits)?;
        let span = ExactRatio::from_time(b.source).sub(&ExactRatio::from_time(a.source), bits)?;
        ExactRatio::from_time(a.source)
            .add(&span.mul(&elapsed.div(&duration, bits)?, bits)?, bits)
            .map(Some)
    }
}
