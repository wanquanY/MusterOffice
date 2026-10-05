//! Private native media protocol. Headers are bounded JSON, planes/read payloads
//! are raw bytes. No paths, URLs, credentials or whole-source buffers are sent.
mod framing;
mod validation;
pub use framing::*;
use mo_media::{DecodeLimits, FrameInfo, MediaError, StreamInfo};
use mo_native_limits::{AppliedLimits, ProcessLimits};
use serde::{Deserialize, Serialize};
pub use validation::*;

pub const PROFILE: &str = "musteroffice.native-media-worker/1";
pub const MAX_HEADER_BYTES: usize = 64 * 1024;
pub const MAX_READ_BYTES: u32 = 32 * 1024;
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
    Open {
        profile: String,
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
    Opened {
        profile: String,
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
