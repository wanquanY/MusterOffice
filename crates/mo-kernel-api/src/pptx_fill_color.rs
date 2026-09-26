use super::pptx_source::{inline_limits, pptx_failure};
use super::{MAX_REQUEST_BYTES, PptxFailure, PptxFailureCode};
use mo_common::from_json_str;
use mo_opc::{Package, ReaderAt};
pub use mo_pptx::source::fill::colors::{FillColorLimits, SourceFillColorQuery, SourceFillColors};
use mo_pptx::{
    PptxError,
    source::{SourceLimits, inspect_source},
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "status", rename_all = "camelCase", deny_unknown_fields)]
pub enum PptxFillColorResponse {
    Evaluated { colors: SourceFillColors },
    Error { error: PptxFailure },
}

/// Range-reader computation entry, usable with host-managed cancellation.
pub fn resolve_pptx_fill_colors<R: ReaderAt>(
    request: &SourceFillColorQuery,
    reader: R,
    length: u64,
    source_limits: SourceLimits,
    fill_limits: FillColorLimits,
    check: &dyn Fn() -> bool,
) -> PptxFillColorResponse {
    let result = Package::open(reader, length, source_limits.package, check)
        .map_err(PptxError::from)
        .and_then(|package| inspect_source(&package, source_limits, check))
        .and_then(|index| {
            mo_pptx::source::fill::colors::query(&index, request, fill_limits, check)
        });
    match result {
        Ok(colors) => PptxFillColorResponse::Evaluated { colors },
        Err(error) => PptxFillColorResponse::Error {
            error: pptx_failure(error),
        },
    }
}

pub fn resolve_pptx_fill_colors_json(input: &str, source: &[u8]) -> String {
    let response = if input.len() > MAX_REQUEST_BYTES {
        PptxFillColorResponse::Error {
            error: PptxFailure {
                code: PptxFailureCode::LimitExceeded,
                message: "fill color query request bytes".into(),
            },
        }
    } else {
        match from_json_str::<SourceFillColorQuery>(input) {
            Ok(request) => resolve_pptx_fill_colors(
                &request,
                source,
                source.len() as u64,
                inline_limits(),
                FillColorLimits::default(),
                &|| false,
            ),
            Err(error) => PptxFillColorResponse::Error {
                error: PptxFailure {
                    code: PptxFailureCode::InputInvalid,
                    message: error.to_string(),
                },
            },
        }
    };
    serde_json::to_string(&response).expect("typed fill color response is serializable")
}
