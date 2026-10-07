//! Thin native implementation; the default crate builds without worker/codec linkage.
use crate::VideoDecoder;
use mo_media::{DecodeTimeline, DecodedFrame, ReadAt, StreamInfo};
use mo_native_media::{NativeContainer, NativeDecoder, WorkerBundle, WorkerError, WorkerOptions};

pub struct NativeVideoDecoder<'a> {
    decoder: NativeDecoder<'a>,
    generation: u64,
}
impl<'a> NativeVideoDecoder<'a> {
    pub fn process_id(&self) -> Option<u32> {
        self.decoder.process_id()
    }
    /// Select the actual tkhd identity, not the demuxer's stream-array ordinal.
    /// Host must bind these immutable source bytes to the same retained index.
    pub fn open_raw_track(
        bundle: &WorkerBundle,
        source: impl ReadAt + 'a,
        length: u64,
        options: WorkerOptions,
        track_id: u32,
        cancelled: &dyn Fn() -> bool,
    ) -> Result<Self, WorkerError> {
        let container = NativeContainer::open_with_timeline(
            bundle,
            source,
            length,
            options,
            DecodeTimeline::RawIsoBmff,
            cancelled,
        )?;
        let index = container
            .streams()
            .iter()
            .find(|s| s.container_track_id == Some(track_id))
            .ok_or(WorkerError::Invalid(
                "indexed video track absent from decoder",
            ))?
            .index;
        Ok(Self {
            decoder: container.select(index, cancelled)?,
            generation: 1,
        })
    }
    /// Always opens a fresh decoder at source origin. Host binds this selected
    /// stream index and immutable bytes to the sampler's resource/stream identity.
    pub fn open(
        bundle: &WorkerBundle,
        source: impl ReadAt + 'a,
        length: u64,
        options: WorkerOptions,
        stream_index: u32,
        cancelled: &dyn Fn() -> bool,
    ) -> Result<Self, WorkerError> {
        let decoder = NativeContainer::open(bundle, source, length, options, cancelled)?
            .select(stream_index, cancelled)?;
        Ok(Self {
            decoder,
            generation: 1,
        })
    }
}
impl VideoDecoder for NativeVideoDecoder<'_> {
    type Error = WorkerError;
    fn timeline(&self) -> DecodeTimeline {
        self.decoder.timeline()
    }
    fn stream(&self) -> &StreamInfo {
        self.decoder.stream()
    }
    fn generation(&self) -> u64 {
        self.generation
    }
    fn next(&mut self, cancelled: &dyn Fn() -> bool) -> Result<Option<DecodedFrame>, WorkerError> {
        self.decoder.next_frame(cancelled)
    }
    fn seek(&mut self, ticks: i64, cancelled: &dyn Fn() -> bool) -> Result<(), WorkerError> {
        self.decoder.seek(ticks, cancelled)?;
        self.generation = self
            .generation
            .checked_add(1)
            .ok_or(WorkerError::Invalid("generation overflow"))?;
        Ok(())
    }
}
impl crate::indexed::IndexedVideoDecoder for NativeVideoDecoder<'_> {
    fn seek_sample(
        &mut self,
        target: crate::indexed::DecodeTarget,
        cancelled: &dyn Fn() -> bool,
    ) -> Result<(), WorkerError> {
        if self.timeline() != DecodeTimeline::RawIsoBmff
            || target.access_start < 0
            || target.access_start > target.decode_start
            || target.access_sample > target.sample
        {
            return Err(WorkerError::Invalid("raw indexed decode target"));
        }
        // In the fixed MOV demuxer with edit lists disabled, min_corrected_pts
        // is zero. mov_seek_stream subtracts dts_shift before searching its DTS
        // index. Pass the indexed candidate's original DTS, chosen no later
        // than the requested picture in both orders. Using picture PTS or just
        // the target sample DTS can select a later GOP or miss leading B frames.
        self.seek(target.access_start, cancelled)
    }
}

/// A bounded timeline capability; dropping it releases the retained index worker.
pub struct NativeVideoTimeline<'a> {
    index: mo_native_media::NativeIndex<'a>,
    track: u32,
    summary: mo_native_media::VideoTimingSummary,
}
impl<'a> NativeVideoTimeline<'a> {
    pub fn new(
        mut index: mo_native_media::NativeIndex<'a>,
        track: u32,
        cancelled: &dyn Fn() -> bool,
    ) -> Result<Self, WorkerError> {
        let summary = index.describe_video_time(track, cancelled)?;
        Ok(Self {
            index,
            track,
            summary,
        })
    }
}
impl crate::indexed::VideoTimeline for NativeVideoTimeline<'_> {
    type Error = WorkerError;
    fn summary(&self) -> &mo_native_media::VideoTimingSummary {
        &self.summary
    }
    fn select(
        &mut self,
        time: &mo_native_media::ExactRatio,
        side: mo_native_media::MediaTimeSide,
        cancelled: &dyn Fn() -> bool,
    ) -> Result<mo_native_media::VideoTimeSelection, WorkerError> {
        self.index
            .select_video_time(self.track, time, side, cancelled)
    }
}
