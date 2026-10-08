use crate::{Result, invalid};
use mo_media::EncodeProfile;
use mo_video_export::{ExportAudio, ExportOptions};
use mo_video_render::{Sampling, SdrTransform, ToneMap};
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize)]
#[serde(tag = "kind", rename_all = "camelCase", deny_unknown_fields)]
pub enum Audio {
    Omit,
    Mono {
        #[serde(rename = "bitRate")]
        bit_rate: u32,
    },
    Stereo {
        #[serde(rename = "bitRate")]
        bit_rate: u32,
    },
}
#[derive(Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Options {
    video_bit_rate: u32,
    gop_frames: u32,
    audio: Audio,
    pub audio_block_frames: u32,
    background_srgb: [u8; 3],
    /// Explicit Reinhard luminance v1 policy; null means no HDR conversion.
    source_peak_nits: Option<f64>,
}
impl Options {
    pub fn core(&self) -> Result<ExportOptions> {
        let tone_map = self
            .source_peak_nits
            .map(|source_peak_nits| ToneMap::ReinhardLuminanceV1 { source_peak_nits });
        SdrTransform::new(false, tone_map).map_err(|_| invalid("tone map"))?;
        Ok(ExportOptions {
            profile: EncodeProfile::H264WebCodecsAacMp4,
            video_bit_rate: self.video_bit_rate,
            gop_frames: self.gop_frames,
            audio: match self.audio {
                Audio::Omit => ExportAudio::Omit,
                Audio::Mono { bit_rate } => ExportAudio::Mono { bit_rate },
                Audio::Stereo { bit_rate } => ExportAudio::Stereo { bit_rate },
            },
            background_srgb: self.background_srgb,
            tone_map,
            sampling: Sampling::Bilinear,
        })
    }
}
