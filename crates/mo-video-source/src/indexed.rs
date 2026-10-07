//! Container-edited presentation backed by exact raw decoded pictures.
//! The host binds timeline and decoder to the same immutable resource bytes.
mod cache;
use crate::{SampleMode, SourceError, SourceLimits, SourceStats, VideoDecoder, frame};
use mo_media::{DecodeTimeline, DecodedFrame, MediaKind, StreamInfo, TimeBase};
pub use mo_media_isobmff::presentation::{
    ExactRatio, Interval, Side, TimingError, VideoAccess, VideoTimeSelection, VideoTiming,
    VideoTimingSummary,
};
use mo_video_compile::MediaBinding;
use mo_video_model::{MediaStream, TimeSample};
use std::{
    panic::{AssertUnwindSafe, catch_unwind},
    sync::Arc,
};

/// The retained container map can run locally/WASM or inside a limited worker.
pub trait VideoTimeline {
    type Error: std::error::Error + 'static;
    fn summary(&self) -> &VideoTimingSummary;
    fn select(
        &mut self,
        time: &ExactRatio,
        side: Side,
        cancelled: &dyn Fn() -> bool,
    ) -> Result<VideoTimeSelection, Self::Error>;
}
#[derive(Clone, Copy, Debug)]
pub struct DecodeTarget {
    pub sample: u32,
    pub decode_start: i64,
    pub access_sample: u32,
    pub access_start: i64,
    pub presentation_start: i64,
}
/// Backend-specific random access in original sample order. The implementation
/// must start early enough to decode this access unit, including codec preroll.
/// It must not reinterpret raw PTS as an interchangeable decode-clock position.
pub trait IndexedVideoDecoder: VideoDecoder {
    fn seek_sample(
        &mut self,
        target: DecodeTarget,
        cancelled: &dyn Fn() -> bool,
    ) -> Result<(), Self::Error>;
}
impl VideoTimeline for VideoTiming {
    type Error = TimingError;
    fn summary(&self) -> &VideoTimingSummary {
        self.summary()
    }
    fn select(
        &mut self,
        time: &ExactRatio,
        side: Side,
        cancelled: &dyn Fn() -> bool,
    ) -> Result<VideoTimeSelection, TimingError> {
        VideoTiming::select(self, time, side, cancelled)
    }
}
#[derive(Debug, thiserror::Error)]
pub enum IndexedSourceError<D: std::error::Error + 'static, T: std::error::Error + 'static> {
    #[error(transparent)]
    Source(#[from] SourceError<D>),
    #[error("container timeline: {0}")]
    Timeline(T),
    #[error(transparent)]
    Timing(#[from] TimingError),
}
type Error<D, T> = IndexedSourceError<<D as VideoDecoder>::Error, <T as VideoTimeline>::Error>;
type ResultFor<V, D, T> = Result<V, Error<D, T>>;

/// The timing remains exact even when an edit boundary cannot fit RationalTime.
/// A picture exists iff the selection is Sample; its original metadata is intact.
#[derive(Debug)]
pub struct IndexedSampledVideo {
    selection: VideoTimeSelection,
    frame: Option<Arc<DecodedFrame>>,
    pub stats: SourceStats,
}
impl IndexedSampledVideo {
    pub fn selection(&self) -> &VideoTimeSelection {
        &self.selection
    }
    pub fn frame(&self) -> Option<&Arc<DecodedFrame>> {
        self.frame.as_ref()
    }
    pub fn into_frame(self) -> Option<Arc<DecodedFrame>> {
        self.frame
    }
}

/// Every returned picture must have the exact PTS selected by the container.
/// Missing pictures are errors, never fabricated gaps or extended prior frames.
/// Cache ownership excludes caller-held Arc clones and decoder-private memory.
pub struct IndexedVideoSampler<D: IndexedVideoDecoder, T: VideoTimeline> {
    decoder: Option<D>,
    timeline: Option<T>,
    summary: VideoTimingSummary,
    binding: MediaBinding,
    stream: StreamInfo,
    limits: SourceLimits,
    generation: u64,
    last_pts: Option<i64>,
    last_target: Option<DecodeTarget>,
    cache: cache::Cache,
}
impl<D: IndexedVideoDecoder, T: VideoTimeline> IndexedVideoSampler<D, T> {
    pub fn new(
        decoder: D,
        timeline: T,
        binding: MediaBinding,
        limits: SourceLimits,
    ) -> ResultFor<Self, D, T> {
        if !frame::valid_limits(limits, 1) {
            return Err(SourceError::Invalid("indexed source limits").into());
        }
        let summary = timeline.summary().clone();
        summary.validate(limits.exact_bits)?;
        let MediaStream::Video {
            available,
            dimensions,
            ..
        } = binding.descriptor
        else {
            return Err(SourceError::Invalid("video binding required").into());
        };
        if !available.valid()
            || dimensions.width == 0
            || dimensions.height == 0
            || !ExactRatio::from_time(available.start)
                .compare(&ExactRatio::integer(0))
                .is_eq()
            || !ExactRatio::from_time(available.end)
                .compare(&summary.movie_end)
                .is_eq()
        {
            return Err(SourceError::Invalid(
                "binding must describe the complete container movie interval",
            )
            .into());
        }
        let stream = decoder.stream();
        if decoder.timeline() != DecodeTimeline::RawIsoBmff
            || decoder.generation() != 1
            || stream.kind != MediaKind::Video
            || stream.width != Some(dimensions.width)
            || stream.height != Some(dimensions.height)
            || stream.container_track_id != Some(summary.track_id)
            || stream.time_base
                != Some(TimeBase {
                    numerator: 1,
                    denominator: summary.media_timescale,
                })
        {
            return Err(SourceError::Invalid("raw video decoder identity or clock").into());
        }
        let stream = stream.clone();
        Ok(Self {
            decoder: Some(decoder),
            timeline: Some(timeline),
            summary,
            binding,
            stream,
            limits,
            generation: 1,
            last_pts: None,
            last_target: None,
            cache: cache::Cache::default(),
        })
    }
    pub fn is_stopped(&self) -> bool {
        self.decoder.is_none()
    }
    /// Immutable metadata admitted with this decoder and binding.
    pub fn stream(&self) -> &StreamInfo {
        &self.stream
    }
    pub fn binding(&self) -> &MediaBinding {
        &self.binding
    }
    pub fn retained_bytes(&self) -> usize {
        self.cache.bytes
    }
    pub fn sample(
        &mut self,
        binding: &MediaBinding,
        sample: &TimeSample,
        mode: SampleMode,
        cancelled: &dyn Fn() -> bool,
    ) -> ResultFor<IndexedSampledVideo, D, T> {
        if self.is_stopped() {
            return Err(SourceError::Stopped.into());
        }
        let result = match catch_unwind(AssertUnwindSafe(|| {
            self.sample_inner(binding, sample, mode, cancelled)
        })) {
            Ok(result) => result,
            Err(payload) => {
                std::mem::forget(payload);
                Err(SourceError::CallbackPanic.into())
            }
        };
        if result.is_err() {
            self.decoder.take();
            self.timeline.take();
            self.cache = cache::Cache::default();
        }
        result
    }
    fn active(cancelled: &dyn Fn() -> bool) -> ResultFor<(), D, T> {
        if cancelled() {
            Err(SourceError::Cancelled.into())
        } else {
            Ok(())
        }
    }
    fn sample_inner(
        &mut self,
        binding: &MediaBinding,
        sample: &TimeSample,
        mode: SampleMode,
        cancelled: &dyn Fn() -> bool,
    ) -> ResultFor<IndexedSampledVideo, D, T> {
        Self::active(cancelled)?;
        if binding != &self.binding {
            return Err(SourceError::Invalid("changed indexed source binding").into());
        }
        sample
            .time
            .check_budget(self.limits.exact_bits)
            .map_err(SourceError::Time)?;
        let side = sample.side;
        let timeline = self.timeline.as_mut().ok_or(SourceError::Stopped)?;
        let selection = timeline
            .select(&sample.time, side, cancelled)
            .map_err(IndexedSourceError::Timeline)?;
        Self::active(cancelled)?;
        if timeline.summary() != &self.summary {
            return Err(SourceError::Invalid("changed container map").into());
        }
        selection.validate(&self.summary, &sample.time, side, self.limits.exact_bits)?;
        let mut stats = SourceStats::default();
        let picture = if let VideoTimeSelection::Sample {
            sample,
            decode_start,
            access,
            raw_start,
            ..
        } = selection
        {
            let access = access.ok_or(SourceError::Unsupported(
                "no indexed video access candidate",
            ))?;
            Some(self.picture(
                DecodeTarget {
                    sample,
                    decode_start,
                    access_sample: access.sample,
                    access_start: access.decode_start,
                    presentation_start: raw_start,
                },
                mode,
                &mut stats,
                cancelled,
            )?)
        } else {
            None
        };
        Self::active(cancelled)?;
        stats.retained_bytes = self.cache.bytes;
        Ok(IndexedSampledVideo {
            selection,
            frame: picture,
            stats,
        })
    }
    fn picture(
        &mut self,
        target: DecodeTarget,
        mode: SampleMode,
        stats: &mut SourceStats,
        cancelled: &dyn Fn() -> bool,
    ) -> ResultFor<Arc<DecodedFrame>, D, T> {
        let pts = target.presentation_start;
        if let Some(frame) = self.cache.get(pts).map_err(|_| SourceError::Budget)? {
            stats.cache_hit = true;
            return Ok(frame);
        }
        if matches!(mode, SampleMode::Seek)
            || self.last_pts.is_none_or(|last| last >= pts)
            || self.last_target.is_some_and(|last| {
                target.sample.saturating_sub(last.sample) as usize
                    >= self.limits.max_decode_calls_per_query
            })
        {
            self.seek(target, stats, cancelled)?;
        }
        for _ in 0..self.limits.max_decode_calls_per_query {
            Self::active(cancelled)?;
            // Reserve the worst allowed next frame before entering the decoder.
            while self.cache.bytes + self.limits.max_frame_bytes > self.limits.max_retained_bytes
                || self.cache.len() >= self.limits.max_retained_frames
            {
                if !self.cache.evict() {
                    return Err(SourceError::Budget.into());
                }
            }
            let decoder = self.decoder.as_mut().ok_or(SourceError::Stopped)?;
            let frame = decoder
                .next(cancelled)
                .map_err(SourceError::Decoder)?
                .ok_or(SourceError::Invalid(
                    "indexed picture missing at decoder EOF",
                ))?;
            Self::active(cancelled)?;
            if decoder.generation() != self.generation {
                return Err(SourceError::Invalid("decoder generation changed").into());
            }
            let (at, bytes) = self.validate_frame(&frame)?;
            if at > pts {
                return Err(SourceError::Invalid("decoder skipped indexed picture").into());
            }
            self.last_pts = Some(at);
            stats.decoded_frames += 1;
            let frame = Arc::new(frame);
            if !self.cache.insert(at, bytes, frame.clone()) {
                return Err(SourceError::Budget.into());
            }
            if at == pts {
                self.last_target = Some(target);
                return Ok(frame);
            }
        }
        Err(SourceError::Budget.into())
    }
    fn seek(
        &mut self,
        target: DecodeTarget,
        stats: &mut SourceStats,
        cancelled: &dyn Fn() -> bool,
    ) -> ResultFor<(), D, T> {
        Self::active(cancelled)?;
        let generation = self.generation.checked_add(1).ok_or(SourceError::Budget)?;
        let decoder = self.decoder.as_mut().ok_or(SourceError::Stopped)?;
        decoder
            .seek_sample(target, cancelled)
            .map_err(SourceError::Decoder)?;
        Self::active(cancelled)?;
        if decoder.generation() != generation {
            return Err(SourceError::Invalid("seek generation").into());
        }
        self.generation = generation;
        self.last_pts = None;
        stats.seeks += 1;
        // Immutable decoded pictures remain valid across decoder generations.
        Ok(())
    }
    fn validate_frame(&self, frame: &DecodedFrame) -> ResultFor<(i64, usize), D, T> {
        let bytes =
            frame::payload_bytes(frame).ok_or(SourceError::Invalid("frame plane layout"))?;
        if bytes > self.limits.max_frame_bytes {
            return Err(SourceError::Budget.into());
        }
        let MediaStream::Video { dimensions, .. } = self.binding.descriptor else {
            unreachable!()
        };
        let info = &frame.info;
        let pts = info
            .pts_ticks
            .ok_or(SourceError::Invalid("missing presentation timestamp"))?;
        if info.kind != MediaKind::Video
            || info.generation != self.generation
            || info.width != Some(dimensions.width)
            || info.height != Some(dimensions.height)
            || info.time_base
                != (TimeBase {
                    numerator: 1,
                    denominator: self.summary.media_timescale,
                })
            || self.last_pts.is_some_and(|last| last >= pts)
            || !self
                .summary
                .raw_span
                .is_some_and(|(start, end)| start <= pts && pts < end)
        {
            return Err(
                SourceError::Invalid("raw frame identity, layout or presentation clock").into(),
            );
        }
        Ok((pts, bytes))
    }
}
