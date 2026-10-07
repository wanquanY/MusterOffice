//! Private native media protocol. Headers are bounded JSON, planes/read payloads
//! are raw bytes. No paths, URLs, credentials or whole-source buffers are sent.
mod indexing;
pub use indexing::*;
mod encoding;
mod framing;
pub use encoding::*;
mod validation;
pub use framing::*;
use mo_media::{
    DecodeLimits, EncodeConfig, EncodeLimits, EncodeStats, FrameInfo, MediaError, StreamInfo,
};
use mo_native_limits::{AppliedLimits, ProcessLimits};
use serde::{Deserialize, Serialize};
pub use validation::*;

pub const PROFILE: &str = "musteroffice.native-media-worker/5";
pub const MAX_HEADER_BYTES: usize = 64 * 1024;
pub const MAX_READ_BYTES: u32 = 32 * 1024;
pub const MAX_WRITE_BYTES: u32 = 32 * 1024;
pub const MAX_PLANES: usize = 64;
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Request {
    pub call_id: u64,
    pub command: Command,
}
#[derive(Debug, Serialize, Deserialize)]
#[serde(
    tag = "type",
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub enum Command {
    DescribeVideoTime {
        track: u32,
    },
    SelectVideoTime {
        track: u32,
        time: mo_media_isobmff::presentation::ExactRatio,
        side: mo_media_isobmff::presentation::Side,
    },
    OpenIndex {
        profile: String,
        byte_length: u64,
        limits: mo_media_isobmff::IndexLimits,
        process_limits: ProcessLimits,
    },
    QueryIndex {
        query: IndexQuery,
    },
    OpenEncoder {
        profile: String,
        configuration: EncodeConfig,
        limits: EncodeLimits,
        process_limits: ProcessLimits,
    },
    EncodeVideo {
        pts: i64,
        duration: i64,
    },
    EncodeAudio {
        first_sample: i64,
        frames: u32,
    },
    FinishEncoder,
    WriteReply {
        write_id: u64,
        byte_length: u32,
    },
    Open {
        profile: String,
        timeline: mo_media::DecodeTimeline,
        byte_length: u64,
        limits: DecodeLimits,
        process_limits: ProcessLimits,
    },
    Select {
        index: u32,
    },
    Next,
    Seek {
        ticks: i64,
    },
    Close,
    ReadReply {
        read_id: u64,
        byte_length: u32,
    },
}
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Response {
    pub call_id: u64,
    pub event: Event,
}
#[derive(Debug, Serialize, Deserialize)]
#[serde(
    tag = "type",
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub enum Event {
    VideoTiming {
        summary: mo_media_isobmff::presentation::VideoTimingSummary,
    },
    VideoTimeSelected {
        selection: mo_media_isobmff::presentation::VideoTimeSelection,
    },
    Indexed {
        profile: String,
        summary: IndexSummary,
        applied: AppliedLimits,
    },
    IndexPage {
        page: IndexPage,
    },
    EncoderOpened {
        profile: String,
        configuration: EncodeConfig,
        applied: AppliedLimits,
    },
    VideoEncoded,
    AudioEncoded,
    Encoded {
        stats: EncodeStats,
    },
    Write {
        write_id: u64,
        offset: u64,
        byte_length: u32,
    },
    Opened {
        profile: String,
        timeline: mo_media::DecodeTimeline,
        streams: Vec<StreamInfo>,
        applied: AppliedLimits,
    },
    Selected,
    Frame {
        info: FrameInfo,
        planes: Vec<PlaneLayout>,
    },
    Eof,
    Seeked,
    Closed,
    Read {
        read_id: u64,
        offset: u64,
        byte_length: u32,
    },
    Error {
        error: MediaError,
    },
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PlaneLayout {
    pub row_bytes: u64,
    pub rows: u64,
}
