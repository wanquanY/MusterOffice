//! Source bytes, explicit font resources, and independent component capabilities
//! enter once. The host owns worker lifecycle, cancellation and publication.
pub(crate) mod author;
mod batch;
pub use batch::{
    ResourceDocumentInputs, render_resource_document_images, render_resource_document_plan,
};
mod retained;
pub use retained::render_retained_resource_document;
mod diagnostic;
mod prepare;
use crate::{pptx_page, pptx_source::inline_limits, *};
pub use author::render_author_resource_document;
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
pub use prepare::{
    PreparedPptxResourceDocument, prepare_pptx_resource_document,
    prepare_pptx_resource_document_inputs,
};

pub use source_resource_page::protocol::{
    AuthorResourceDocumentRequest, PptxResourceDocumentRequest, PptxResourcePageProfile,
    PptxResourcePageRequest, RetainedResourceDocumentRequest,
};
pub type PptxResourcePageRasterResponse =
    source_resource_page::protocol::ResourcePageRasterResponse<PptxResourcePageFailure>;
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
    let result = prepare_input(request, source, fonts, check);
    match result {
        Ok(prepared) => render_prepared_pptx_resource_page(&prepared, request, backends, check),
        Err(error) => (
            PptxResourcePageRasterResponse::Error {
                error: Box::new(error),
            },
            vec![],
        ),
    }
}
pub fn render_prepared_pptx_resource_page<R: mo_opc::ReaderAt>(
    prepared: &PreparedPptxResourceDocument<'_, R>,
    request: &PptxResourcePageRequest,
    backends: PptxResourcePageBackends<'_>,
    check: &dyn Fn() -> bool,
) -> (PptxResourcePageRasterResponse, Vec<u8>) {
    if let Err(error) = prepared.check_request(request) {
        return (
            PptxResourcePageRasterResponse::Error {
                error: Box::new(error),
            },
            vec![],
        );
    }
    render_pptx_resource_document_page(
        prepared,
        &request.page,
        request.image_source,
        request.sampling,
        backends,
        check,
    )
}

/// Renders using the document's already bound font manifest. Per-page source
/// identity and options are still validated by the shared page compiler.
pub fn render_pptx_resource_document_page<R: mo_opc::ReaderAt>(
    prepared: &PreparedPptxResourceDocument<'_, R>,
    page: &SourcePageRequest,
    image_source: ImageSourceSelection,
    sampling: mo_raster::ImageSampling,
    backends: PptxResourcePageBackends<'_>,
    check: &dyn Fn() -> bool,
) -> (PptxResourcePageRasterResponse, Vec<u8>) {
    let result = (|| {
        let package = &prepared.package;
        let index = &prepared.index;
        let manifest = &prepared.manifest;
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
            package,
            index,
            page,
            backends.decoder,
            text,
            ResourcePageOptions {
                selection: image_source,
                sampling,
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
