//! Digest-bound placement of imported objects; independent of host I/O.
use super::pptx_source::{inline_limits, pptx_failure};
use super::{MAX_REQUEST_BYTES, PptxFailure, PptxFailureCode};
use mo_common::from_json_str;
use mo_opc::{Package, ReaderAt};
use mo_pptx::{
    PptxError,
    source::{SourceLimits, inspect_source},
};
use mo_presentation_compile::source_placement::{SourcePlacementError, source_placements};
pub use mo_presentation_compile::source_placement::{
    SourcePlacementLimits, SourcePlacementQuery, SourcePlacements,
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "status", rename_all = "camelCase", deny_unknown_fields)]
pub enum PptxPlacementResponse {
    Evaluated { placements: SourcePlacements },
    Error { error: PptxFailure },
}
fn failure(error: SourcePlacementError) -> PptxFailure {
    PptxFailure {
        code: match error {
            SourcePlacementError::Cancelled => PptxFailureCode::Cancelled,
            SourcePlacementError::Limit(_) => PptxFailureCode::LimitExceeded,
            SourcePlacementError::SourceConflict => PptxFailureCode::SourceConflict,
            SourcePlacementError::Invalid(_) => PptxFailureCode::InputInvalid,
        },
        message: error.to_string(),
    }
}
pub fn place_pptx_objects<R: ReaderAt>(
    request: &SourcePlacementQuery,
    reader: R,
    length: u64,
    source_limits: SourceLimits,
    placement_limits: SourcePlacementLimits,
    check: &dyn Fn() -> bool,
) -> PptxPlacementResponse {
    let result = Package::open(reader, length, source_limits.package, check)
        .map_err(PptxError::from)
        .and_then(|package| inspect_source(&package, source_limits, check))
        .map_err(pptx_failure)
        .and_then(|index| {
            source_placements(&index, request, placement_limits, check).map_err(failure)
        });
    match result {
        Ok(placements) => PptxPlacementResponse::Evaluated { placements },
        Err(error) => PptxPlacementResponse::Error { error },
    }
}
pub fn place_pptx_objects_json(input: &str, source: &[u8]) -> String {
    let response = if input.len() > MAX_REQUEST_BYTES {
        PptxPlacementResponse::Error {
            error: PptxFailure {
                code: PptxFailureCode::LimitExceeded,
                message: "source placement request bytes".into(),
            },
        }
    } else {
        match from_json_str::<SourcePlacementQuery>(input) {
            Ok(request) => place_pptx_objects(
                &request,
                source,
                source.len() as u64,
                inline_limits(),
                SourcePlacementLimits::default(),
                &|| false,
            ),
            Err(error) => PptxPlacementResponse::Error {
                error: PptxFailure {
                    code: PptxFailureCode::InputInvalid,
                    message: error.to_string(),
                },
            },
        }
    };
    serde_json::to_string(&response).expect("typed placement response is serializable")
}
