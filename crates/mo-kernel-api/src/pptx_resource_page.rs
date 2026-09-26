//! Source bytes, explicit font resources, and independent component capabilities
//! enter once. The host owns worker lifecycle, cancellation and publication.
mod diagnostic;
mod prepare;
use crate::{pptx_page, pptx_source::inline_limits, *};
pub(crate) use diagnostic::page_failure;
pub use diagnostic::*;
use mo_opc::Package;
use mo_pptx::{
    PptxError,
    source::{images::ImageSourceSelection, inspect_source},
};
pub use mo_presentation_compile::source_resource_page::SourceResourcePageRasterInfo;
use mo_presentation_compile::{
    source_page::SourcePageError,
    source_resource_page::{self, ResourcePageOptions, TextPageContext},
};
use mo_text::manifest::{FontManifest, ManifestLimits, PreparedManifest};
pub(crate) use prepare::prepare_input;
#[derive(Debug, Clone, Copy, Serialize, Deserialize, JsonSchema)]
pub enum PptxResourcePageProfile {
    #[serde(rename = "drawingml-resource-page-q32-v1-draft")]
    NativeResourcesDraftV1,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PptxResourcePageRequest {
    pub profile: PptxResourcePageProfile,
    pub page: SourcePageRequest,
    pub image_source: ImageSourceSelection,
    pub sampling: mo_raster::ImageSampling,
    /// None disables source text; a visible text body then requires resources.
    /// Fonts are explicit names/content ranges, never OS discovery or paths.
    pub fonts: Option<FontManifest>,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "status", rename_all = "camelCase", deny_unknown_fields)]
pub enum PptxResourcePageRasterResponse {
    Rendered {
        info: Box<SourceResourcePageRasterInfo>,
    },
    Error {
        error: Box<PptxResourcePageFailure>,
    },
}
pub struct PptxResourcePageBackends<'a> {
    pub decoder: &'a mut dyn mo_image::ImageDecoder,
    pub text: Option<&'a mut dyn mo_text::backend::TextBackend>,
    pub raster: &'a mut dyn mo_raster::RasterBackend,
}
pub(crate) fn request_failure(
    code: PptxPageFailureCode,
    message: String,
) -> PptxResourcePageFailure {
    PptxResourcePageFailure::Request {
        error: pptx_page::basic(code, message),
    }
}
pub fn render_pptx_resource_page(
    request: &PptxResourcePageRequest,
    source: &[u8],
    fonts: &[u8],
    backends: PptxResourcePageBackends<'_>,
    check: &dyn Fn() -> bool,
) -> (PptxResourcePageRasterResponse, Vec<u8>) {
    let result = (|| {
        let prepared = prepare_input(request, source, fonts, check)?;
        let prepare::PreparedInput {
            package,
            index,
            manifest,
        } = prepared;
        let text = match manifest.as_ref() {
            Some(manifest) => Some(TextPageContext {
                manifest,
                backend: backends.text.ok_or_else(|| {
                    request_failure(
                        PptxPageFailureCode::ResourceRequired,
                        "text component required".into(),
                    )
                })?,
            }),
            None => None,
        };
        source_resource_page::prepare(
            &package,
            &index,
            &request.page,
            backends.decoder,
            text,
            ResourcePageOptions {
                selection: request.image_source,
                sampling: request.sampling,
                text_limits: Default::default(),
            },
            check,
        )
        .and_then(|page| page.render(backends.raster, check))
        .map_err(diagnostic::page_failure)
    })();
    match result {
        Ok(image) => (
            PptxResourcePageRasterResponse::Rendered {
                info: Box::new(image.info),
            },
            image.pixels,
        ),
        Err(error) => (
            PptxResourcePageRasterResponse::Error {
                error: Box::new(error),
            },
            vec![],
        ),
    }
}
pub fn render_pptx_resource_page_json(
    input: &str,
    source: &[u8],
    fonts: &[u8],
    backends: PptxResourcePageBackends<'_>,
    check: &dyn Fn() -> bool,
) -> (String, Vec<u8>) {
    let request = if input.len() > MAX_REQUEST_BYTES {
        Err(request_failure(
            PptxPageFailureCode::LimitExceeded,
            "resource page request bytes".into(),
        ))
    } else {
        mo_common::from_json_str(input)
            .map_err(|e| request_failure(PptxPageFailureCode::InputInvalid, e.to_string()))
    };
    let (response, pixels) = match request {
        Ok(request) => render_pptx_resource_page(&request, source, fonts, backends, check),
        Err(error) => (
            PptxResourcePageRasterResponse::Error {
                error: Box::new(error),
            },
            vec![],
        ),
    };
    (
        serde_json::to_string(&response).expect("typed resource page response"),
        pixels,
    )
}
