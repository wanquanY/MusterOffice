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
#[repr(C)]
pub struct Encode {
    _private: [u8; 0],
}
#[repr(C)]
pub struct EncodeIo {
    pub opaque: *mut c_void,
    pub write_at: unsafe extern "C" fn(*mut c_void, u64, *const u8, i32) -> i32,
    pub cancelled: unsafe extern "C" fn(*mut c_void) -> i32,
}
#[repr(C)]
pub struct EncoderConfig {
    pub profile: i32,
    pub width: i32,
    pub height: i32,
    pub video_time_scale: i32,
    pub frame_rate_num: i32,
    pub frame_rate_den: i32,
    pub video_duration: i64,
    pub video_frames: i64,
    pub video_bit_rate: i64,
    pub audio_sample_rate: i32,
    pub audio_channels: i32,
    pub audio_bit_rate: i32,
    pub gop_frames: i32,
    pub audio_samples: i64,
}
#[repr(C)]
pub struct EncoderLimits {
    pub max_file_bytes: u64,
    pub max_write_bytes: u64,
    pub max_pending_bytes: u64,
    pub max_frame_bytes: u64,
    pub max_pixels: u32,
    pub max_packets: u32,
    pub max_packet_bytes: u32,
    pub max_pending_packets: u32,
    pub max_steps: u32,
    pub max_write_calls: u32,
}
#[repr(C)]
pub struct EncoderVideo {
    pub pts: i64,
    pub duration: i64,
    pub planes: [*const u8; 3],
    pub plane_bytes: [u64; 3],
    pub strides: [u64; 3],
}
#[repr(C)]
pub struct EncoderStats {
    pub byte_length: u64,
    pub write_bytes: u64,
    pub video_frames: u64,
    pub audio_samples: u64,
    pub video_packets: u64,
    pub audio_packets: u64,
    pub peak_pending_bytes: u64,
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

unsafe extern "C" {
    pub fn mo_encode_abi_version() -> u32;
    pub fn mo_encode_profiles() -> u32;
    pub fn mo_encode_open(
        io: *const EncodeIo,
        config: *const EncoderConfig,
        limits: *const EncoderLimits,
        out: *mut *mut Encode,
        error: *mut RawError,
    ) -> i32;
    pub fn mo_encode_video(
        encode: *mut Encode,
        frame: *const EncoderVideo,
        error: *mut RawError,
    ) -> i32;
    pub fn mo_encode_audio(
        encode: *mut Encode,
        first: i64,
        samples: *const f32,
        frames: u32,
        error: *mut RawError,
    ) -> i32;
    pub fn mo_encode_finish(
        encode: *mut Encode,
        stats: *mut EncoderStats,
        error: *mut RawError,
    ) -> i32;
    pub fn mo_encode_close(encode: *mut Encode);
}
