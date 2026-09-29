//! Snapshot-bound sample compilation/rendering. Host owns presentation clock and publication.
use super::{PageFailure, TimelineEvaluateRequest, TimelineFailure, TimelineFailureCode};
use mo_common::from_json_str;
pub use mo_presentation_compile::playback::{
    PLAYBACK_PAGE_PROFILE, PlaybackCompiledFrame, PlaybackRasterInfo, frame_profile,
};
use mo_presentation_compile::{
    PagePaintDefaults, PagePlacementRequest, PageRenderRequest,
    playback::{PlaybackError, PlaybackPagePlan},
};
use mo_raster::{RasterBackend, RasterViewport};
use mo_timeline::TimelineLimits;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PlaybackPageRequest {
    pub playback: TimelineEvaluateRequest,
    pub viewport: RasterViewport,
    pub defaults: PagePaintDefaults,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "camelCase", deny_unknown_fields)]
pub enum PlaybackFailure {
    Timeline { error: TimelineFailure },
    Page { error: PageFailure },
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "status", rename_all = "camelCase", deny_unknown_fields)]
pub enum PlaybackCompileResponse {
    Compiled { frame: Box<PlaybackCompiledFrame> },
    Error { error: PlaybackFailure },
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "status", rename_all = "camelCase", deny_unknown_fields)]
pub enum PlaybackRasterResponse {
    Rendered { info: Box<PlaybackRasterInfo> },
    Error { error: PlaybackFailure },
}
pub(crate) fn time(error: TimelineFailure) -> PlaybackFailure {
    PlaybackFailure::Timeline { error }
}
pub(crate) fn failure(error: PlaybackError) -> PlaybackFailure {
    match error {
        PlaybackError::Time(e) => time(super::timeline::failure(e)),
        PlaybackError::Page(e) => PlaybackFailure::Page {
            error: super::page::failure(e),
        },
    }
}
fn prepare(
    input: &str,
    check: &dyn Fn() -> bool,
) -> Result<
    (
        PlaybackPagePlan,
        mo_common::RationalTime,
        Option<mo_timeline::EventHistory>,
    ),
    PlaybackFailure,
> {
    if input.len() > super::MAX_REQUEST_BYTES {
        return Err(time(TimelineFailure {
            code: TimelineFailureCode::LimitExceeded,
            message: "playback page request bytes".into(),
        }));
    }
    let q: PlaybackPageRequest = from_json_str(input).map_err(|e| {
        time(TimelineFailure {
            code: TimelineFailureCode::InputInvalid,
            message: e.to_string(),
        })
    })?;
    let p = q.playback;
    let snapshot =
        super::timeline::restore_snapshot(p.snapshot, &p.binding, check).map_err(time)?;
    let request = PageRenderRequest {
        page: PagePlacementRequest {
            document: snapshot.into_record().document,
            slide: p.slide,
        },
        viewport: q.viewport,
        defaults: q.defaults,
    };
    let plan = PlaybackPagePlan::new(request, p.binding, TimelineLimits::default(), check)
        .map_err(failure)?;
    Ok((plan, p.at, p.history))
}
pub fn compile_playback_page_json(input: &str, check: &dyn Fn() -> bool) -> String {
    let result = prepare(input, check).and_then(|(mut plan, at, history)| {
        plan.compile_frame(at, history.as_ref(), check)
            .map_err(failure)
    });
    let response = match result {
        Ok(frame) => PlaybackCompileResponse::Compiled {
            frame: Box::new(frame),
        },
        Err(error) => PlaybackCompileResponse::Error { error },
    };
    serde_json::to_string(&response).expect("typed playback compilation")
}
pub fn render_playback_page_json(
    input: &str,
    backend: &mut dyn RasterBackend,
    check: &dyn Fn() -> bool,
) -> (String, Vec<u8>) {
    let result = prepare(input, check).and_then(|(mut plan, at, history)| {
        plan.render_frame(at, history.as_ref(), backend, check)
            .map_err(failure)
    });
    let (response, pixels) = match result {
        Ok(image) => (
            PlaybackRasterResponse::Rendered {
                info: Box::new(image.info),
            },
            image.pixels,
        ),
        Err(error) => (PlaybackRasterResponse::Error { error }, vec![]),
    };
    (
        serde_json::to_string(&response).expect("typed playback frame"),
        pixels,
    )
}
