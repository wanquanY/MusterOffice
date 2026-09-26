//! Explicit owner for imported source timing and immutable local resources.
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
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
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
        if !matches!(request, Q::Prepare { .. }) && (!source.is_empty() || !fonts.is_empty()) {
            return Err(fail(
                Code::InputInvalid,
                "source and font bytes are only admitted during prepare",
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
                    source_sha256: q.page.page.expected_source_sha256,
                    slide: q.page.page.slide,
                    binding: q.binding,
                    preparation: plan.preparation().clone(),
                };
                let ready = Box::new(Ready { plan, info });
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
                            profile: PPTX_PLAYBACK_PROFILE.into(),
                            playback,
                            page: page.info,
                        }),
                    },
                    page.pixels,
                ));
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
