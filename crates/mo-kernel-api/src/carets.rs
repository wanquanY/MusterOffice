use super::{MAX_REQUEST_BYTES, ShapeFailure, shape_failure};
use mo_common::from_json_str;
pub use mo_text::carets::{FontCaretsRequest, FontCaretsResult};
use mo_text::{TextError, backend::TextBackend};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "status", rename_all = "camelCase", deny_unknown_fields)]
pub enum FontCaretsResponse {
    Queried { carets: FontCaretsResult },
    Error { error: ShapeFailure },
}
pub fn font_carets_json(
    input: &str,
    bytes: &[u8],
    backend: &mut dyn TextBackend,
    check: &dyn Fn() -> bool,
) -> String {
    let response = if input.len() > MAX_REQUEST_BYTES {
        FontCaretsResponse::Error {
            error: shape_failure(TextError::Limit("font carets request bytes")),
        }
    } else {
        match from_json_str::<FontCaretsRequest>(input) {
            Ok(request) => match mo_text::carets::query(&request, bytes, backend, check) {
                Ok(carets) => FontCaretsResponse::Queried { carets },
                Err(error) => FontCaretsResponse::Error {
                    error: shape_failure(error),
                },
            },
            Err(error) => FontCaretsResponse::Error {
                error: ShapeFailure::invalid(error.to_string()),
            },
        }
    };
    serde_json::to_string(&response).expect("font carets use bounded integers")
}
