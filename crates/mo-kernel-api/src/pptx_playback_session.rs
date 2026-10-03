//! Explicit owner for imported source timing and immutable local resources.
mod prepared;
mod resize;
use crate::playback_owner::{Bound, Owner};
use crate::*;
use mo_common::{Digest, from_json_str};
use mo_presentation_compile::{
    playback::GenerationError,
    source_playback::{RetainedSourcePlaybackPlan, SourcePlaybackPlan},
    source_resource_page::{ResourcePageOptions, ResourcePreparationInfo, TextPageContext},
};
use mo_raster::RasterBackend;
use mo_timeline::{PlaybackGeneration, TimelineLimits};
pub use prepared::PreparedPptxPlaybackRender;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
pub const PPTX_PLAYBACK_SESSION_PROFILE: &str = "source-resource-sampler-session-v1-draft";
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PptxPlaybackPrepareRequest {
    pub page: PptxResourcePageRequest,
    pub binding: PlaybackBinding,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "operation", rename_all = "camelCase", deny_unknown_fields)]
pub enum PptxPlaybackSessionRequest {
    Prepare {
        request: Box<PptxPlaybackPrepareRequest>,
    },
    Inspect {
        binding: PlaybackBinding,
    },
    InspectTiming {
        binding: PlaybackBinding,
    },
    Render {
        sample: PlaybackSampleRequest,
    },
    Resize {
        binding: PlaybackBinding,
        #[serde(rename = "expectedViewportRevision")]
        expected_viewport_revision: u32,
        viewport: mo_raster::RasterViewport,
    },
    ResizeToFit {
        binding: PlaybackBinding,
        #[serde(rename = "expectedViewportRevision")]
        expected_viewport_revision: u32,
        width: u32,
        height: u32,
    },
    Advance {
        binding: PlaybackBinding,
        generation: PlaybackGeneration,
    },
    Dispose {
        binding: PlaybackBinding,
    },
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PptxPlaybackSessionInfo {
    pub profile: String,
    /// In-process implementation/content identity, not host authorization.
    pub plan_id: Digest,
    pub viewport: mo_raster::RasterViewport,
    pub viewport_revision: u32,
    pub source_sha256: Digest,
    pub slide: String,
    pub binding: PlaybackBinding,
    pub preparation: ResourcePreparationInfo,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "camelCase", deny_unknown_fields)]
pub enum PptxPlaybackSessionFailure {
    Session {
        code: PlaybackSessionFailureCode,
        message: String,
    },
    Computation {
        error: PptxPlaybackFailure,
    },
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "status", rename_all = "camelCase", deny_unknown_fields)]
pub enum PptxPlaybackSessionResponse {
    Prepared { info: Box<PptxPlaybackSessionInfo> },
    Inspected { info: Box<PptxPlaybackSessionInfo> },
    TimingInspected { info: Box<PlaybackTimingInfo> },
    Resized { info: Box<PptxPlaybackSessionInfo> },
    Advanced { info: Box<PptxPlaybackSessionInfo> },
    Disposed { binding: PlaybackBinding },
    Rendered { info: Box<PptxPlaybackRasterInfo> },
    Error { error: PptxPlaybackSessionFailure },
}
pub struct PptxPlaybackResources<'a> {
    pub decoder: &'a mut dyn mo_image::ImageDecoder,
    pub text: Option<&'a mut dyn mo_text::backend::TextBackend>,
}
fn fail(code: PlaybackSessionFailureCode, message: impl ToString) -> PptxPlaybackSessionFailure {
    PptxPlaybackSessionFailure::Session {
        code,
        message: message.to_string(),
    }
}
fn computed(error: PptxPlaybackFailure) -> PptxPlaybackSessionFailure {
    PptxPlaybackSessionFailure::Computation { error }
}
fn cancel(check: &dyn Fn() -> bool) -> Result<(), PptxPlaybackSessionFailure> {
    if check() {
        Err(fail(
            PlaybackSessionFailureCode::Cancelled,
            "playback session cancelled",
        ))
    } else {
        Ok(())
    }
}
impl From<crate::playback_owner::Failure> for PptxPlaybackSessionFailure {
    fn from(e: crate::playback_owner::Failure) -> Self {
        fail(e.code, e.message)
    }
}
struct Ready {
    plan: RetainedSourcePlaybackPlan,
    info: PptxPlaybackSessionInfo,
    request: PptxResourcePageRequest,
}
impl Bound for Ready {
    fn binding(&self) -> &PlaybackBinding {
        self.plan.binding()
    }
}
impl Ready {
    fn info(&self) -> Box<PptxPlaybackSessionInfo> {
        let mut info = self.info.clone();
        info.binding = self.binding().clone();
        Box::new(info)
    }
}
#[derive(Default)]
pub struct PptxPlaybackSession {
    identity: Arc<()>,

