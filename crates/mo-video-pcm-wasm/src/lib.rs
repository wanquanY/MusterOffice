//! Synchronous computation in an explicitly owned Worker. The host supplies
//! WebCodecs output and tile capabilities, owns deadlines and atomic storage.
mod decode;
mod movie;
pub use decode::PcmDecode;
use mo_common::{Digest, ResourceId, digest};
use mo_time::ExactRatio;
use mo_video_model::StreamId;
use mo_video_pcm::{PcmError, PcmStreamLimits, PcmStreamOrigin, QUANTIZED_PCM_PROFILE};
pub use movie::MoviePcm;
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use wasm_bindgen::prelude::*;

type Result<T> = std::result::Result<T, JsValue>;
pub const PROFILE: &str = "musteroffice.pcm-worker/1-draft";
#[wasm_bindgen]
pub fn pcm_profile() -> String {
    PROFILE.into()
}
fn invalid() -> JsValue {
    failure(PcmError::Invalid("PCM Worker request"))
}
fn failure(error: PcmError) -> JsValue {
    let code = match &error {
        PcmError::Budget | PcmError::Audio(mo_video_audio::AudioError::Budget) => "resource_limit",
        PcmError::Cancelled | PcmError::Audio(mo_video_audio::AudioError::Cancelled) => "cancelled",
        PcmError::Missing { .. }
        | PcmError::Audio(mo_video_audio::AudioError::PendingSource { .. }) => "pending_source",
        _ => "invalid_request",
    };
    JsValue::from_str(&serde_json::json!({"code":code,"detail":error.to_string()}).to_string())
}
fn parse<T: DeserializeOwned>(text: &str, limit: usize) -> Result<T> {
    if text.len() > limit {
        return Err(failure(PcmError::Budget));
    }
    mo_common::from_json_str_with_path(text).map_err(|_| invalid())
}
fn json<T: Serialize>(value: &T) -> Result<String> {
    serde_json::to_string(value).map_err(|_| invalid())
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Options {
    resource: ResourceId,
    content: Digest,
    stream: StreamId,
    decoder_profile: Digest,
    track_id: u32,
    raw_origin: ExactRatio,
    sample_rate: u32,
    channels: u32,
}
impl Options {
    fn read(value: &str) -> Result<Self> {
        let options: Self = parse(value, 16384)?;
        if options.track_id == 0
            || !(1..=2).contains(&options.channels)
            || !(8000..=192000).contains(&options.sample_rate)
        {
            return Err(invalid());
        }
        options
            .raw_origin
            .check_budget(256)
            .map_err(|_| invalid())?;
        Ok(options)
    }
    fn origin(&self) -> Result<PcmStreamOrigin> {
        Ok(PcmStreamOrigin {
            resource: self.resource.clone(),
            content: self.content.clone(),
            stream: self.stream.clone(),
            decoder_profile: digest(
                QUANTIZED_PCM_PROFILE,
                &(
                    &self.decoder_profile,
                    self.track_id,
                    &self.raw_origin,
                    self.sample_rate,
                    self.channels,
                ),
            )
            .map_err(|_| invalid())?,
        })
    }
}
fn limits() -> PcmStreamLimits {
    let mut limits = PcmStreamLimits {
        max_working_bytes: 4 << 20,
        max_tiles_per_step: 3,
        ..Default::default()
    };
    limits.inspection.frame.max_frames = 8192;
    limits.inspection.frame.max_bytes = 8192 * 2 * 4;
    limits
}
