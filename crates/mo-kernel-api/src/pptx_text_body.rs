use super::pptx_source::{inline_limits, pptx_failure};
use super::{MAX_REQUEST_BYTES, PptxFailure, PptxFailureCode};
use mo_common::from_json_str;
use mo_opc::{Package, ReaderAt};
pub use mo_pptx::source::text::body::{SourceTextBodies, SourceTextBodyQuery, TextBodyLimits};
use mo_pptx::{
    PptxError,
    source::{SourceLimits, inspect_source},
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "status", rename_all = "camelCase", deny_unknown_fields)]
pub enum PptxTextBodyResponse {
    Evaluated { styles: SourceTextBodies },
    Error { error: PptxFailure },
}

/// Range-reader computation entry, usable with host-managed cancellation.
pub fn resolve_pptx_text_bodies<R: ReaderAt>(
    request: &SourceTextBodyQuery,
    reader: R,
    length: u64,
    source_limits: SourceLimits,
    body_limits: TextBodyLimits,
    check: &dyn Fn() -> bool,
) -> PptxTextBodyResponse {
    let result = Package::open(reader, length, source_limits.package, check)
        .map_err(PptxError::from)
        .and_then(|package| inspect_source(&package, source_limits, check))
        .and_then(|index| mo_pptx::source::text::body::query(&index, request, body_limits, check));
    match result {
        Ok(styles) => PptxTextBodyResponse::Evaluated { styles },
        Err(error) => PptxTextBodyResponse::Error {
            error: pptx_failure(error),
        },
    }
}

pub fn resolve_pptx_text_bodies_json(input: &str, source: &[u8]) -> String {
    let response = if input.len() > MAX_REQUEST_BYTES {
        PptxTextBodyResponse::Error {
            error: PptxFailure {
                code: PptxFailureCode::LimitExceeded,
                message: "text body query request bytes".into(),
            },
        }
    } else {
        match from_json_str::<SourceTextBodyQuery>(input) {
            Ok(request) => resolve_pptx_text_bodies(
                &request,
                source,
                source.len() as u64,
                inline_limits(),
                TextBodyLimits::default(),
                &|| false,
            ),
            Err(error) => PptxTextBodyResponse::Error {
                error: PptxFailure {
                    code: PptxFailureCode::InputInvalid,
                    message: error.to_string(),
                },
            },
        }
    };
    serde_json::to_string(&response).expect("typed text body response is serializable")
}
