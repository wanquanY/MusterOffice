use super::pptx_source::{inline_limits, pptx_failure};
use super::{MAX_REQUEST_BYTES, PptxFailure, PptxFailureCode};
use mo_common::from_json_str;
use mo_opc::{Package, ReaderAt};
pub use mo_pptx::source::images::{SourceImageLimits, SourceImageQuery, SourceImageResources};
use mo_pptx::{
    PptxError,
    source::{SourceLimits, inspect_source},
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "status", rename_all = "camelCase", deny_unknown_fields)]
pub enum PptxImagesResponse {
    Inspected { images: Box<SourceImageResources> },
    Error { error: PptxFailure },
}
pub fn inspect_pptx_images<R: ReaderAt>(
    request: &SourceImageQuery,
    reader: R,
    length: u64,
    source_limits: SourceLimits,
    image_limits: SourceImageLimits,
    extract: bool,
    check: &dyn Fn() -> bool,
) -> (PptxImagesResponse, Vec<u8>) {
    let result = (|| {
        let package = Package::open(reader, length, source_limits.package, check)?;
        let index = inspect_source(&package, source_limits, check)?;
        let images =
            mo_pptx::source::images::query(&package, &index, request, image_limits, check)?;
        let bytes = if extract {
            mo_pptx::source::images::extract(&package, &images, image_limits, check)?
        } else {
            vec![]
        };
        Ok::<_, PptxError>((images, bytes))
    })();
    match result {
        Ok((images, bytes)) => (
            PptxImagesResponse::Inspected {
                images: Box::new(images),
            },
            bytes,
        ),
        Err(error) => (
            PptxImagesResponse::Error {
                error: pptx_failure(error),
            },
            vec![],
        ),
    }
}
fn execute(input: &str, source: &[u8], extract: bool) -> (String, Vec<u8>) {
    let (response, bytes) = if input.len() > MAX_REQUEST_BYTES {
        (
            PptxImagesResponse::Error {
                error: PptxFailure {
                    code: PptxFailureCode::LimitExceeded,
                    message: "image query request bytes".into(),
                },
            },
            vec![],
        )
    } else {
        match from_json_str::<SourceImageQuery>(input) {
            Ok(request) => inspect_pptx_images(
                &request,
                source,
                source.len() as u64,
                inline_limits(),
                SourceImageLimits::default(),
                extract,
                &|| false,
            ),
            Err(error) => (
                PptxImagesResponse::Error {
                    error: PptxFailure {
                        code: PptxFailureCode::InputInvalid,
                        message: error.to_string(),
                    },
                },
                vec![],
            ),
        }
    };
    (
        serde_json::to_string(&response).expect("bounded image resource response"),
        bytes,
    )
}
pub fn inspect_pptx_images_json(input: &str, source: &[u8]) -> String {
    execute(input, source, false).0
}
pub fn extract_pptx_images_json(input: &str, source: &[u8]) -> (String, Vec<u8>) {
    execute(input, source, true)
}
