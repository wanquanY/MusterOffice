//! Exact nonfragmented video composition and edit-list mapping. This establishes
//! container timing, not successful decoding or one-picture-per-sample admission.
mod access;
mod build;
mod validation;
use crate::TrackIndex;
pub use mo_time::ExactRatio;
use mo_time::TimeError;
use serde::{Deserialize, Serialize};
use std::panic::{AssertUnwindSafe, catch_unwind};

#[derive(Clone, Copy, Debug)]
pub struct TimingLimits {
    pub max_samples: usize,
    pub max_edits: usize,
    pub exact_bits: u64,
}
impl Default for TimingLimits {
    fn default() -> Self {
        Self {
            max_samples: 2_000_000,
            max_edits: 4096,
            exact_bits: 256,
        }
    }
}
#[derive(Debug, thiserror::Error)]
pub enum TimingError {
    #[error("invalid video composition timing: {0}")]
    Invalid(&'static str),
    #[error("unsupported video composition timing: {0}")]
    Unsupported(&'static str),
    #[error("video composition timing budget exceeded")]
    Budget,
    #[error("video composition timing cancelled")]
    Cancelled,
    #[error("video composition callback panicked")]
    CallbackPanic,
    #[error(transparent)]
    Time(#[from] TimeError),
}
pub type Result<T> = std::result::Result<T, TimingError>;
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Side {
    Before,
    After,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Interval {
    pub start: ExactRatio,
    pub end: ExactRatio,
}
impl Interval {
    pub fn contains(&self, time: &ExactRatio, side: Side) -> bool {
        match side {
            Side::After => self.start.compare(time).is_le() && time.compare(&self.end).is_lt(),
            Side::Before => self.start.compare(time).is_lt() && time.compare(&self.end).is_le(),
        }
    }
    pub fn validate(&self, bits: u64) -> Result<()> {
        self.start.check_budget(bits)?;
        self.end.check_budget(bits)?;
        if self.start.compare(&self.end).is_ge() {
            return Err(TimingError::Invalid("empty interval"));
        }
        Ok(())
    }
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct VideoTimingSummary {
    pub track_id: u32,
    pub movie_timescale: u32,
    pub media_timescale: u32,
    pub samples: u32,
    pub edits: u32,
    /// Raw media composition ticks; end = last displayed PTS + its decode delta.
    pub raw_span: Option<(i64, i64)>,
    /// Track-local movie time ends here, before any document clip mapping.
    pub movie_end: ExactRatio,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    tag = "type",
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub enum VideoTimeSelection {
    Outside,
    Gap {
        movie: Interval,
        edit: Option<u32>,
    },
    Sample {
        /// Original decode-order sample table index; never a nominal frame number.
        sample: u32,
        /// Original decode-clock start of this access unit, before DTS shifts.
        decode_start: i64,
        /// Latest preceding sync candidate in both decode and presentation
        /// order. Codec-specific roll/recovery may require an earlier point.
        access: Option<VideoAccess>,
        edit: Option<u32>,
        movie: Interval,
        raw_start: i64,
        raw_end: i64,
        media_time: ExactRatio,
        media_side: Side,
    },
}
#[derive(Clone, Copy)]
struct Raw {
    pts: i64,
    dts: i64,
    duration: u32,
    index: u32,
    access: u32,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct VideoAccess {
    pub sample: u32,
    pub decode_start: i64,
}
struct Segment {
    movie: Interval,
    media: Option<ExactRatio>,
    rate: ExactRatio,
    index: Option<u32>,
}
pub struct VideoTiming {
    summary: VideoTimingSummary,
    samples: Vec<Raw>,
    access: Vec<VideoAccess>,
    segments: Vec<Segment>,
    bits: u64,
}
fn active(check: &dyn Fn() -> bool) -> Result<()> {
    if check() {
        Err(TimingError::Cancelled)
    } else {
        Ok(())
    }
}
fn guarded<T>(work: impl FnOnce() -> Result<T>) -> Result<T> {
    match catch_unwind(AssertUnwindSafe(work)) {
        Ok(result) => result,
        Err(payload) => {
            std::mem::forget(payload);
            Err(TimingError::CallbackPanic)
        }
    }
}
fn seconds(ticks: i64, scale: u32, bits: u64) -> Result<ExactRatio> {
    Ok(ExactRatio::new(ticks.into(), scale.into(), bits)?)
}
impl VideoTiming {
    pub fn build(
        track: &TrackIndex,
        movie_timescale: u32,
        limits: TimingLimits,
        check: &dyn Fn() -> bool,
    ) -> Result<Self> {
        guarded(|| build::build(track, movie_timescale, limits, check))
    }
    pub fn summary(&self) -> &VideoTimingSummary {
        &self.summary
    }
    pub fn select(
        &self,
        time: &ExactRatio,
        side: Side,
        check: &dyn Fn() -> bool,
    ) -> Result<VideoTimeSelection> {
        guarded(|| {
            let selected = self.select_inner(time, side, check)?;
            active(check)?;
            Ok(selected)
        })
    }
    fn select_inner(
        &self,
        time: &ExactRatio,
        side: Side,
        check: &dyn Fn() -> bool,
    ) -> Result<VideoTimeSelection> {
        active(check)?;
        time.check_budget(self.bits)?;
        let i = self.segments.partition_point(|s| match side {
            Side::After => s.movie.end.compare(time).is_le(),
            Side::Before => s.movie.end.compare(time).is_lt(),
        });
        let Some(segment) = self
            .segments
            .get(i)
            .filter(|s| s.movie.contains(time, side))
        else {
            return Ok(VideoTimeSelection::Outside);
        };
        let gap = |movie| VideoTimeSelection::Gap {
            movie,
            edit: segment.index,
        };
        let Some(origin) = &segment.media else {
            return Ok(gap(segment.movie.clone()));
        };
        let dwell = !segment.rate.positive();
        let media = origin.add(
            &time
                .sub(&segment.movie.start, self.bits)?
                .mul(&segment.rate, self.bits)?,
            self.bits,
        )?;
        let media_side = if dwell { Side::After } else { side };
        let ticks = media.mul(
            &ExactRatio::integer(i64::from(self.summary.media_timescale)),
            self.bits,
        )?;
        let sample_at = self.samples.partition_point(|s| match media_side {
            Side::After => ExactRatio::integer(s.pts).compare(&ticks).is_le(),
            Side::Before => ExactRatio::integer(s.pts).compare(&ticks).is_lt(),
        });
        let project = |value: &ExactRatio| -> Result<ExactRatio> {
            Ok(segment.movie.start.add(
                &value
                    .sub(origin, self.bits)?
                    .div(&segment.rate, self.bits)?,
                self.bits,
            )?)
        };
        let clip = |start: ExactRatio, end: ExactRatio| Interval {
            start: if start.compare(&segment.movie.start).is_lt() {
                segment.movie.start.clone()
            } else {
                start
            },
            end: if end.compare(&segment.movie.end).is_gt() {
                segment.movie.end.clone()
            } else {
                end
            },
        };
        let Some((first, last)) = self.summary.raw_span else {
            return Ok(gap(segment.movie.clone()));
        };
        let outside = sample_at == 0
            || match media_side {
                Side::After => ticks.compare(&ExactRatio::integer(last)).is_ge(),
                Side::Before => ticks.compare(&ExactRatio::integer(last)).is_gt(),
            };
        if outside {
            let range = if dwell {
                segment.movie.clone()
            } else if sample_at == 0 {
                clip(
                    segment.movie.start.clone(),
                    project(&seconds(first, self.summary.media_timescale, self.bits)?)?,
                )
            } else {
                clip(
                    project(&seconds(last, self.summary.media_timescale, self.bits)?)?,
                    segment.movie.end.clone(),
                )
            };
            range.validate(self.bits)?;
            active(check)?;
            return Ok(gap(range));
        }
        let sample = self.samples[sample_at - 1];
        let access = sample
            .access
            .checked_sub(1)
            .and_then(|i| self.access.get(i as usize))
            .copied();
        let end = self.samples.get(sample_at).map_or(last, |s| s.pts);
        let movie = if dwell {
            segment.movie.clone()
        } else {
            clip(
                project(&seconds(
                    sample.pts,
                    self.summary.media_timescale,
                    self.bits,
                )?)?,
                project(&seconds(end, self.summary.media_timescale, self.bits)?)?,
            )
        };
        movie.validate(self.bits)?;
        if !movie.contains(time, side) {
            return Err(TimingError::Invalid("mapped selection boundary"));
        }
        active(check)?;
        Ok(VideoTimeSelection::Sample {
            sample: sample.index,
            decode_start: sample.dts,
            access,
            edit: segment.index,
            movie,
            raw_start: sample.pts,
            raw_end: end,
            media_time: media,
            media_side,
        })
    }
}
