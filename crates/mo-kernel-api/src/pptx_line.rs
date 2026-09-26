use super::pptx_source::{inline_limits, pptx_failure};
use super::{MAX_REQUEST_BYTES, PptxFailure, PptxFailureCode};
use mo_common::from_json_str;
use mo_opc::{Package, ReaderAt};
pub use mo_pptx::source::line::resolve::{LineResolveLimits, SourceLineQuery, SourceLineStyles};
use mo_pptx::{
    PptxError,
    source::{SourceLimits, inspect_source},
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "status", rename_all = "camelCase", deny_unknown_fields)]
pub enum PptxLineResponse {
    Evaluated { styles: SourceLineStyles },
    Error { error: PptxFailure },
}

/// Range-reader computation entry, usable with host-managed cancellation.
pub fn resolve_pptx_lines<R: ReaderAt>(
    request: &SourceLineQuery,
    reader: R,
    length: u64,
    source_limits: SourceLimits,
    line_limits: LineResolveLimits,
    check: &dyn Fn() -> bool,
) -> PptxLineResponse {
    let result = Package::open(reader, length, source_limits.package, check)
        .map_err(PptxError::from)
        .and_then(|package| inspect_source(&package, source_limits, check))
        .and_then(|index| {
            mo_pptx::source::line::resolve::query(&index, request, line_limits, check)
        });
    match result {
        Ok(styles) => PptxLineResponse::Evaluated { styles },
        Err(error) => PptxLineResponse::Error {
            error: pptx_failure(error),
        },
    }
}

pub fn resolve_pptx_lines_json(input: &str, source: &[u8]) -> String {
    let response = if input.len() > MAX_REQUEST_BYTES {
        PptxLineResponse::Error {
            error: PptxFailure {
                code: PptxFailureCode::LimitExceeded,
                message: "line query request bytes".into(),
            },
        }
    } else {
        match from_json_str::<SourceLineQuery>(input) {
            Ok(request) => resolve_pptx_lines(
                &request,
                source,
                source.len() as u64,
                inline_limits(),
                LineResolveLimits::default(),
                &|| false,
            ),
            Err(error) => PptxLineResponse::Error {
                error: PptxFailure {
                    code: PptxFailureCode::InputInvalid,
                    message: error.to_string(),
                },
            },
        }
    };
    serde_json::to_string(&response).expect("typed line response is serializable")
}
