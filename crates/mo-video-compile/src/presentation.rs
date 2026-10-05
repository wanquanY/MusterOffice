use crate::{CompileError, MediaBinding, Result};
use mo_common::Digest;
use mo_video_model::{MediaStream, SampleSide, StreamId, TimeRange, TimeSample};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

/// Host-supplied bounded presentation-order metadata, independent of packet decode order.
/// A window does not claim that the whole source was indexed; outside-window requires another read.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PresentationWindow {
    pub resource_digest: Digest,
    pub stream: StreamId,
    pub coverage: TimeRange,
    pub samples: Vec<PresentationSample>,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PresentationSample {
    /// Opaque identity assigned by the host in this window, not an inferred CFR frame number.
    pub id: u32,
    pub interval: TimeRange,
}
#[derive(Debug, Clone, Copy)]
pub struct PresentationLimits {
    pub max_samples: usize,
    pub exact_bits: u64,
}
impl Default for PresentationLimits {
    fn default() -> Self {
        Self {
            max_samples: 100_000,
            exact_bits: 256,
        }
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "status", rename_all = "camelCase", deny_unknown_fields)]
pub enum PresentationSelection {
    Sample { sample: PresentationSample },
    Gap,
    OutsideWindow,
}
#[derive(Debug)]
pub struct PresentationIndex {
    window: PresentationWindow,
    bits: u64,
}
impl PresentationIndex {
    pub fn new(
        window: PresentationWindow,
        limits: PresentationLimits,
        check: &dyn Fn() -> bool,
    ) -> Result<Self> {
        if !(128..=4096).contains(&limits.exact_bits) || limits.max_samples == 0 {
            return Err(CompileError::Limits);
        }
        if check() {
            return Err(CompileError::Cancelled);
        }
        if window.samples.len() > limits.max_samples {
            return Err(CompileError::Budget);
        }
        if !window.coverage.valid() {
            return Err(CompileError::Presentation("invalid coverage"));
        }
        let mut ids = BTreeSet::new();
        let mut previous_end = None;
        for sample in &window.samples {
            if check() {
                return Err(CompileError::Cancelled);
            }
            if !sample.interval.valid()
                || !window.coverage.contains_boundary(sample.interval.start)
                || !window.coverage.contains_boundary(sample.interval.end)
            {
                return Err(CompileError::Presentation(
                    "sample outside coverage or empty interval",
                ));
            }
            if !ids.insert(sample.id) {
                return Err(CompileError::Presentation("duplicate sample identity"));
            }
            if previous_end.is_some_and(|end: mo_common::RationalTime| {
                end.compare_time(sample.interval.start).is_gt()
            }) {
                return Err(CompileError::Presentation(
                    "samples overlap or are not in presentation order",
                ));
            }
            previous_end = Some(sample.interval.end);
        }
        Ok(Self {
            window,
            bits: limits.exact_bits,
        })
    }
    pub fn window(&self) -> &PresentationWindow {
        &self.window
    }
    pub fn select(
        &self,
        binding: &MediaBinding,
        sample: &TimeSample,
        check: &dyn Fn() -> bool,
    ) -> Result<PresentationSelection> {
        if check() {
            return Err(CompileError::Cancelled);
        }
        sample.time.check_budget(self.bits)?;
        if binding.digest != self.window.resource_digest || binding.stream != self.window.stream {
            return Err(CompileError::Presentation(
                "source binding does not match this index",
            ));
        }
        let MediaStream::Video { available, .. } = binding.descriptor else {
            return Err(CompileError::Presentation(
                "presentation index requires a video stream",
            ));
        };
        if !available.contains_boundary(self.window.coverage.start)
            || !available.contains_boundary(self.window.coverage.end)
        {
            return Err(CompileError::Presentation(
                "index coverage exceeds the bound stream",
            ));
        }
        if !sample.covers(self.window.coverage) {
            return Ok(PresentationSelection::OutsideWindow);
        }
        let right = self.window.samples.partition_point(|entry| {
            let start = mo_time::ExactRatio::from_time(entry.interval.start).compare(&sample.time);
            match sample.side {
                SampleSide::After => start.is_le(),
                SampleSide::Before => start.is_lt(),
            }
        });
        Ok(right
            .checked_sub(1)
            .and_then(|i| self.window.samples.get(i))
            .filter(|entry| sample.covers(entry.interval))
            .map_or(PresentationSelection::Gap, |entry| {
                PresentationSelection::Sample { sample: *entry }
            }))
    }
}
