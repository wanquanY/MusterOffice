//! Only the fixed wrapper ABI, never FFmpeg's private structs.
use std::{
    ffi::{c_char, c_void},
    mem::{align_of, offset_of, size_of},
};
#[repr(C)]
pub struct Media {
    _private: [u8; 0],
}
#[repr(C)]
pub struct RawError {
    pub status: i32,
    pub upstream_code: i32,
    pub message: [c_char; 192],
}
#[repr(C)]
pub struct Io {
    pub opaque: *mut c_void,
    pub byte_length: u64,
    pub read_at: unsafe extern "C" fn(*mut c_void, u64, *mut u8, i32) -> i32,
    pub cancelled: unsafe extern "C" fn(*mut c_void) -> i32,
}
#[repr(C)]
pub struct Limits {
    pub max_input_bytes: u64,
    pub max_read_bytes: u64,
    pub max_frame_bytes: u64,
    pub max_streams: u32,
    pub max_pixels: u32,
    pub max_audio_samples: u32,
    pub max_channels: u32,
    pub max_decode_steps: u32,
}
#[repr(C)]
pub struct Stream {
    pub kind: i32,
    pub index: i32,
    pub time_base_num: i32,
    pub time_base_den: i32,
    pub start: i64,
    pub duration: i64,
    pub has_start: i32,
    pub has_duration: i32,
    pub width: i32,
    pub height: i32,
    pub sample_rate: i32,
    pub channels: i32,
    pub frame_rate_num: i32,
    pub frame_rate_den: i32,
    pub sar_num: i32,
    pub sar_den: i32,
    pub color_primaries: i32,
    pub color_transfer: i32,
    pub color_matrix: i32,
    pub color_range: i32,
    pub chroma_location: i32,
    pub has_display_matrix: i32,
    pub display_matrix: [i32; 9],
    pub codec: [c_char; 64],
    pub channel_layout: [c_char; 128],
}
#[repr(C)]
pub struct Frame {
    pub generation: u64,
    pub kind: i32,
    pub time_base_num: i32,
    pub time_base_den: i32,
    pub has_pts: i32,
    pub has_duration: i32,
    pub pts: i64,
    pub duration: i64,
    pub width: i32,
    pub height: i32,
    pub sample_rate: i32,
    pub channels: i32,
    pub samples: i32,
    pub planes: i32,
    pub color_primaries: i32,
    pub color_transfer: i32,
    pub color_matrix: i32,
    pub color_range: i32,
    pub chroma_location: i32,
    pub sar_num: i32,
    pub sar_den: i32,
    pub alpha_mode: i32,
    pub key_frame: i32,
    pub interlaced: i32,
    pub top_field_first: i32,
    pub format: [c_char; 64],
    pub channel_layout: [c_char; 128],
}
#[repr(C)]
pub struct Plane {
    pub row_bytes: u64,
    pub rows: u64,
}
include!(concat!(env!("OUT_DIR"), "/abi_layout.rs"));
unsafe extern "C" {
    pub fn mo_media_abi_version() -> u32;
    pub fn mo_media_upstream_version() -> *const c_char;
    pub fn mo_media_open(
        io: *const Io,
        limits: *const Limits,
        out: *mut *mut Media,
        error: *mut RawError,
    ) -> i32;
    pub fn mo_media_close(media: *mut Media);
    pub fn mo_media_stream_count(media: *const Media) -> u32;
    pub fn mo_media_stream(
        media: *const Media,
        index: u32,
        out: *mut Stream,
        error: *mut RawError,
    ) -> i32;
    pub fn mo_media_select(media: *mut Media, index: u32, error: *mut RawError) -> i32;
    pub fn mo_media_next(media: *mut Media, out: *mut Frame, error: *mut RawError) -> i32;
    pub fn mo_media_seek(media: *mut Media, timestamp: i64, error: *mut RawError) -> i32;
    pub fn mo_media_plane(
        media: *const Media,
        index: u32,
        out: *mut Plane,
        error: *mut RawError,
    ) -> i32;
    pub fn mo_media_copy_plane(
        media: *const Media,
        index: u32,
        out: *mut u8,
        capacity: u64,
        stride: u64,
        error: *mut RawError,
    ) -> i32;
}
