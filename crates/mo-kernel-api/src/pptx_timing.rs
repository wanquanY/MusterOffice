use super::{
    PptxFailure, PptxFailureCode,
    pptx_source::{inline_limits, pptx_failure},
};
use mo_common::from_json_str;
use mo_opc::{Package, ReaderAt};
pub use mo_pptx::timing::{SourceTiming, SourceTimingQuery};
use mo_pptx::{PptxError, source::SourceLimits};
use mo_timeline::TimelineLimits;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "status", rename_all = "camelCase", deny_unknown_fields)]
pub enum PptxTimingResponse {
    Inspected { timing: SourceTiming },
    Error { error: PptxFailure },
}
pub fn inspect_pptx_timing<R: ReaderAt>(
    request: &SourceTimingQuery,
    reader: R,
    length: u64,
    limits: SourceLimits,
    timing_limits: TimelineLimits,
    check: &dyn Fn() -> bool,
) -> PptxTimingResponse {
    let result = Package::open(reader, length, limits.package, check)
        .map_err(PptxError::from)
        .and_then(|package| {
            mo_pptx::timing::query(&package, request, limits, timing_limits, check)
        });
    match result {
        Ok(timing) => PptxTimingResponse::Inspected { timing },
        Err(e) => PptxTimingResponse::Error {
            error: pptx_failure(e),
        },
    }
}
pub fn inspect_pptx_timing_json(input: &str, source: &[u8]) -> String {
    let response = if input.len() > super::MAX_REQUEST_BYTES {
        PptxTimingResponse::Error {
            error: PptxFailure {
                code: PptxFailureCode::LimitExceeded,
                message: "timing query request bytes".into(),
            },
        }
    } else {
        match from_json_str::<SourceTimingQuery>(input) {
            Ok(request) => inspect_pptx_timing(
                &request,
                source,
                source.len() as u64,
                inline_limits(),
                TimelineLimits::default(),
                &|| false,
            ),
            Err(e) => PptxTimingResponse::Error {
                error: PptxFailure {
                    code: PptxFailureCode::InputInvalid,
                    message: e.to_string(),
                },
            },
        }
    };
    serde_json::to_string(&response).expect("typed source timing response")
}
