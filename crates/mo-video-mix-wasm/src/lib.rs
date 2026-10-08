//! Project audio computation in a host-owned Worker. No codec, network, storage,
//! device clock or product authorization. Shared Rust semantics and DSP only.
mod resources;
use js_sys::{Function, Uint8Array};
use mo_common::{Digest, Ticks};
use mo_video_audio::{AudioError, AudioMixer, ChannelLayout, MixLimits, MixStats, PcmLimits};
use mo_video_compile::{CompileLimits, CompiledDocument};
use mo_video_model::VideoDocument;
use resources::Resources;
use std::sync::Arc;
use wasm_bindgen::prelude::*;

pub const PROFILE: &str = "musteroffice.project-audio-worker/1-draft";
type Result<T> = std::result::Result<T, JsValue>;
#[wasm_bindgen]
pub fn mix_profile() -> String {
    PROFILE.into()
}

fn failure(error: AudioError, pending: Option<&resources::Pending>) -> JsValue {
    let code = match &error {
        AudioError::PendingSource { .. } => "pending_source",
        AudioError::PendingDerived { .. } => "pending_derived",
        AudioError::Unsupported(_) => "unsupported_capability",
        AudioError::Budget => "resource_limit",
        AudioError::Cancelled => "cancelled",
        _ => "invalid_request",
    };
    JsValue::from_str(
        &serde_json::json!({
            "code": code, "detail": error.to_string(), "source": pending,
        })
        .to_string(),
    )
}
fn invalid(detail: &'static str) -> JsValue {
    failure(AudioError::Invalid(detail), None)
}
fn json(value: &impl serde::Serialize) -> Result<String> {
    serde_json::to_string(value).map_err(|_| invalid("audio result"))
}

#[wasm_bindgen]
pub struct ProjectAudio {
    document: Arc<CompiledDocument>,
    mixer: AudioMixer<'static>,
    audio_channels: u32,
    resources: Resources,
    stats: Option<MixStats>,
}
#[wasm_bindgen]
impl ProjectAudio {
    /// Callbacks are synchronous capabilities. `describe(bindingJson)` returns
    /// a shared AudioResourceDescription JSON; `read(windowJson)` returns exact
    /// interleaved f32le. Null indicates an unprepared source. Promises are invalid.
    pub fn open(
        document: &str,
        expected_digest: &str,
        describe: Function,
        read: Function,
    ) -> Result<Self> {
        Self::open_with_channels(document, expected_digest, 2, describe, read)
    }
    /// Explicit output layout for preview/export. Channel routing remains in
    /// the shared mixer; the host must never downmix the returned PCM in JS.
    pub fn open_with_channels(
        document: &str,
        expected_digest: &str,
        channels: u32,
        describe: Function,
        read: Function,
    ) -> Result<Self> {
        let layout = match channels {
            1 => ChannelLayout::mono(),
            2 => ChannelLayout::stereo(),
            _ => return Err(invalid("output channel layout")),
        };
        if document.len() > 32 << 20 {
            return Err(failure(AudioError::Budget, None));
        }
        let expected =
            Digest::try_from(expected_digest.to_owned()).map_err(|_| invalid("document digest"))?;
        let document: VideoDocument =
            mo_common::from_json_str_with_path(document).map_err(|_| invalid("video document"))?;
        let document = Arc::new(
            CompiledDocument::new(document, CompileLimits::default(), &|| false)
                .map_err(|e| failure(e.into(), None))?,
        );
        if document.digest() != &expected {
            return Err(invalid("document digest mismatch"));
        }
        let mixer = AudioMixer::from_shared(
            document.clone(),
            layout,
            MixLimits {
                pcm: PcmLimits {
                    max_frames: 65536,
                    max_bytes: 4 << 20,
                },
                max_active_pcm_bytes: 64 << 20,
                max_kernel_bytes: 32 << 20,
                max_retime_plan_bytes: 16 << 20,
                ..Default::default()
            },
        )
        .map_err(|e| failure(e, None))?;
        Ok(Self {
            document,
            mixer,
            audio_channels: channels,
            resources: Resources::new(describe, read),
            stats: None,
        })
    }
    pub fn info(&self) -> Result<String> {
        let d = self.document.document();
        json(&serde_json::json!({
            "profile": PROFILE, "documentDigest": self.document.digest(),
            "canvasWidth": d.canvas.width, "canvasHeight": d.canvas.height,
            "frameRateNumerator": d.frame_rate.numerator,
            "frameRateDenominator": d.frame_rate.denominator,
            "frameCount": Ticks::new(self.document.frame_count().map_err(|e| failure(e.into(), None))?),
            "audioSampleRate": d.audio_sample_rate, "audioChannels": self.audio_channels,
            "audioSampleCount": Ticks::new(self.document.audio_sample_count().map_err(|e| failure(e.into(), None))?),
        }))
    }
    /// Complete project window, never a partially mixed success. Random access
    /// does not mutate playhead or clock. Worker termination owns cancellation.
    pub fn read(&mut self, first: i64, frames: u32) -> Result<Uint8Array> {
        self.resources.pending = None;
        self.stats = None;
        if frames == 0 || frames > 4096 {
            return Err(invalid("audio output window"));
        }
        let mixed = self
            .mixer
            .render(&mut self.resources, first, frames, &|| false)
            .map_err(|e| failure(e, self.resources.pending.as_ref()))?;
        self.stats = Some(mixed.stats);
        let bytes: Vec<u8> = mixed
            .pcm
            .samples()
            .iter()
            .flat_map(|v| v.to_le_bytes())
            .collect();
        Ok(Uint8Array::from(bytes.as_slice()))
    }
    /// Last successful query's work and core PCM/cache charges; not total WASM
    /// or opaque browser memory. Failed queries clear this receipt.
    pub fn stats(&self) -> Result<String> {
        json(&self.stats)
    }
    pub fn clear(&mut self) {
        self.mixer.clear();
        self.stats = None;
        self.resources.pending = None;
    }
}
