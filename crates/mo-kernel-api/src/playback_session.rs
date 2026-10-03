//! One bounded, explicitly owned page sampler. No clock, event queue or global handles.
mod prepared;
mod resize;
pub use crate::playback_owner::PlaybackCompletionFailure;
use crate::{PlaybackBinding, PlaybackCompiledFrame, PlaybackFailure, PlaybackRasterInfo};
use mo_common::{Digest, RationalTime, SlideId, from_json_str};
use mo_presentation_compile::playback::{GenerationError, PlaybackPagePlan};
use mo_presentation_compile::{PagePaintDefaults, PagePlacementRequest, PageRenderRequest};
use mo_presentation_edit::SnapshotRecord;
use mo_raster::{RasterBackend, RasterViewport};
use mo_timeline::{EventHistory, PlaybackGeneration, TimelineLimits, TimelineSamplerInfo};
pub use prepared::PreparedPlaybackRender;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

pub const PLAYBACK_SESSION_PROFILE: &str = "author-page-sampler-session-v1-draft";

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PlaybackPrepareRequest {
    pub snapshot: SnapshotRecord,
    pub slide: SlideId,
    pub binding: PlaybackBinding,
    pub viewport: RasterViewport,
    pub defaults: PagePaintDefaults,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PlaybackSampleRequest {
    pub binding: PlaybackBinding,
    pub at: RationalTime,
    pub history: Option<EventHistory>,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "operation", rename_all = "camelCase", deny_unknown_fields)]
pub enum PlaybackSessionRequest {
    Prepare {
        request: Box<PlaybackPrepareRequest>,
    },
    Inspect {
        binding: PlaybackBinding,
    },
    InspectTiming {
        binding: PlaybackBinding,
    },
    Compile {
        sample: PlaybackSampleRequest,
    },
    Render {
        sample: PlaybackSampleRequest,
    },
    Resize {
        binding: PlaybackBinding,
        #[serde(rename = "expectedViewportRevision")]
        expected_viewport_revision: u32,
        viewport: RasterViewport,
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
pub struct PlaybackSessionInfo {
    pub profile: String,
    /// Content identity for this implementation profile, not an authority token
    /// or a portable serialized-plan compatibility promise.
    pub plan_id: Digest,
    pub viewport: RasterViewport,
    pub viewport_revision: u32,
    pub document_sha256: Digest,
    pub slide: SlideId,
    pub binding: PlaybackBinding,
}
/// Read-only diagnostics. Counts successful timing evaluations, including those
/// followed by a page/raster failure; does not count published or displayed frames.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PlaybackTimingInfo {
    pub binding: PlaybackBinding,
    pub sampler: TimelineSamplerInfo,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum PlaybackSessionFailureCode {
    InputInvalid,
    LimitExceeded,
    Cancelled,
    NotPrepared,
    AlreadyPrepared,
    Disposed,
    BindingConflict,
    ViewportConflict,
    GenerationNotIncreasing,
    RasterRequired,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "camelCase", deny_unknown_fields)]
pub enum PlaybackSessionFailure {
    Session {
        code: PlaybackSessionFailureCode,
        message: String,
    },
    Computation {
        error: PlaybackFailure,
    },
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "status", rename_all = "camelCase", deny_unknown_fields)]
pub enum PlaybackSessionResponse {
    Prepared { info: PlaybackSessionInfo },
    Inspected { info: PlaybackSessionInfo },
    TimingInspected { info: Box<PlaybackTimingInfo> },
    Resized { info: PlaybackSessionInfo },
    Advanced { info: PlaybackSessionInfo },
    Disposed { binding: PlaybackBinding },
    Compiled { frame: Box<PlaybackCompiledFrame> },
    Rendered { info: Box<PlaybackRasterInfo> },
    Error { error: PlaybackSessionFailure },
}
fn fail(code: PlaybackSessionFailureCode, message: impl ToString) -> PlaybackSessionFailure {
    PlaybackSessionFailure::Session {
        code,
        message: message.to_string(),
    }
}
fn computed(error: mo_presentation_compile::playback::PlaybackError) -> PlaybackSessionFailure {
    PlaybackSessionFailure::Computation {
        error: crate::playback::failure(error),
    }
}
fn cancel(check: &dyn Fn() -> bool) -> Result<(), PlaybackSessionFailure> {
    if check() {
        Err(fail(
            PlaybackSessionFailureCode::Cancelled,
            "playback session cancelled",
        ))
    } else {
        Ok(())
    }
}
struct Ready {
    plan: PlaybackPagePlan,
    id: Digest,
    slide: SlideId,
    viewport_revision: u32,
}
impl Ready {
    fn info(&self) -> PlaybackSessionInfo {
        PlaybackSessionInfo {
            profile: PLAYBACK_SESSION_PROFILE.into(),
            plan_id: self.id.clone(),
            document_sha256: self.plan.document_sha256().clone(),
            viewport: self.plan.viewport().clone(),
            viewport_revision: self.viewport_revision,
            slide: self.slide.clone(),
            binding: self.plan.binding().clone(),
        }
    }
}
impl crate::playback_owner::Bound for Ready {
    fn binding(&self) -> &PlaybackBinding {
        self.plan.binding()
    }
}
type State = crate::playback_owner::Owner<Ready>;
impl From<crate::playback_owner::Failure> for PlaybackSessionFailure {
    fn from(e: crate::playback_owner::Failure) -> Self {
        fail(e.code, e.message)
    }
}
/// Owns one validated plan and a bounded consumed-prefix interval cache. Never
/// retains sampled frames, raw histories or future events.
/// Drop releases it. A disposed owner cannot be prepared again (no handle ABA).
#[derive(Default)]
pub struct PlaybackSession {
    identity: Arc<()>,

