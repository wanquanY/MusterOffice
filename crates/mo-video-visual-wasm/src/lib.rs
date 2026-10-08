//! Immutable project visual compilation for a host-owned Worker. No device,
//! codec, pixel transfer, storage, authorization or JS geometry implementation.
use mo_common::{Digest, Ticks};
use mo_video_compile::{CompileError, CompileLimits, CompiledDocument, QueryLimits};
use mo_video_gpu_plan::{PreparedGpuFrame, ProgramLimits, VisualReply};
use mo_video_model::VideoDocument;
use mo_video_render::{FrameRenderOptions, RasterExtent, RenderError, RenderLimits, Sampling};
use wasm_bindgen::prelude::*;

pub const PROFILE: &str = "musteroffice.project-visual-worker/1-draft";
type Result<T> = std::result::Result<T, JsValue>;
fn failure(code: &str, detail: impl std::fmt::Display) -> JsValue {
    JsValue::from_str(&serde_json::json!({"code":code,"detail":detail.to_string()}).to_string())
}
fn invalid(detail: &'static str) -> JsValue {
    failure("invalid_request", detail)
}
fn render_error(e: RenderError) -> JsValue {
    failure(
        match &e {
            RenderError::Budget => "resource_limit",
            RenderError::Cancelled => "cancelled",
            RenderError::Unsupported(_) => "unsupported_capability",
            _ => "invalid_request",
        },
        e,
    )
}
fn compile_error(e: CompileError) -> JsValue {
    failure(
        match &e {
            CompileError::Budget => "resource_limit",
            CompileError::Cancelled => "cancelled",
            _ => "invalid_request",
        },
        e,
    )
}
fn json(v: &impl serde::Serialize) -> Result<String> {
    serde_json::to_string(v).map_err(|_| invalid("visual result"))
}
#[wasm_bindgen]
pub fn visual_profile() -> String {
    PROFILE.into()
}
#[wasm_bindgen]
pub fn composition_shader() -> String {
    mo_video_gpu_plan::SHADER.into()
}
#[wasm_bindgen]
pub struct ProjectVisual {
    document: CompiledDocument,
}
#[wasm_bindgen]
impl ProjectVisual {
    pub fn open(document: &str, expected_digest: &str) -> Result<Self> {
        if document.len() > 32 << 20 {
            return Err(failure("resource_limit", "document bytes"));
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
        Ok(Self { document })
    }
    pub fn info(&self) -> Result<String> {
        let d = self.document.document();
        json(
            &serde_json::json!({"profile":PROFILE,"documentDigest":self.document.digest(),
            "canvasWidth":d.canvas.width,"canvasHeight":d.canvas.height,
            "frameRateNumerator":d.frame_rate.numerator,"frameRateDenominator":d.frame_rate.denominator,
            "frameCount":Ticks::new(self.document.frame_count().map_err(compile_error)?)}),
        )
    }
    /// Host bounds candidate lifetime/concurrency. A candidate owns its plan;
    /// dropping the project or preparing another frame cannot invalidate it.
    pub fn frame(&self, index: i64, width: u32, height: u32) -> Result<VisualCandidate> {
        let plan = self
            .document
            .frame(index, QueryLimits::default(), &|| false)
            .map_err(compile_error)?
            .value;
        let inner = PreparedGpuFrame::new(
            plan,
            FrameRenderOptions {
                raster: RasterExtent { width, height },
                sampling: Sampling::Bilinear,
                limits: RenderLimits::default(),
            },
            ProgramLimits::default(),
            &|| false,
        )
        .map_err(render_error)?;
        Ok(VisualCandidate { inner })
    }
}
#[wasm_bindgen]
pub struct VisualCandidate {
    inner: PreparedGpuFrame,
}
#[wasm_bindgen]
impl VisualCandidate {
    pub fn requests(&self) -> Result<String> {
        json(&self.inner.requests())
    }
    pub fn identity(&self) -> Result<String> {
        let p = self.inner.plan();
        json(
            &serde_json::json!({"documentDigest":p.document_digest,"frameIndex":p.frame_index,"sample":p.sample}),
        )
    }
    /// Metadata only. Reply IDs do not authorize pixels; the host must bind each
    /// actual normalized GPU image to the exact request that produced this reply.
    pub fn compile(&self, replies: &str) -> Result<String> {
        if replies.len() > 1 << 20 {
            return Err(failure("resource_limit", "visual reply bytes"));
        }
        let replies: Vec<VisualReply> =
            mo_common::from_json_str_with_path(replies).map_err(|_| invalid("visual replies"))?;
        json(
            &self
                .inner
                .compile(replies, &|| false)
                .map_err(render_error)?,
        )
    }
}
