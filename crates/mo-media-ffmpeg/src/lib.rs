//! Safe, thread-affine access to the fixed native media component.
//!
//! This is a leaf adapter for a killable worker, **not** a hostile-input sandbox.
//! Vendor allocations/calls require process limits outside this crate. It does
//! not open files/URLs, infer missing timestamps, or select presentation frames.
//!
//! Opening owns the host read/cancel capabilities until native close. Selecting
//! consumes the container; frames own copied active planes independently. Every
//! operation error closes the native context, so no stale frame can be retrieved.
//! Catches use Rust unwind semantics; panic=abort still terminates the worker.
//!
//! A decoder cannot cross threads, even with thread-safe resource callbacks:
//! ```compile_fail
//! fn requires_send<T: Send>() {}
//! requires_send::<mo_media_ffmpeg::Decoder<'static>>();
//! ```
//! ```compile_fail
//! fn requires_sync<T: Sync>() {}
//! requires_sync::<mo_media_ffmpeg::Decoder<'static>>();
//! ```
//! Selecting consumes the container, preventing a second decoder selection:
//! ```compile_fail
//! fn select_twice(container: mo_media_ffmpeg::Container<'_>) {
//!     let _first = container.select(0);
//!     let _second = container.select(1);
//! }
//! ```
pub use mo_media::*;
mod encoder;
pub use encoder::*;
#[allow(unsafe_code)]
mod ffi;

pub struct Container<'a> {
    context: ffi::Context<'a>,
    streams: Vec<StreamInfo>,
}
impl<'a> Container<'a> {
    pub fn open(
        source: impl ReadAt + 'a,
        byte_length: u64,
        limits: DecodeLimits,
        cancelled: impl FnMut() -> bool + 'a,
    ) -> Result<Self, MediaError> {
        Self::open_with_timeline(
            source,
            byte_length,
            limits,
            DecodeTimeline::Demuxer,
            cancelled,
        )
    }
    pub fn open_with_timeline(
        source: impl ReadAt + 'a,
        byte_length: u64,
        limits: DecodeLimits,
        timeline: DecodeTimeline,
        cancelled: impl FnMut() -> bool + 'a,
    ) -> Result<Self, MediaError> {
        let mut context = ffi::Context::open(source, byte_length, limits, timeline, cancelled)?;
        let streams = context.streams()?;
        Ok(Self { context, streams })
    }
    /// Header metadata can be incomplete. Decode to obtain actual frame metadata.
    pub fn streams(&self) -> &[StreamInfo] {
        &self.streams
    }
    pub fn select(mut self, index: u32) -> Result<Decoder<'a>, MediaError> {
        let stream = self
            .streams
            .iter()
            .find(|s| s.index == index)
            .cloned()
            .ok_or_else(|| MediaError::new(ErrorKind::Invalid, "stream index outside container"))?;
        self.context.select(index)?;
        Ok(Decoder {
            context: self.context,
            stream,
        })
    }
}
pub struct Decoder<'a> {
    context: ffi::Context<'a>,
    stream: StreamInfo,
}
impl Decoder<'_> {
    pub fn stream(&self) -> &StreamInfo {
        &self.stream
    }
    /// None is drained EOF, distinct from a failed or cancelled session.
    pub fn next_frame(&mut self) -> Result<Option<DecodedFrame>, MediaError> {
        self.context.next_frame()
    }
    /// Seek in selected stream ticks to an earlier random access point. The next
    /// frame belongs to a new generation and may precede the target timestamp.
    pub fn seek(&mut self, timestamp_ticks: i64) -> Result<(), MediaError> {
        self.context.seek(timestamp_ticks)
    }
    pub fn is_closed(&self) -> bool {
        self.context.is_closed()
    }
}
