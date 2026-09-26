use super::MAX_REQUEST_BYTES;
use mo_common::from_json_str;
use mo_font::FontError;
pub use mo_font::{FontInspection, FontLimits, FontRequest};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

pub const MAX_INLINE_FONT_BYTES: usize = 128 * 1024 * 1024;
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "status", rename_all = "camelCase", deny_unknown_fields)]
pub enum FontResponse {
    Inspected { font: Box<FontInspection> },
    Error { error: FontFailure },
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FontFailure {
    pub code: FontFailureCode,
    pub message: String,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum FontFailureCode {
    InputInvalid,
    FontInvalid,
    Unsupported,
    ResourceConflict,
    LimitExceeded,
    Cancelled,
}
fn failure(error: FontError) -> FontResponse {
    let code = match error {
        FontError::Invalid(_) | FontError::Read(_) => FontFailureCode::FontInvalid,
        FontError::InvalidRequest(_) => FontFailureCode::InputInvalid,
        FontError::Unsupported(_) => FontFailureCode::Unsupported,
        FontError::ResourceConflict => FontFailureCode::ResourceConflict,
        FontError::Limit(_) => FontFailureCode::LimitExceeded,
        FontError::Cancelled => FontFailureCode::Cancelled,
    };
    FontResponse::Error {
        error: FontFailure {
            code,
            message: error.to_string(),
        },
    }
}
pub fn inspect_font(
    request: &FontRequest,
    bytes: &[u8],
    limits: FontLimits,
    check: &dyn Fn() -> bool,
) -> FontResponse {
    match mo_font::inspect(request, bytes, limits, check) {
        Ok(font) => FontResponse::Inspected {
            font: Box::new(font),
        },
        Err(error) => failure(error),
    }
}
pub fn inspect_font_json(input: &str, bytes: &[u8]) -> String {
    let response = if input.len() > MAX_REQUEST_BYTES {
        failure(FontError::Limit("font request bytes"))
    } else {
        match from_json_str::<FontRequest>(input) {
            Ok(request) => inspect_font(&request, bytes, FontLimits::default(), &|| false),
            Err(error) => FontResponse::Error {
                error: FontFailure {
                    code: FontFailureCode::InputInvalid,
                    message: error.to_string(),
                },
            },
        }
    };
    serde_json::to_string(&response).expect("font response is serializable")
}
