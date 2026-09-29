//! Digest-bound native time sampling and the shared full resource-page renderer.
use crate::*;
pub use mo_presentation_compile::source_playback::{
    PROFILE as PPTX_PLAYBACK_PROFILE, SourcePlaybackFrame,
};
use mo_presentation_compile::source_playback::{SourcePlaybackError, SourcePlaybackPlan};
use mo_presentation_compile::{
    source_page::SourcePageError,
    source_resource_page::{ResourcePageOptions, TextPageContext},
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PptxPlaybackPageRequest {
    pub page: PptxResourcePageRequest,
    /// For native source sampling, revision equals the source package SHA-256.
    /// Product revision provenance and authorization remain the host's job.
    pub sample: PlaybackSampleRequest,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "stage", rename_all = "camelCase", deny_unknown_fields)]
pub enum PptxPlaybackFailure {
    Resource { error: Box<PptxResourcePageFailure> },
    Timing { error: TimelineFailure },
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PptxPlaybackRasterInfo {
    pub profile: String,
    pub playback: SourcePlaybackFrame,
    pub page: SourceResourcePageRasterInfo,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "status", rename_all = "camelCase", deny_unknown_fields)]
pub enum PptxPlaybackRasterResponse {
    Rendered { info: Box<PptxPlaybackRasterInfo> },
    Error { error: PptxPlaybackFailure },
}
pub(crate) fn resource(error: PptxResourcePageFailure) -> PptxPlaybackFailure {
    PptxPlaybackFailure::Resource {
        error: Box::new(error),
    }
}
pub(crate) fn failure(error: SourcePlaybackError) -> PptxPlaybackFailure {
    match error {
        SourcePlaybackError::Timeline(e) => PptxPlaybackFailure::Timing {
            error: crate::timeline::failure(e),
        },
        SourcePlaybackError::RevisionConflict => PptxPlaybackFailure::Timing {
            error: TimelineFailure {
                code: TimelineFailureCode::RevisionConflict,
                message: "native playback revision does not match source package".into(),
            },
        },
        SourcePlaybackError::Page(e) => resource(match e {
            SourcePageError::Source(_) | SourcePageError::SourceConflict => {
                PptxResourcePageFailure::Source {
                    error: crate::pptx_page::failure(e),
                }
            }
            other => crate::pptx_resource_page::page_failure(other),
        }),
    }
}
pub fn render_pptx_playback_page(
    request: &PptxPlaybackPageRequest,
    source: &[u8],
    fonts: &[u8],
    backends: PptxResourcePageBackends<'_>,
    check: &dyn Fn() -> bool,
) -> (PptxPlaybackRasterResponse, Vec<u8>) {
    let result = (|| {
        let q = &request.page;
        let input =
            crate::pptx_resource_page::prepare_input(q, source, fonts, check).map_err(resource)?;
        let mut plan = SourcePlaybackPlan::new(
            &input.package,
            &input.index,
            q.page.clone(),
            request.sample.binding.clone(),
            crate::pptx_source::inline_limits(),
            TimelineLimits::default(),
            check,
        )
        .map_err(failure)?;
        let sample = plan
            .sample(request.sample.at, request.sample.history.as_ref(), check)
            .map_err(failure)?;
        let text = match input.manifest.as_ref() {
            Some(manifest) => Some(TextPageContext {
                manifest,
                backend: backends.text.ok_or_else(|| {
                    resource(crate::pptx_resource_page::request_failure(
                        PptxPageFailureCode::ResourceRequired,
                        "text component required".into(),
                    ))
                })?,
            }),
            None => None,
        };
        let page = sample
            .prepare(
                &input.package,
                &input.index,
                backends.decoder,
                text,
                ResourcePageOptions {
                    selection: q.image_source,
                    sampling: q.sampling,
                    text_limits: Default::default(),
                },
                check,
            )
            .and_then(|page| page.render(backends.raster, check))
            .map_err(|e| resource(crate::pptx_resource_page::page_failure(e)))?;
        Ok((
            PptxPlaybackRasterInfo {
                profile: sample.frame().profile().into(),
                playback: sample.frame().clone(),
                page: page.info,
            },
            page.pixels,
        ))
    })();
    match result {
        Ok((info, pixels)) => (
            PptxPlaybackRasterResponse::Rendered {
                info: Box::new(info),
            },
            pixels,
        ),
        Err(error) => (PptxPlaybackRasterResponse::Error { error }, vec![]),
    }
}
pub fn render_pptx_playback_page_json(
    input: &str,
    source: &[u8],
    fonts: &[u8],
    backends: PptxResourcePageBackends<'_>,
    check: &dyn Fn() -> bool,
) -> (String, Vec<u8>) {
    let request = if input.len() > MAX_REQUEST_BYTES {
        Err(resource(crate::pptx_resource_page::request_failure(
            PptxPageFailureCode::LimitExceeded,
            "source playback request bytes".into(),
        )))
    } else {
        mo_common::from_json_str(input).map_err(|e| {
            resource(crate::pptx_resource_page::request_failure(
                PptxPageFailureCode::InputInvalid,
                e.to_string(),
            ))
        })
    };
    let (response, pixels) = match request {
        Ok(q) => render_pptx_playback_page(&q, source, fonts, backends, check),
        Err(error) => (PptxPlaybackRasterResponse::Error { error }, vec![]),
    };
    (
        serde_json::to_string(&response).expect("typed native playback response"),
        pixels,
    )
}
