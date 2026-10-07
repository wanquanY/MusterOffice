//! Shared exact movie-to-media edit mapping. Independent of video sample
//! selection and audio sample rate; neither packet tables nor edits prove that
//! their bytes actually decode. A caller must bind the resulting source ranges
//! to separately verified video/PCM coverage.
use crate::{
    Edit, TrackIndex,
    timing::{TimingError, TimingLimits},
};
use mo_time::{ExactInterval, ExactRatio as R, SampleSide};

type Result<T> = std::result::Result<T, TimingError>;
#[derive(Debug, Clone)]
pub struct MovieSegment {
    pub(crate) movie: ExactInterval,
    pub(crate) media: Option<R>,
    pub(crate) rate: R,
    pub(crate) index: Option<u32>,
}
impl MovieSegment {
    pub fn movie(&self) -> &ExactInterval {
        &self.movie
    }
    /// None is an explicit empty edit, not an unindexed or unavailable source.
    pub fn media_origin(&self) -> Option<&R> {
        self.media.as_ref()
    }
    pub fn rate(&self) -> &R {
        &self.rate
    }
    pub fn edit_index(&self) -> Option<u32> {
        self.index
    }
    /// Zero-rate (dwell) remains explicit. Video may hold a picture; audio must
    /// choose a documented policy rather than treating a single sample as a
    /// continuous sound. This method supplies timing, not that policy.
    pub fn media_time(&self, time: &R, bits: u64) -> Result<Option<R>> {
        self.media
            .as_ref()
            .map(|origin| {
                Ok(origin.add(
                    &time.sub(&self.movie.start, bits)?.mul(&self.rate, bits)?,
                    bits,
                )?)
            })
            .transpose()
    }
}
/// Minimal immutable clock metadata, also constructible from bounded index
/// pages without copying compressed sample tables into the embedding process.
#[derive(Debug, Clone, Copy)]
pub struct MovieClock<'a> {
    pub track_id: u32,
    pub movie_timescale: u32,
    pub media_timescale: u32,
    pub declared_track_duration: Option<u64>,
    pub edits: &'a [Edit],
}
#[derive(Debug, Clone)]
pub struct MovieTimeline {
    track_id: u32,
    movie_timescale: u32,
    media_timescale: u32,
    end: R,
    segments: Vec<MovieSegment>,
    bits: u64,
}
impl MovieTimeline {
    /// `unedited_media_end` is a caller-established raw endpoint in seconds,
    /// used only when the track has neither edits nor a declared track duration.
    /// Video derives it from composition order; audio can use decoded coverage.
    /// There is no average-FPS or compressed-packet-to-PCM inference here.
    pub fn build(
        track: &TrackIndex,
        movie_scale: u32,
        unedited_media_end: &R,
        limits: TimingLimits,
        check: &dyn Fn() -> bool,
    ) -> Result<Self> {
        Self::from_clock(
            MovieClock {
                track_id: track.id,
                movie_timescale: movie_scale,
                media_timescale: track.timescale,
                declared_track_duration: track.declared_track_duration,
                edits: &track.edits,
            },
            unedited_media_end,
            limits,
            check,
        )
    }
    pub fn from_clock(
        input: MovieClock<'_>,
        unedited_media_end: &R,
        limits: TimingLimits,
        check: &dyn Fn() -> bool,
    ) -> Result<Self> {
        crate::timing::guarded(|| {
            crate::timing::active(check)?;
            if input.movie_timescale == 0 || input.media_timescale == 0 || input.track_id == 0 {
                return Err(TimingError::Invalid("track clock"));
            }
            if limits.max_edits > 100_000
                || input.edits.len() > limits.max_edits
                || !(32..=4096).contains(&limits.exact_bits)
            {
                return Err(TimingError::Budget);
            }
            let bits = limits.exact_bits;
            unedited_media_end.check_budget(bits)?;
            if unedited_media_end.compare(&R::integer(0)).is_lt() {
                return Err(TimingError::Invalid("negative unedited media endpoint"));
            }
            let clock = |ticks: u64| -> Result<R> {
                Ok(R::new(ticks.into(), input.movie_timescale.into(), bits)?)
            };
            let mut segments = Vec::new();
            segments
                .try_reserve_exact(input.edits.len().max(1))
                .map_err(|_| TimingError::Budget)?;
            let end = if input.edits.is_empty() {
                let end = input
                    .declared_track_duration
                    .map(clock)
                    .transpose()?
                    .unwrap_or_else(|| unedited_media_end.clone());
                if end.positive() {
                    segments.push(MovieSegment {
                        movie: ExactInterval {
                            start: R::integer(0),
                            end: end.clone(),
                        },
                        media: Some(R::integer(0)),
                        rate: R::integer(1),
                        index: None,
                    });
                }
                end
            } else {
                let mut end = 0u64;
                for (i, edit) in input.edits.iter().enumerate() {
                    crate::timing::active(check)?;
                    if edit.movie_start != end || edit.media_time.is_some_and(|t| t < 0) {
                        return Err(TimingError::Invalid("edit clock sequence"));
                    }
                    if edit.rate_16_16 < 0 {
                        return Err(TimingError::Unsupported("negative container edit rate"));
                    }
                    end = end
                        .checked_add(edit.movie_duration)
                        .ok_or(TimingError::Invalid("edit end overflow"))?;
                    if edit.movie_duration != 0 {
                        segments.push(MovieSegment {
                            movie: ExactInterval {
                                start: clock(edit.movie_start)?,
                                end: clock(end)?,
                            },
                            media: edit
                                .media_time
                                .map(|t| R::new(t.into(), input.media_timescale.into(), bits))
                                .transpose()?,
                            rate: R::new(edit.rate_16_16.into(), 65536.into(), bits)?,
                            index: Some(i as u32),
                        });
                    }
                }
                clock(end)?
            };
            crate::timing::active(check)?;
            Ok(Self {
                track_id: input.track_id,
                movie_timescale: input.movie_timescale,
                media_timescale: input.media_timescale,
                end,
                segments,
                bits,
            })
        })
    }
    pub fn track_id(&self) -> u32 {
        self.track_id
    }
    pub fn movie_timescale(&self) -> u32 {
        self.movie_timescale
    }
    pub fn media_timescale(&self) -> u32 {
        self.media_timescale
    }
    pub fn end(&self) -> &R {
        &self.end
    }
    pub fn segments(&self) -> &[MovieSegment] {
        &self.segments
    }
    /// O(log edits) with exact before/after boundary ownership. A missing segment
    /// means outside the track, whereas a selected empty edit remains present.
    pub fn segment_at(
        &self,
        time: &R,
        side: SampleSide,
        check: &dyn Fn() -> bool,
    ) -> Result<Option<&MovieSegment>> {
        crate::timing::guarded(|| {
            crate::timing::active(check)?;
            time.check_budget(self.bits)?;
            let index = self.segments.partition_point(|s| match side {
                SampleSide::After => s.movie.end.compare(time).is_le(),
                SampleSide::Before => s.movie.end.compare(time).is_lt(),
            });
            let selected = self
                .segments
                .get(index)
                .filter(|s| s.movie.contains(time, side));
            crate::timing::active(check)?;
            Ok(selected)
        })
    }
}
