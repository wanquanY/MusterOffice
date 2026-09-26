use super::pptx_source::{inline_limits, pptx_failure};
use super::{MAX_REQUEST_BYTES, PptxFailure, PptxFailureCode};
use mo_common::from_json_str;
use mo_opc::{Package, ReaderAt};
pub use mo_pptx::source::geometry::evaluate::{
    GeometryLimits, SourceGeometryQuery, SourceGeometryValues,
};
use mo_pptx::{
    PptxError,
    source::{SourceLimits, inspect_source},
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "status", rename_all = "camelCase", deny_unknown_fields)]
pub enum PptxGeometryResponse {
    Evaluated { geometry: SourceGeometryValues },
    Error { error: PptxFailure },
}

/// Range-reader computation entry, usable with host-managed cancellation.
pub fn evaluate_pptx_geometry<R: ReaderAt>(
    request: &SourceGeometryQuery,
    reader: R,
    length: u64,
    source_limits: SourceLimits,
    geometry_limits: GeometryLimits,
    check: &dyn Fn() -> bool,
) -> PptxGeometryResponse {
    let result = Package::open(reader, length, source_limits.package, check)
        .map_err(PptxError::from)
        .and_then(|package| inspect_source(&package, source_limits, check))
        .and_then(|index| {
            mo_pptx::source::geometry::evaluate::query(&index, request, geometry_limits, check)
        });
    match result {
        Ok(geometry) => PptxGeometryResponse::Evaluated { geometry },
        Err(error) => PptxGeometryResponse::Error {
            error: pptx_failure(error),
        },
    }
}

pub fn evaluate_pptx_geometry_json(input: &str, source: &[u8]) -> String {
    let response = if input.len() > MAX_REQUEST_BYTES {
        PptxGeometryResponse::Error {
            error: PptxFailure {
                code: PptxFailureCode::LimitExceeded,
                message: "geometry query request bytes".into(),
            },
        }
    } else {
        match from_json_str::<SourceGeometryQuery>(input) {
            Ok(request) => evaluate_pptx_geometry(
                &request,
                source,
                source.len() as u64,
                inline_limits(),
                GeometryLimits::default(),
                &|| false,
            ),
            Err(error) => PptxGeometryResponse::Error {
                error: PptxFailure {
                    code: PptxFailureCode::InputInvalid,
                    message: error.to_string(),
                },
            },
        }
    };
    serde_json::to_string(&response).expect("typed geometry response is serializable")
}
