//! Shared validation for in-process and transported timing results.
use super::{
    ExactRatio, Interval, Result, Side as MediaTimeSide, TimingError, VideoTimeSelection,
    VideoTimingSummary,
};
impl VideoTimingSummary {
    pub fn validate(&self, bits: u64) -> Result<()> {
        if !(32..=4096).contains(&bits) {
            return Err(TimingError::Budget);
        }
        self.movie_end.check_budget(bits)?;
        if self.track_id == 0
            || self.movie_timescale == 0
            || self.media_timescale == 0
            || self.samples > 10_000_000
            || self.edits > 100_000
            || self.movie_end.compare(&ExactRatio::integer(0)).is_lt()
            || !match self.raw_span {
                Some((a, b)) => a < b && self.samples > 0,
                None => self.samples == 0,
            }
        {
            return Err(TimingError::Invalid("video timing summary"));
        }
        Ok(())
    }
}
impl VideoTimeSelection {
    /// Structural consistency, not proof that a codec produced this sample.
    pub fn validate(
        &self,
        summary: &VideoTimingSummary,
        time: &ExactRatio,
        side: MediaTimeSide,
        bits: u64,
    ) -> Result<()> {
        summary.validate(bits)?;
        time.check_budget(bits)?;
        let full = Interval {
            start: ExactRatio::integer(0),
            end: summary.movie_end.clone(),
        };
        let valid_range = |range: &Interval, edit: Option<u32>| {
            range.validate(bits).is_ok()
                && range.start.compare(&full.start).is_ge()
                && range.end.compare(&full.end).is_le()
                && range.contains(time, side)
                && match edit {
                    None => summary.edits == 0,
                    Some(index) => index < summary.edits,
                }
        };
        let valid = match self {
            VideoTimeSelection::Outside => !full.contains(time, side),
            VideoTimeSelection::Gap { movie, edit } => valid_range(movie, *edit),
            VideoTimeSelection::Sample {
                sample,
                decode_start,
                access,
                edit,
                movie,
                raw_start,
                raw_end,
                media_time,
                media_side,
            } => {
                if *sample >= summary.samples
                    || *decode_start < 0
                    || access.is_some_and(|a| {
                        a.sample > *sample || a.decode_start < 0 || a.decode_start > *decode_start
                    })
                    || !valid_range(movie, *edit)
                    || raw_start >= raw_end
                    || !summary
                        .raw_span
                        .is_some_and(|(first, last)| first <= *raw_start && *raw_end <= last)
                    || media_time.check_budget(bits).is_err()
                {
                    return Err(TimingError::Invalid("mapped sample bounds"));
                }
                let raw = Interval {
                    start: ExactRatio::integer(*raw_start),
                    end: ExactRatio::integer(*raw_end),
                };
                media_time
                    .mul(
                        &ExactRatio::integer(i64::from(summary.media_timescale)),
                        bits,
                    )
                    .is_ok_and(|ticks| raw.contains(&ticks, *media_side))
            }
        };
        if valid {
            Ok(())
        } else {
            Err(TimingError::Invalid("mapped selection bounds"))
        }
    }
}
