//! Bounded exact video presentation using an indexed container timeline or an
//! explicitly host-attested continuous source span.
//! No average-FPS indexing, color conversion, storage, or implicit timestamp repair.
mod frame;
pub mod indexed;
#[cfg(feature = "native")]
pub mod native;
mod sampler;
mod timing;
use mo_common::RationalTime;
use mo_media::{DecodeTimeline, DecodedFrame, StreamInfo};
use mo_video_model::TimeRange;
pub use sampler::VideoSampler;
use std::sync::Arc;

/// Decoder output must be in presentation order, with exact source PTS. Each
/// successful seek increments generation. A new implementation starts at origin.
/// Callback/decoder calls must cooperate with cancellation and enforce their own
/// execution deadlines; this layer only bounds frame work and retained payload.
pub trait VideoDecoder {
    type Error: std::error::Error + 'static;
    fn stream(&self) -> &StreamInfo;
    fn generation(&self) -> u64;
    fn timeline(&self) -> DecodeTimeline;
    fn next(&mut self, cancelled: &dyn Fn() -> bool) -> Result<Option<DecodedFrame>, Self::Error>;
    fn seek(&mut self, ticks: i64, cancelled: &dyn Fn() -> bool) -> Result<(), Self::Error>;
}
#[derive(Debug, Clone, Copy)]
pub struct SourceLimits {
    pub exact_bits: u64,
    /// Each decoder frame must fit. Retention reserves space BEFORE reading it.
    pub max_frame_bytes: usize,
    /// Active plane bytes retained here, including lookahead; excludes caller-owned clones.
    pub max_retained_bytes: usize,
    /// Continuous sampling needs two for lookahead; indexed sampling needs one.
    pub max_retained_frames: usize,
    pub max_decode_calls_per_query: usize,
}
impl Default for SourceLimits {
    fn default() -> Self {
        Self {
            exact_bits: 256,
            max_frame_bytes: 32 << 20,
            max_retained_bytes: 128 << 20,
            max_retained_frames: 120,
            max_decode_calls_per_query: 1000,
        }
    }
}
#[derive(Debug, Clone, Copy)]
pub enum SampleMode {
    /// Reuse sequential decoding; a backwards cache miss still seeks.
    Continue,
    /// Seek on a cache miss. Scrubbing does not decode every intervening frame.
    Seek,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EndEvidence {
    NextPresentationTimestamp,
    /// Explicit, host-attested continuous-span end, never average FPS or model duration.
    ContinuousSpanEnd,
}
#[derive(Debug, Clone)]
pub struct PresentedFrame {
    pub interval: TimeRange,
    pub end_evidence: EndEvidence,
    /// Original PTS, duration, color tags, format and owned planes are preserved.
    pub frame: Arc<DecodedFrame>,
}
#[derive(Debug, Clone)]
pub enum SourceSelection {
    Frame(PresentedFrame),
    Gap(TimeRange),
    OutsideSpan,
}
#[derive(Debug, Default, Clone, Copy)]
pub struct SourceStats {
    pub decoded_frames: usize,
    pub seeks: usize,
    pub cache_hit: bool,
    pub retained_bytes: usize,
}
#[derive(Debug)]
pub struct SampledVideo {
    pub selection: SourceSelection,
    pub stats: SourceStats,
}
#[derive(Debug, thiserror::Error)]
pub enum SourceError<E: std::error::Error + 'static> {
    #[error("video source decoder: {0}")]
    Decoder(E),
    #[error(transparent)]
    Time(#[from] mo_time::TimeError),
    #[error(transparent)]
    Compile(#[from] mo_video_compile::CompileError),
    #[error("invalid presentation source: {0}")]
    Invalid(&'static str),
    #[error("unsupported presentation source: {0}")]
    Unsupported(&'static str),
    #[error("video source budget exceeded")]
    Budget,
    #[error("video source cancelled")]
    Cancelled,
    #[error("video source callback panicked")]
    CallbackPanic,
    #[error("video source is stopped")]
    Stopped,
}
pub(crate) struct Pending {
    pub start: RationalTime,
    pub frame: Arc<DecodedFrame>,
    pub bytes: usize,
}
