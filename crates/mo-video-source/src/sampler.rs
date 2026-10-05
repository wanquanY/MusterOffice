use crate::{
    EndEvidence, Pending, PresentedFrame, SampleMode, SampledVideo, SourceError, SourceLimits,
    SourceSelection, SourceStats, VideoDecoder, timing,
};
use mo_common::RationalTime;
use mo_media::{MediaKind, TimeBase};
use mo_time::ExactRatio;
use mo_video_compile::{
    MediaBinding, PresentationIndex, PresentationLimits, PresentationSample, PresentationSelection,
    PresentationWindow,
};
use mo_video_model::{MediaStream, SampleSide, TimeRange, TimeSample};
use std::{
    collections::VecDeque,
    panic::{AssertUnwindSafe, catch_unwind},
    sync::Arc,
};

type Result<T, D> = std::result::Result<T, SourceError<<D as VideoDecoder>::Error>>;

/// One explicitly attested continuous presentation span. This does not infer
/// continuity from file duration or parse edit lists. Host demux/probe supplies
/// spans and gaps; never stretch a span across a known source discontinuity.
pub struct VideoSampler<D: VideoDecoder> {
    decoder: Option<D>,
    binding: MediaBinding,
    span: TimeRange,
    limits: SourceLimits,
    time_base: TimeBase,
    origin: i64,
    generation: u64,
    cache: VecDeque<PresentedFrame>,
    cached_bytes: usize,
    pending: Option<Pending>,
    last_pts: Option<RationalTime>,
    first_pts: Option<RationalTime>,
    prefix_gap: Option<TimeRange>,
    at_origin: bool,
    done: bool,
}
#[derive(Default)]
struct Work {
    stats: SourceStats,
    reads: usize,
}
impl<D: VideoDecoder> VideoSampler<D> {
    pub fn new(
        decoder: D,
        binding: MediaBinding,
        span: TimeRange,
        limits: SourceLimits,
    ) -> Result<Self, D> {
        let MediaStream::Video {
            available,
            dimensions,
            ..
        } = binding.descriptor
        else {
            return Err(SourceError::Invalid("video stream required"));
        };
        if !span.valid()
            || !available.valid()
            || !available.contains_boundary(span.start)
            || !available.contains_boundary(span.end)
            || dimensions.width == 0
            || dimensions.height == 0
        {
            return Err(SourceError::Invalid("continuous span or dimensions"));
        }
        if !(128..=4096).contains(&limits.exact_bits)
            || limits.max_frame_bytes == 0
            || limits.max_frame_bytes > 512 << 20
            || limits.max_retained_bytes > 1 << 30
            || limits.max_retained_frames < 2
            || limits.max_retained_frames > 100_000
            || limits.max_decode_calls_per_query == 0
            || limits.max_decode_calls_per_query > 1_000_000
            || limits
                .max_frame_bytes
                .checked_mul(2)
                .is_none_or(|n| n > limits.max_retained_bytes)
        {
            return Err(SourceError::Invalid("source limits"));
        }
        let stream = decoder.stream();
        let time_base = stream
            .time_base
            .ok_or(SourceError::Invalid("unknown stream timebase"))?;
        let origin = stream
            .start_ticks
            .ok_or(SourceError::Invalid("unknown seek origin"))?;
        timing::timestamp(origin, time_base, limits.exact_bits)?;
        if stream.kind != MediaKind::Video || decoder.generation() == 0 {
            return Err(SourceError::Invalid("decoder kind or generation"));
        }
        let generation = decoder.generation();
        Ok(Self {
            decoder: Some(decoder),
            binding,
            span,
            limits,
            time_base,
            origin,
            generation,
            cache: VecDeque::new(),
            cached_bytes: 0,
            pending: None,
            last_pts: None,
            first_pts: None,
            prefix_gap: None,
            at_origin: true,
            done: false,
        })
    }
    pub fn is_stopped(&self) -> bool {
        self.decoder.is_none()
    }
    pub fn retained_bytes(&self) -> usize {
        self.cached_bytes + self.pending.as_ref().map_or(0, |p| p.bytes)
    }
    pub fn sample(
        &mut self,
        binding: &MediaBinding,
        sample: &TimeSample,
        mode: SampleMode,
        cancelled: &dyn Fn() -> bool,
    ) -> Result<SampledVideo, D> {
        if self.decoder.is_none() {
            return Err(SourceError::Stopped);
        }
        let result = match catch_unwind(AssertUnwindSafe(|| {
            self.sample_inner(binding, sample, mode, cancelled)
        })) {
            Ok(result) => result,
            Err(payload) => {
                std::mem::forget(payload);
                Err(SourceError::CallbackPanic)
            }
        };
        if result.is_err() {
            self.decoder.take();
            self.pending.take();
            self.cache.clear();
            self.cached_bytes = 0;
        }
        result
    }
    fn sample_inner(
        &mut self,
        binding: &MediaBinding,
        sample: &TimeSample,
        mode: SampleMode,
        cancelled: &dyn Fn() -> bool,
    ) -> Result<SampledVideo, D> {
        ensure_active::<D>(cancelled)?;
        if binding != &self.binding {
            return Err(SourceError::Invalid("changed source binding"));
        }
        sample.time.check_budget(self.limits.exact_bits)?;
        let mut work = Work::default();
        if !sample.covers(self.span) {
            return Ok(SampledVideo {
                selection: SourceSelection::OutsideSpan,
                stats: work.stats,
            });
        }
        if let Some(selection) = self.lookup(sample, cancelled)? {
            ensure_active::<D>(cancelled)?;
            work.stats.cache_hit = true;
            work.stats.retained_bytes = self.retained_bytes();
            return Ok(SampledVideo {
                selection,
                stats: work.stats,
            });
        }
        let before_pending = self
            .pending
            .as_ref()
            .is_some_and(|p| before(sample, p.start));
        if matches!(mode, SampleMode::Seek) || before_pending || self.done {
            let tick =
                timing::seek_tick(sample, self.time_base, self.limits.exact_bits)?.max(self.origin);
            self.seek(tick, &mut work, cancelled)?;
        }
        loop {
            ensure_active::<D>(cancelled)?;
            self.step(&mut work, cancelled)?;
            if let Some(selection) = self.lookup(sample, cancelled)? {
                ensure_active::<D>(cancelled)?;
                work.stats.retained_bytes = self.retained_bytes();
                return Ok(SampledVideo {
                    selection,
                    stats: work.stats,
                });
            }
            // A seek returning only later frames proves no preceding coverage.
            // Rewind once, under this same work/cancellation budget, rather than
            // call an unindexed prefix a gap or substitute the nearest picture.
            if !self.at_origin && (self.done || self.first_pts.is_some_and(|t| before(sample, t))) {
                self.seek(self.origin, &mut work, cancelled)?;
            } else if self.done {
                return Err(SourceError::Invalid("source span has no proven coverage"));
            }
        }
    }
    fn seek(&mut self, tick: i64, work: &mut Work, check: &dyn Fn() -> bool) -> Result<(), D> {
        if work.stats.seeks >= 2 {
            return Err(SourceError::Budget);
        }
        ensure_active::<D>(check)?;
        let expected = self.generation.checked_add(1).ok_or(SourceError::Budget)?;
        let decoder = self.decoder.as_mut().ok_or(SourceError::Stopped)?;
        decoder.seek(tick, check).map_err(SourceError::Decoder)?;
        if decoder.generation() != expected {
            return Err(SourceError::Invalid("seek generation"));
        }
        self.generation = expected;
        self.cache.clear();
        self.cached_bytes = 0;
        self.pending = None;
        self.last_pts = None;
        self.first_pts = None;
        self.prefix_gap = None;
        self.at_origin = tick == self.origin;
        self.done = false;
        work.stats.seeks += 1;
        ensure_active::<D>(check)
    }
    fn lookup(
        &self,
        sample: &TimeSample,
        check: &dyn Fn() -> bool,
    ) -> Result<Option<SourceSelection>, D> {
        if let Some(gap) = self.prefix_gap
            && sample.covers(gap)
        {
            return Ok(Some(SourceSelection::Gap(gap)));
        }
        let (Some(first), Some(last)) = (self.cache.front(), self.cache.back()) else {
            return Ok(None);
        };
        let index = PresentationIndex::new(
            PresentationWindow {
                resource_digest: self.binding.digest.clone(),
                stream: self.binding.stream.clone(),
                coverage: TimeRange {
                    start: first.interval.start,
                    end: last.interval.end,
                },
                samples: self
                    .cache
                    .iter()
                    .enumerate()
                    .map(|(i, f)| PresentationSample {
                        id: i as u32,
                        interval: f.interval,
                    })
                    .collect(),
            },
            PresentationLimits {
                max_samples: self.limits.max_retained_frames,
                exact_bits: self.limits.exact_bits,
            },
            check,
        )?;
        match index.select(&self.binding, sample, check)? {
            PresentationSelection::Sample { sample } => Ok(Some(SourceSelection::Frame(
                self.cache[sample.id as usize].clone(),
            ))),
            PresentationSelection::OutsideWindow => Ok(None),
            PresentationSelection::Gap => Err(SourceError::Invalid(
                "gap inside a declared continuous span",
            )),
        }
    }
    fn step(&mut self, work: &mut Work, check: &dyn Fn() -> bool) -> Result<(), D> {
        if self.done {
            return Ok(());
        }
        let current = match self.pending.take() {
            Some(p) => p,
            None => match self.read(0, 0, work, check)? {
                Some(p) => p,
                None => {
                    self.done = true;
                    if self.at_origin {
                        self.prefix_gap = Some(self.span);
                    }
                    return Ok(());
                }
            },
        };
        if current.start.compare_time(self.span.end).is_ge() {
            self.done = true;
            return Ok(());
        }
        let next = self.read(current.bytes, 1, work, check)?;
        let (end, evidence) = match &next {
            Some(p) if p.start.compare_time(self.span.end).is_lt() => {
                (p.start, EndEvidence::NextPresentationTimestamp)
            }
            _ => (self.span.end, EndEvidence::ContinuousSpanEnd),
        };
        let interval = TimeRange {
            start: timing::max(current.start, self.span.start),
            end,
        };
        if interval.valid() {
            self.cached_bytes += current.bytes;
            self.cache.push_back(PresentedFrame {
                interval,
                end_evidence: evidence,
                frame: current.frame,
            });
        }
        self.done = next
            .as_ref()
            .is_none_or(|p| p.start.compare_time(self.span.end).is_ge());
        self.pending = if self.done { None } else { next };
        Ok(())
    }
    fn read(
        &mut self,
        held_bytes: usize,
        held_frames: usize,
        work: &mut Work,
        cancelled: &dyn Fn() -> bool,
    ) -> Result<Option<Pending>, D> {
        ensure_active::<D>(cancelled)?;
        if work.reads >= self.limits.max_decode_calls_per_query {
            return Err(SourceError::Budget);
        }
        work.reads += 1;
        while self.cached_bytes + held_bytes + self.limits.max_frame_bytes
            > self.limits.max_retained_bytes
            || self.cache.len() + held_frames + 1 > self.limits.max_retained_frames
        {
            let Some(frame) = self.cache.pop_front() else {
                return Err(SourceError::Budget);
            };
            self.cached_bytes -= payload_bytes(&frame.frame).ok_or(SourceError::Budget)?;
        }
        let decoder = self.decoder.as_mut().ok_or(SourceError::Stopped)?;
        let frame = decoder.next(cancelled).map_err(SourceError::Decoder)?;
        ensure_active::<D>(cancelled)?;
        let Some(frame) = frame else {
            return Ok(None);
        };
        work.stats.decoded_frames += 1;
        let bytes = payload_bytes(&frame).ok_or(SourceError::Invalid("frame plane layout"))?;
        let MediaStream::Video { dimensions, .. } = self.binding.descriptor else {
            unreachable!()
        };
        if bytes > self.limits.max_frame_bytes {
            return Err(SourceError::Budget);
        }
        if frame.info.kind != MediaKind::Video
            || frame.info.generation != self.generation
            || frame.info.width != Some(dimensions.width)
            || frame.info.height != Some(dimensions.height)
        {
            return Err(SourceError::Invalid("frame dimensions, kind or generation"));
        }
        let pts = frame
            .info
            .pts_ticks
            .ok_or(SourceError::Invalid("missing presentation timestamp"))?;
        let start = timing::timestamp(pts, frame.info.time_base, self.limits.exact_bits)?;
        if self
            .last_pts
            .is_some_and(|previous| previous.compare_time(start).is_ge())
        {
            return Err(SourceError::Invalid(
                "duplicate or nonmonotonic presentation timestamps",
            ));
        }
        self.last_pts = Some(start);
        if self.first_pts.is_none() {
            self.first_pts = Some(start);
            let prefix = TimeRange {
                start: self.span.start,
                end: timing::min(start, self.span.end),
            };
            if self.at_origin && prefix.valid() {
                self.prefix_gap = Some(prefix);
            }
        }
        Ok(Some(Pending {
            start,
            frame: Arc::new(frame),
            bytes,
        }))
    }
}
fn before(sample: &TimeSample, time: RationalTime) -> bool {
    let order = sample.time.compare(&ExactRatio::from_time(time));
    order.is_lt() || (order.is_eq() && sample.side == SampleSide::Before)
}
fn ensure_active<D: VideoDecoder>(callback: &dyn Fn() -> bool) -> Result<(), D> {
    if callback() {
        Err(SourceError::Cancelled)
    } else {
        Ok(())
    }
}
fn payload_bytes(frame: &mo_media::DecodedFrame) -> Option<usize> {
    if frame.planes.is_empty() || frame.planes.len() > 4 {
        return None;
    }
    frame.planes.iter().try_fold(0usize, |sum, p| {
        let n = p.row_bytes.checked_mul(p.rows)?;
        if n == 0 || n != p.bytes.len() {
            return None;
        }
        sum.checked_add(n)
    })
}
