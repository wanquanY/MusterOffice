//! Media byte capabilities and decoded values without any native codec linkage.
//! Codec metadata is preserved; this is not a document or color-transform contract.
mod encode;
pub use encode::*;
use serde::{Deserialize, Serialize};
use std::{fmt, io};

/// A host capability over immutable resource bytes. No path or URL is accepted.
/// A successful read may be short; zero is valid only at the declared resource end.
/// Callbacks must return promptly. Hard time limits belong to the enclosing worker.
pub trait ReadAt {
    fn read_at(&mut self, offset: u64, destination: &mut [u8]) -> io::Result<usize>;
}
impl<F: FnMut(u64, &mut [u8]) -> io::Result<usize>> ReadAt for F {
    fn read_at(&mut self, offset: u64, destination: &mut [u8]) -> io::Result<usize> {
        self(offset, destination)
    }
}

/// Explicit per-session admission/work limits, including total copied frame bytes.
/// These are not a bound on all allocations inside a vendor decoder.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DecodeLimits {
    pub max_input_bytes: u64,
    pub max_read_bytes: u64,
    pub max_frame_bytes: u64,
    pub max_streams: u32,
    pub max_pixels: u32,
    pub max_audio_samples: u32,
    pub max_channels: u32,
    pub max_decode_steps: u32,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub enum ErrorKind {
    Invalid,
    Io,
    Cancelled,
    Limit,
    Unsupported,
    Decode,
    Encode,
    State,
    CallbackPanic,
    CallbackContract,
    Abi,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MediaError {
    pub kind: ErrorKind,
    pub upstream_code: Option<i32>,
    pub message: String,
}
impl MediaError {
    pub fn new(kind: ErrorKind, message: impl Into<String>) -> Self {
        Self {
            kind,
            upstream_code: None,
            message: message.into(),
        }
    }
}
impl fmt::Display for MediaError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "media {:?}: {}", self.kind, self.message)
    }
}
impl std::error::Error for MediaError {}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub enum MediaKind {
    Video,
    Audio,
    Other,
}
/// Container timestamp policy. Raw ISO BMFF keeps edits for the shared exact
/// mapper instead of letting the demuxer rewrite them. Neither policy proves
/// continuity or converts compressed sample durations into frame durations.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum DecodeTimeline {
    Demuxer,
    RawIsoBmff,
}
/// Source ticks, deliberately not rescaled into average FPS or floating seconds.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TimeBase {
    pub numerator: u32,
    pub denominator: u32,
}
/// The pinned codec's numeric color tags; unknown/reserved values are retained.
/// This is a native adapter type, not the portable document/color contract.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CodecColorTags {
    pub primaries: i32,
    pub transfer: i32,
    pub matrix: i32,
    pub range: i32,
    pub chroma_location: i32,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct StreamInfo {
    pub index: u32,
    /// ISO BMFF tkhd ID, independent from decoder array index. Other container
    /// identity schemes require their own explicit adapter.
    pub container_track_id: Option<u32>,
    pub kind: MediaKind,
    pub codec: String,
    pub time_base: Option<TimeBase>,
    pub start_ticks: Option<i64>,
    pub duration_ticks: Option<i64>,
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub sample_rate: Option<u32>,
    pub channels: Option<u32>,
    pub nominal_frame_rate: Option<TimeBase>,
    pub sample_aspect_ratio: Option<TimeBase>,
    pub colors: CodecColorTags,
    pub channel_layout: String,
    /// FFmpeg's fixed-point container display matrix, preserved without rounding.
    pub display_matrix: Option<[i32; 9]>,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FrameInfo {
    pub generation: u64,
    pub kind: MediaKind,
    pub time_base: TimeBase,
    pub pts_ticks: Option<i64>,
    pub duration_ticks: Option<i64>,
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub sample_rate: Option<u32>,
    pub channels: Option<u32>,
    pub samples: Option<u32>,
    pub format: String,
    pub channel_layout: String,
    pub colors: CodecColorTags,
    pub sample_aspect_ratio: Option<TimeBase>,
    /// Pinned codec alpha mode; never implies a premultiplication conversion.
    pub alpha_mode: i32,
    pub key_frame: bool,
    pub interlaced: bool,
    pub top_field_first: bool,
}
#[derive(Debug, PartialEq, Eq)]
pub struct FramePlane {
    pub row_bytes: usize,
    pub rows: usize,
    /// Contiguous active rows only; stride equals row_bytes. No decoder padding.
    pub bytes: Vec<u8>,
}
#[derive(Debug, PartialEq, Eq)]
pub struct DecodedFrame {
    pub info: FrameInfo,
    /// Independently owned; next/seek/drop cannot invalidate these bytes.
    pub planes: Vec<FramePlane>,
}