    state: Owner<Ready>,
}
impl PptxPlaybackSession {
    fn execute(
        &mut self,
        request: PptxPlaybackSessionRequest,
        source: &[u8],
        fonts: &[u8],
        resources: Option<PptxPlaybackResources<'_>>,
        raster: Option<&mut dyn RasterBackend>,
        check: &dyn Fn() -> bool,
    ) -> Result<(PptxPlaybackSessionResponse, Vec<u8>), PptxPlaybackSessionFailure> {
        use PlaybackSessionFailureCode as Code;
        use PptxPlaybackSessionRequest as Q;
        use PptxPlaybackSessionResponse as R;
        cancel(check)?;
        let accepts_source = matches!(
            request,
            Q::Prepare { .. } | Q::Resize { .. } | Q::ResizeToFit { .. }
        );
        let accepts_fonts = matches!(request, Q::Prepare { .. });
        if (!accepts_source && !source.is_empty()) || (!accepts_fonts && !fonts.is_empty()) {
            return Err(fail(
                Code::InputInvalid,
                "source bytes require prepare or resize; font bytes require prepare",
            ));
        }
        let response = match request {
            Q::Prepare { request } => {
                self.state.vacant()?;
                let q = *request;
                let resources = resources.ok_or_else(|| {
                    fail(Code::InputInvalid, "prepare requires resource components")
                })?;
                let input = crate::pptx_resource_page::prepare_input(&q.page, source, fonts, check)
                    .map_err(|e| computed(crate::pptx_playback::resource(e)))?;
                let plan = SourcePlaybackPlan::new(
                    &input.package,
                    &input.index,
                    q.page.page.clone(),
                    q.binding.clone(),
                    crate::pptx_source::inline_limits(),
                    TimelineLimits::default(),
                    check,
                )
                .map_err(|e| computed(crate::pptx_playback::failure(e)))?;
                let text = match input.manifest.as_ref() {
                    Some(manifest) => Some(TextPageContext {
                        manifest,
                        backend: resources.text.ok_or_else(|| {
                            computed(crate::pptx_playback::resource(
                                crate::pptx_resource_page::request_failure(
                                    PptxPageFailureCode::ResourceRequired,
                                    "text component required".into(),
                                ),
                            ))
                        })?,
                    }),
                    None => None,
                };
                let plan = plan
                    .retain(
                        &input.package,
                        input.index,
                        resources.decoder,
                        text,
                        ResourcePageOptions {
                            selection: q.page.image_source,
                            sampling: q.page.sampling,
                            text_limits: Default::default(),
                        },
                        check,
                    )
                    .map_err(|e| computed(crate::pptx_playback::failure(e.into())))?;
                // Manifest identifies every used font range, including its content
                // digest and face/instance selection. Unreferenced bundle padding
                // is deliberately not part of rendering identity.
                let id = mo_common::digest(
                    PPTX_PLAYBACK_SESSION_PROFILE,
                    &(
                        &q.page,
                        &q.binding.revision,
                        plan.preparation(),
                        PPTX_PLAYBACK_PROFILE,
                    ),
                )
                .map_err(|e| fail(Code::InputInvalid, e))?;
                let info = PptxPlaybackSessionInfo {
                    profile: PPTX_PLAYBACK_SESSION_PROFILE.into(),
                    plan_id: id,
                    source_sha256: q.page.page.expected_source_sha256.clone(),
                    viewport: q.page.page.viewport.clone(),
                    viewport_revision: 0,
                    slide: q.page.page.slide.clone(),
                    binding: q.binding,
                    preparation: plan.preparation().clone(),
                };
                let ready = Box::new(Ready {
                    plan,
                    info,
                    request: q.page,
                });
                let info = ready.info();
                cancel(check)?;
                self.state = Owner::Ready(ready);
                R::Prepared { info }
            }
            Q::Inspect { binding } => R::Inspected {
                info: self.state.ready(&binding)?.info(),
            },
            Q::InspectTiming { binding } => R::TimingInspected {
                info: Box::new(PlaybackTimingInfo {
                    sampler: self.state.ready(&binding)?.plan.timing_info(),
                    binding,
                }),
            },
            Q::Render { sample } => {
                let r = self.state.ready(&sample.binding)?;
                let raster = raster.ok_or_else(|| {
                    fail(Code::RasterRequired, "render requires a raster component")
                })?;
                let (playback, page) = r
                    .plan
                    .render(sample.at, sample.history.as_ref(), raster, check)
                    .map_err(|e| computed(crate::pptx_playback::failure(e)))?;
                return Ok((
                    R::Rendered {
                        info: Box::new(PptxPlaybackRasterInfo {
                            profile: playback.profile().into(),
                            playback,
                            page: page.info,
                        }),
                    },
                    page.pixels,
                ));
            }
            Q::Resize {
                binding,
                expected_viewport_revision,
                viewport,
            } => {
                let resources = resources
                    .ok_or_else(|| fail(Code::InputInvalid, "resize requires an image decoder"))?;
                R::Resized {
                    info: self.resize(
                        &binding,
                        expected_viewport_revision,
                        viewport,
                        source,
                        resources.decoder,
                        check,
                    )?,
                }
            }
            Q::ResizeToFit {
                binding,
                expected_viewport_revision,
                width,
                height,
            } => {
                let viewport = self
                    .state
                    .ready(&binding)?
                    .plan
                    .fit_viewport(width, height)
                    .map_err(|e| computed(crate::pptx_playback::failure(e)))?;
                let resources = resources
                    .ok_or_else(|| fail(Code::InputInvalid, "resize requires an image decoder"))?;
                R::Resized {
                    info: self.resize(
                        &binding,
                        expected_viewport_revision,
                        viewport,
                        source,
                        resources.decoder,
                        check,
                    )?,
                }
            }
            Q::Advance {
                binding,
                generation,
            } => {
                let r = self.state.ready(&binding)?;
                cancel(check)?;
                r.plan
                    .advance_generation(&binding, generation)
                    .map_err(|e| {
                        fail(
                            match e {
                                GenerationError::BindingConflict => Code::BindingConflict,
                                GenerationError::NotIncreasing => Code::GenerationNotIncreasing,
                            },
                            e,
                        )
                    })?;
                R::Advanced { info: r.info() }
            }
            Q::Dispose { binding } => {
                self.state.dispose(binding.clone(), check)?;
                R::Disposed { binding }
            }
        };
        Ok((response, vec![]))
    }
    pub fn dispatch(
        &mut self,
        request: PptxPlaybackSessionRequest,
        source: &[u8],
        fonts: &[u8],
        resources: Option<PptxPlaybackResources<'_>>,
        raster: Option<&mut dyn RasterBackend>,
        check: &dyn Fn() -> bool,
    ) -> (PptxPlaybackSessionResponse, Vec<u8>) {
        self.execute(request, source, fonts, resources, raster, check)
            .unwrap_or_else(|error| (PptxPlaybackSessionResponse::Error { error }, vec![]))
    }
    pub fn dispatch_json(
        &mut self,
        input: &str,
        source: &[u8],
        fonts: &[u8],
        resources: Option<PptxPlaybackResources<'_>>,
        raster: Option<&mut dyn RasterBackend>,
        check: &dyn Fn() -> bool,
    ) -> (String, Vec<u8>) {
        let request = if input.len() > MAX_REQUEST_BYTES {
            Err(fail(
                PlaybackSessionFailureCode::LimitExceeded,
                "source session request bytes",
            ))
        } else {
            from_json_str(input).map_err(|e| fail(PlaybackSessionFailureCode::InputInvalid, e))
        };
        let (response, pixels) = match request {
            Ok(q) => self.dispatch(q, source, fonts, resources, raster, check),
            Err(error) => (PptxPlaybackSessionResponse::Error { error }, vec![]),
        };
        (
            serde_json::to_string(&response).expect("typed source session response"),
            pixels,
        )
    }
}
