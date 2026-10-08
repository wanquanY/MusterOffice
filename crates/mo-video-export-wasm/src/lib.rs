//! Exact export planning in a host-owned Worker. No codec, mux, file, task or
//! permission authority. Native and browser drivers use the same ExportPlan.
mod options;
use mo_common::{Digest, Ticks};
use mo_video_compile::{CompileError, CompileLimits, CompiledDocument};
use mo_video_export::{
    ExportLimits, ExportPlan, ExportSchedule, ExportWork, PlanError, ScheduleError,
};
use mo_video_model::VideoDocument;
use options::Options;
use wasm_bindgen::prelude::*;

pub const PROFILE: &str = "musteroffice.project-export-worker/1-draft";
type Result<T> = std::result::Result<T, JsValue>;
fn failure(code: &str, detail: impl std::fmt::Display) -> JsValue {
    JsValue::from_str(&serde_json::json!({"code":code,"detail":detail.to_string()}).to_string())
}
fn invalid(detail: &'static str) -> JsValue {
    failure("invalid_request", detail)
}
fn json(value: &impl serde::Serialize) -> Result<String> {
    serde_json::to_string(value).map_err(|_| invalid("export result"))
}
fn schedule_error(error: ScheduleError) -> JsValue {
    failure(
        match error {
            ScheduleError::Transport => "unsupported_capability",
            _ => "invalid_request",
        },
        error,
    )
}
fn compile_error(error: CompileError) -> JsValue {
    failure(
        match error {
            CompileError::Budget => "resource_limit",
            CompileError::Cancelled => "cancelled",
            _ => "invalid_request",
        },
        error,
    )
}
fn plan_error(error: PlanError) -> JsValue {
    failure(
        match error {
            PlanError::Budget => "resource_limit",
            PlanError::Cancelled => "cancelled",
            _ => "invalid_request",
        },
        error,
    )
}
#[wasm_bindgen]
pub fn export_profile() -> String {
    PROFILE.into()
}
#[wasm_bindgen]
pub struct ProjectExport {
    schedule: ExportSchedule,
    info: String,
}
#[wasm_bindgen]
impl ProjectExport {
    /// The digest must name the same immutable document used by both visual
    /// and audio Workers. Host owns that binding and resource closure.
    pub fn open(document: &str, expected_digest: &str, options: &str) -> Result<Self> {
        if document.len() > 32 << 20 || options.len() > 4096 {
            return Err(failure("resource_limit", "export input bytes"));
        }
        let expected =
            Digest::try_from(expected_digest.to_owned()).map_err(|_| invalid("document digest"))?;
        let document: VideoDocument =
            mo_common::from_json_str_with_path(document).map_err(|_| invalid("video document"))?;
        let document = CompiledDocument::new(document, CompileLimits::default(), &|| false)
            .map_err(compile_error)?;
        if document.digest() != &expected {
            return Err(invalid("document digest mismatch"));
        }
        let options: Options =
            serde_json::from_str(options).map_err(|_| invalid("export options"))?;
        let plan = ExportPlan::new(
            &document,
            options.core()?,
            ExportLimits {
                audio_block_frames: options.audio_block_frames,
                ..ExportLimits::default()
            },
            &|| false,
        )
        .map_err(plan_error)?;
        let c = plan.configuration();
        let schedule = plan.schedule();
        // All full frames have a period >= 1/240s. Validate the finite last
        // interval as well, before the host creates a candidate or encoder.
        schedule
            .video(c.video_frames - 1)
            .map_err(schedule_error)?
            .codec_interval()
            .map_err(schedule_error)?;
        if c.audio_samples > 0 {
            mo_video_export::ExportAudioSample {
                first_sample: Ticks::new(c.audio_samples - 1),
                frames: 1,
                sample_rate: c.audio_sample_rate,
            }
            .codec_interval()
            .map_err(schedule_error)?;
        }
        let info = json(&serde_json::json!({
            "profile": PROFILE, "documentDigest": expected,
            "outputProfile": c.profile,
            "colorProfile": mo_video_render::I420_PROFILE,
            "width": c.width, "height": c.height,
            "videoTimeScale": c.video_time_scale,
            "videoDuration": Ticks::new(c.video_duration),
            "videoFrames": c.video_frames,
            "frameRateNumerator": c.frame_rate_num, "frameRateDenominator": c.frame_rate_den,
            "videoBitRate": c.video_bit_rate, "gopFrames": c.gop_frames,
            "audioSampleRate": c.audio_sample_rate, "audioChannels": c.audio_channels,
            "audioBitRate": c.audio_bit_rate, "audioSamples": Ticks::new(c.audio_samples),
            "audioTail": plan.audio_tail(), "options": options,
        }))?;
        Ok(Self { schedule, info })
    }
    pub fn info(&self) -> String {
        self.info.clone()
    }
    /// Codec microseconds are transport only. The `work` interval is the
    /// authoritative media clock and must be retained through mux verification.
    pub fn next(&self) -> Result<String> {
        let work = self.schedule.next();
        let transport = match work {
            ExportWork::Video(v) => Some(v.codec_interval().map_err(schedule_error)?),
            ExportWork::Audio(a) => Some(a.codec_interval().map_err(schedule_error)?),
            ExportWork::Complete => None,
        };
        json(&serde_json::json!({"work":work,"transport":transport}))
    }
    /// A host acknowledges only the original `work`, after codec acceptance.
    /// Waiting, failed and cancelled operations must not acknowledge a step.
    pub fn accept(&mut self, work: &str) -> Result<String> {
        if work.len() > 1024 {
            return Err(failure("resource_limit", "acknowledgement bytes"));
        }
        let work: ExportWork =
            serde_json::from_str(work).map_err(|_| invalid("acknowledgement"))?;
        self.schedule.accept(work).map_err(schedule_error)?;
        self.progress()
    }
    pub fn progress(&self) -> Result<String> {
        let progress = self.schedule.progress();
        json(&serde_json::json!({"videoFrames":progress.video_frames,
            "audioSamples":Ticks::new(progress.audio_samples)}))
    }
}
