use super::pptx_source::{inline_limits, pptx_failure};
use super::{MAX_REQUEST_BYTES, PptxFailure, PptxFailureCode};
use mo_common::from_json_str;
use mo_opc::{Package, ReaderAt};
pub use mo_pptx::source::color::{ColorLimits, SourceColorPalette, SourceColorQuery};
use mo_pptx::{
    PptxError,
    source::{SourceLimits, inspect_source},
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "status", rename_all = "camelCase", deny_unknown_fields)]
pub enum PptxColorResponse {
    Evaluated { palette: SourceColorPalette },
    Error { error: PptxFailure },
}

/// Range-reader computation entry, usable with host-managed cancellation.
pub fn resolve_pptx_colors<R: ReaderAt>(
    request: &SourceColorQuery,
    reader: R,
    length: u64,
    source_limits: SourceLimits,
    color_limits: ColorLimits,
    check: &dyn Fn() -> bool,
) -> PptxColorResponse {
    let result = Package::open(reader, length, source_limits.package, check)
        .map_err(PptxError::from)
        .and_then(|package| inspect_source(&package, source_limits, check))
        .and_then(|index| mo_pptx::source::color::query(&index, request, color_limits, check));
    match result {
        Ok(palette) => PptxColorResponse::Evaluated { palette },
        Err(error) => PptxColorResponse::Error {
            error: pptx_failure(error),
        },
    }
}

pub fn resolve_pptx_colors_json(input: &str, source: &[u8]) -> String {
    let response = if input.len() > MAX_REQUEST_BYTES {
        PptxColorResponse::Error {
            error: PptxFailure {
                code: PptxFailureCode::LimitExceeded,
                message: "color query request bytes".into(),
            },
        }
    } else {
        match from_json_str::<SourceColorQuery>(input) {
            Ok(request) => resolve_pptx_colors(
                &request,
                source,
                source.len() as u64,
                inline_limits(),
                ColorLimits::default(),
                &|| false,
            ),
            Err(error) => PptxColorResponse::Error {
                error: PptxFailure {
                    code: PptxFailureCode::InputInvalid,
                    message: error.to_string(),
                },
            },
        }
    };
    serde_json::to_string(&response).expect("typed color response is serializable")
}
