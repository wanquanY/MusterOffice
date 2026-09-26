use super::{MAX_REQUEST_BYTES, ShapeFailure, shape_failure};
use mo_common::from_json_str;
pub use mo_text::outlines::{FontOutlinesRequest, FontOutlinesResult};
use mo_text::{TextError, backend::TextBackend};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "status", rename_all = "camelCase", deny_unknown_fields)]
pub enum FontOutlinesResponse {
    Outlined { outlines: FontOutlinesResult },
    Error { error: ShapeFailure },
}
pub fn outline_font_json(
    input: &str,
    bytes: &[u8],
    backend: &mut dyn TextBackend,
    check: &dyn Fn() -> bool,
) -> String {
    let response = if input.len() > MAX_REQUEST_BYTES {
        FontOutlinesResponse::Error {
            error: shape_failure(TextError::Limit("font outlines request bytes")),
        }
    } else {
        match from_json_str::<FontOutlinesRequest>(input) {
            Ok(request) => match mo_text::outlines::extract(&request, bytes, backend, check) {
                Ok(outlines) => FontOutlinesResponse::Outlined { outlines },
                Err(error) => FontOutlinesResponse::Error {
                    error: shape_failure(error),
                },
            },
            Err(error) => FontOutlinesResponse::Error {
                error: ShapeFailure::invalid(error.to_string()),
            },
        }
    };
    serde_json::to_string(&response).expect("font outlines use bounded integers")
}
