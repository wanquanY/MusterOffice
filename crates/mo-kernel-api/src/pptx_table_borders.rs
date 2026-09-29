use super::pptx_source::{inline_limits, pptx_failure};
use super::{MAX_REQUEST_BYTES, PptxFailure, PptxFailureCode};
use mo_common::from_json_str;
use mo_opc::{Package, ReaderAt};
pub use mo_pptx::source::table::borders::{
    SourceTableBorderQuery, SourceTableBorders, TableBorderLimits,
};
use mo_pptx::{
    PptxError,
    source::{SourceLimits, inspect_source},
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "status", rename_all = "camelCase", deny_unknown_fields)]
pub enum PptxTableBorderResponse {
    Evaluated { borders: SourceTableBorders },
    Error { error: PptxFailure },
}

/// Range-reader computation entry, usable with host-managed cancellation.
pub fn resolve_pptx_table_borders<R: ReaderAt>(
    request: &SourceTableBorderQuery,
    reader: R,
    length: u64,
    source_limits: SourceLimits,
    border_limits: TableBorderLimits,
    check: &dyn Fn() -> bool,
) -> PptxTableBorderResponse {
    let result = Package::open(reader, length, source_limits.package, check)
        .map_err(PptxError::from)
        .and_then(|package| inspect_source(&package, source_limits, check))
        .and_then(|index| {
            mo_pptx::source::table::borders::query(&index, request, border_limits, check)
        });
    match result {
        Ok(borders) => PptxTableBorderResponse::Evaluated { borders },
        Err(error) => PptxTableBorderResponse::Error {
            error: pptx_failure(error),
        },
    }
}

pub fn resolve_pptx_table_borders_json(input: &str, source: &[u8]) -> String {
    let response = if input.len() > MAX_REQUEST_BYTES {
        PptxTableBorderResponse::Error {
            error: PptxFailure {
                code: PptxFailureCode::LimitExceeded,
                message: "table border query request bytes".into(),
            },
        }
    } else {
        match from_json_str::<SourceTableBorderQuery>(input) {
            Ok(request) => resolve_pptx_table_borders(
                &request,
                source,
                source.len() as u64,
                inline_limits(),
                TableBorderLimits::default(),
                &|| false,
            ),
            Err(error) => PptxTableBorderResponse::Error {
                error: PptxFailure {
                    code: PptxFailureCode::InputInvalid,
                    message: error.to_string(),
                },
            },
        }
    };
    serde_json::to_string(&response).expect("typed table border response is serializable")
}
