//! Typed candidate-output capabilities. No paths, URLs, codec option strings or
//! implicit format fallback. Publication and storage durability belong to hosts.
use serde::{Deserialize, Serialize};
use std::io;

/// Random-access writes into a new, isolated candidate. Short writes are legal;
/// zero writes are errors. A failed encoder leaves bytes to discard, never publish.
pub trait WriteAt {
    fn write_at(&mut self, offset: u64, bytes: &[u8]) -> io::Result<usize>;
}
impl<F: FnMut(u64, &[u8]) -> io::Result<usize>> WriteAt for F {
    fn write_at(&mut self, offset: u64, bytes: &[u8]) -> io::Result<usize> {
        self(offset, bytes)
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum EncodeProfile {
    H264VideoToolboxAacMp4,
    /// Explicit browser capability; native adapters must never alias it to a
    /// platform encoder. Availability is checked against the actual WebCodecs
    /// configuration by the browser adapter before accepting any input.
    H264WebCodecsAacMp4,
    /// Lossless audit profile. FFV1/float PCM MP4 is not a browser playback profile.
    Ffv1PcmMp4,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct EncodeConfig {
    pub profile: EncodeProfile,
    pub width: u32,
    pub height: u32,
    /// Video clock is 1 / video_time_scale; every full frame must fit exactly.
    pub video_time_scale: u32,
    pub frame_rate_num: u32,
    pub frame_rate_den: u32,
    pub video_duration: i64,
    pub video_frames: u32,
    pub video_bit_rate: u32,
    pub audio_sample_rate: u32,
    /// 1 = mono, 2 = ordered FL/FR. Any downmix must have happened explicitly.
    pub audio_channels: u32,
    pub audio_bit_rate: u32,
    pub gop_frames: u32,
    /// Zero means no audio stream; otherwise an exact finite sample count.
    pub audio_samples: i64,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct EncodeLimits {
    pub max_file_bytes: u64,
    pub max_write_bytes: u64,
    pub max_pending_bytes: u64,
    /// Active frame payload; vendor padding/internal allocations are separate.
    pub max_frame_bytes: u64,
    pub max_pixels: u32,
    pub max_packets: u32,
    pub max_packet_bytes: u32,
    pub max_pending_packets: u32,
    pub max_steps: u32,
    pub max_write_calls: u32,
}
impl Default for EncodeLimits {
    fn default() -> Self {
        Self {
            max_file_bytes: 4 << 30,
            max_write_bytes: 8 << 30,
            max_pending_bytes: 32 << 20,
            max_frame_bytes: 256 << 20,
            max_pixels: 8192 * 8192,
            max_packets: 10_000_000,
            max_packet_bytes: 16 << 20,
            max_pending_packets: 256,
            max_steps: 16384,
            max_write_calls: 1_000_000,
        }
    }
}
/// Caller has already converted/composited to BT.709 limited-range I420 with
/// left-cosited chroma, square pixels, opaque output and explicit SDR interpretation.
pub struct EncodeVideoFrame<'a> {
    pub pts: i64,
    pub duration: i64,
    pub planes: [&'a [u8]; 3],
    pub strides: [usize; 3],
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct EncodeStats {
    pub byte_length: u64,
    pub write_bytes: u64,
    pub video_frames: u64,
    pub audio_samples: u64,
    pub video_packets: u64,
    pub audio_packets: u64,
    pub peak_pending_bytes: u64,
}

/// Finite configured candidate encoder. Implementations own their codec state
/// and cancel capability. Drop must abort without publishing or finalizing.
/// Core drivers check cancellation between steps; a worker implementation must
/// additionally interrupt each blocked operation using its host cancel signal.
pub trait VideoEncoder {
    type Error: std::error::Error;
    fn configuration(&self) -> EncodeConfig;
    fn video(&mut self, frame: EncodeVideoFrame<'_>) -> Result<(), Self::Error>;
    fn audio(&mut self, first_sample: i64, samples: &[f32]) -> Result<(), Self::Error>;
    fn finish(&mut self) -> Result<EncodeStats, Self::Error>;
}