    state: State,
}
impl PlaybackSession {
    fn ready(&mut self, binding: &PlaybackBinding) -> Result<&mut Ready, PlaybackSessionFailure> {
        self.state.ready(binding).map_err(Into::into)
    }

    fn execute(
        &mut self,
        request: PlaybackSessionRequest,
        backend: Option<&mut dyn RasterBackend>,
        check: &dyn Fn() -> bool,
    ) -> Result<(PlaybackSessionResponse, Vec<u8>), PlaybackSessionFailure> {
        use PlaybackSessionFailureCode as Code;
        use PlaybackSessionRequest as Q;
        use PlaybackSessionResponse as R;
        cancel(check)?;
        let response = match request {
            Q::Prepare { request } => {
                self.state.vacant()?;
                let q = *request;
                let snapshot = crate::timeline::restore_snapshot(q.snapshot, &q.binding, check)
                    .map_err(|e| PlaybackSessionFailure::Computation {
                        error: crate::playback::time(e),
                    })?;
                let id = mo_common::digest(
                    PLAYBACK_SESSION_PROFILE,
                    &(
                        &q.binding.revision,
                        snapshot.semantic_digest(),
                        &q.slide,
                        &q.viewport,
                        &q.defaults,
                        crate::PLAYBACK_PAGE_PROFILE,
                    ),
                )
                .map_err(|e| fail(Code::InputInvalid, e))?;
                cancel(check)?;
                let plan = PlaybackPagePlan::new(
                    PageRenderRequest {
                        page: PagePlacementRequest {
                            document: snapshot.into_record().document,
                            slide: q.slide.clone(),
                        },
                        viewport: q.viewport,
                        defaults: q.defaults,
                    },
                    q.binding,
                    TimelineLimits::default(),
                    check,
                )
                .map_err(computed)?;
                let ready = Box::new(Ready {
                    plan,
                    id,
                    slide: q.slide,
                    viewport_revision: 0,
                });
                let info = ready.info();
                cancel(check)?;
                self.state = State::Ready(ready);
                R::Prepared { info }
            }
            Q::Inspect { binding } => R::Inspected {
                info: self.ready(&binding)?.info(),
            },
            Q::InspectTiming { binding } => R::TimingInspected {
                info: Box::new(PlaybackTimingInfo {
                    sampler: self.ready(&binding)?.plan.timing_info(),
                    binding,
                }),
            },
            Q::Compile { sample } => {
                let r = self.ready(&sample.binding)?;
                let frame = r
                    .plan
                    .compile_frame(sample.at, sample.history.as_ref(), check)
                    .map_err(computed)?;
                R::Compiled {
                    frame: Box::new(frame),
                }
            }
            Q::Render { sample } => {
                let r = self.ready(&sample.binding)?;
                let backend = backend.ok_or_else(|| {
                    fail(Code::RasterRequired, "render requires a raster component")
                })?;
                let image = r
                    .plan
                    .render_frame(sample.at, sample.history.as_ref(), backend, check)
                    .map_err(computed)?;
                return Ok((
                    R::Rendered {
                        info: Box::new(image.info),
                    },
                    image.pixels,
                ));
            }
            Q::Resize {
                binding,
                expected_viewport_revision,
                viewport,
            } => R::Resized {
                info: self.resize(&binding, expected_viewport_revision, viewport, check)?,
            },
            Q::Advance {
                binding,
                generation,
            } => {
                let r = self.ready(&binding)?;
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
        request: PlaybackSessionRequest,
        backend: Option<&mut dyn RasterBackend>,
        check: &dyn Fn() -> bool,
    ) -> (PlaybackSessionResponse, Vec<u8>) {
        self.execute(request, backend, check)
            .unwrap_or_else(|error| (PlaybackSessionResponse::Error { error }, vec![]))
    }
    pub fn dispatch_json(
        &mut self,
        input: &str,
        backend: Option<&mut dyn RasterBackend>,
        check: &dyn Fn() -> bool,
    ) -> (String, Vec<u8>) {
        let request = if input.len() > crate::MAX_REQUEST_BYTES {
            Err(fail(
                PlaybackSessionFailureCode::LimitExceeded,
                "playback session request bytes",
            ))
        } else {
            from_json_str(input).map_err(|e| fail(PlaybackSessionFailureCode::InputInvalid, e))
        };
        let (response, pixels) = match request {
            Ok(q) => self.dispatch(q, backend, check),
            Err(error) => (PlaybackSessionResponse::Error { error }, vec![]),
        };
        (
            serde_json::to_string(&response).expect("typed session response"),
            pixels,
        )
    }
}
