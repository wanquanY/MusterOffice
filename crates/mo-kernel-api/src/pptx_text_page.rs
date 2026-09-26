//! A page operation owns no resource discovery: source and font bytes are
//! supplied separately by the host, checked once, then borrowed by the engine.
mod diagnostic;
use crate::{pptx_page, pptx_source::inline_limits, *};
pub(crate) use diagnostic::page_failure;
pub use diagnostic::*;
use mo_common::from_json_str;
pub use mo_presentation_compile::source_text_page::SourceTextPageRasterInfo;
use mo_presentation_compile::{source_page::SourcePageError, source_text_page};
use mo_raster::RasterBackend;
use mo_text::{
    backend::TextBackend,
    manifest::{FontManifest, ManifestLimits, PreparedManifest},
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, JsonSchema)]
pub enum PptxTextPageProfile {
    #[serde(rename = "drawingml-solid-text-page-q32-draft-v1")]
    SolidTextDraftV1,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PptxTextPageRequest {
    pub profile: PptxTextPageProfile,
    /// The shared shape/layer policy, digest, slide, color context and viewport.
    pub page: SourcePageRequest,
    /// Verified content ranges and exact name/instance contracts, not filenames.
    pub fonts: FontManifest,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "status", rename_all = "camelCase", deny_unknown_fields)]
pub enum PptxTextPageRasterResponse {
    Rendered { info: Box<SourceTextPageRasterInfo> },
    Error { error: Box<PptxTextPageFailure> },
}

/// Typed entry uses the same inline resource budgets as the JSON boundary.
/// Hosts own component lifetimes and publication; failure returns no pixels.
pub fn render_pptx_text_page(
    request: &PptxTextPageRequest,
    source: &[u8],
    font_bundle: &[u8],
    text: &mut dyn TextBackend,
    raster: &mut dyn RasterBackend,
    check: &dyn Fn() -> bool,
) -> (PptxTextPageRasterResponse, Vec<u8>) {
    let result = (|| {
        check_resources(source, font_bundle)?;
        let index = pptx_page::inspect(source, source.len() as u64, inline_limits(), check)
            .map_err(|error| PptxTextPageFailure::Source { error })?;
        if index.source_sha256 != request.page.expected_source_sha256 {
            return Err(PptxTextPageFailure::Source {
                error: pptx_page::failure(SourcePageError::SourceConflict),
            });
        }
        let manifest = PreparedManifest::load(
            &request.fonts,
            font_bundle,
            ManifestLimits::default(),
            check,
        )
        .map_err(|e| PptxTextPageFailure::Fonts {
            error: crate::text::shape_failure(e),
        })?;
        source_text_page::render(
            &index,
            &request.page,
            &manifest,
            text,
            raster,
            source_text_page::TextPageLimits::default(),
            check,
        )
        .map_err(diagnostic::page_failure)
    })();
    match result {
        Ok(image) => (
            PptxTextPageRasterResponse::Rendered {
                info: Box::new(image.info),
            },
            image.pixels,
        ),
        Err(error) => (
            PptxTextPageRasterResponse::Error {
                error: Box::new(error),
            },
            vec![],
        ),
    }
}
fn request_failure(code: PptxPageFailureCode, message: String) -> PptxTextPageFailure {
    PptxTextPageFailure::Request {
        error: pptx_page::basic(code, message),
    }
}
fn check_resources(source: &[u8], fonts: &[u8]) -> Result<(), PptxTextPageFailure> {
    if source.len() > MAX_INLINE_RESOURCE_BYTES || fonts.len() > MAX_INLINE_FONT_BYTES {
        Err(request_failure(
            PptxPageFailureCode::LimitExceeded,
            "text page resource bytes".into(),
        ))
    } else {
        Ok(())
    }
}
pub fn render_pptx_text_page_json(
    input: &str,
    source: &[u8],
    font_bundle: &[u8],
    text: &mut dyn TextBackend,
    raster: &mut dyn RasterBackend,
    check: &dyn Fn() -> bool,
) -> (String, Vec<u8>) {
    let request = if input.len() > MAX_REQUEST_BYTES {
        Err(request_failure(
            PptxPageFailureCode::LimitExceeded,
            "text page request bytes".into(),
        ))
    } else {
        from_json_str(input)
            .map_err(|e| request_failure(PptxPageFailureCode::InputInvalid, e.to_string()))
    };
    let (response, pixels) = match request {
        Ok(request) => render_pptx_text_page(&request, source, font_bundle, text, raster, check),
        Err(error) => (
            PptxTextPageRasterResponse::Error {
                error: Box::new(error),
            },
            vec![],
        ),
    };
    (
        serde_json::to_string(&response).expect("typed text page response"),
        pixels,
    )
}
