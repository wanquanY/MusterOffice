use super::{FontLimits, MAX_REQUEST_BYTES};
use mo_common::from_json_str;
use mo_font::FontError;
pub use mo_text::{ShapeRequest, ShapedText, TextLimits};
use mo_text::{TextError, backend::TextBackend};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "status", rename_all = "camelCase", deny_unknown_fields)]
pub enum ShapeResponse {
    Shaped { text: ShapedText },
    Error { error: ShapeFailure },
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ShapeFailure {
    pub code: ShapeFailureCode,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub font_selection: Option<Box<mo_text::manifest::FontSelectionFailure>>,
}
impl ShapeFailure {
    pub(crate) fn invalid(message: String) -> Self {
        Self {
            code: ShapeFailureCode::InputInvalid,
            message,
            font_selection: None,
        }
    }
}
impl From<TextError> for ShapeFailure {
    fn from(error: TextError) -> Self {
        shape_failure(error)
    }
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ShapeFailureCode {
    InputInvalid,
    FontInvalid,
    Unsupported,
    ResourceConflict,
    ResourceRequired,
    LimitExceeded,
    Cancelled,
    ComponentFailure,
    ComponentInvalid,
    HostFailure,
}
pub(crate) fn shape_failure(error: TextError) -> ShapeFailure {
    use ShapeFailureCode::*;
    let code = match &error {
        TextError::Invalid(_) | TextError::Font(FontError::InvalidRequest(_)) => InputInvalid,
        TextError::Font(FontError::Invalid(_) | FontError::Read(_)) => FontInvalid,
        TextError::Font(FontError::Unsupported(_)) => Unsupported,
        TextError::Font(FontError::ResourceConflict) => ResourceConflict,
        TextError::FontSelection(_) => ResourceRequired,
        TextError::Limit(_)
        | TextError::Font(FontError::Limit(_))
        | TextError::BackendFailure { status: 4, .. } => LimitExceeded,
        TextError::Cancelled | TextError::Font(FontError::Cancelled) => Cancelled,
        TextError::BackendInvalid(_) => ComponentInvalid,
        TextError::BackendFailure { .. } => ComponentFailure,
        TextError::Host(_) => HostFailure,
    };
    let message = error.to_string();
    let font_selection = if let TextError::FontSelection(selection) = error {
        Some(selection)
    } else {
        None
    };
    ShapeFailure {
        code,
        message,
        font_selection,
    }
}
fn failure(error: TextError) -> ShapeResponse {
    ShapeResponse::Error {
        error: shape_failure(error),
    }
}
pub fn shape_text_json(
    input: &str,
    bytes: &[u8],
    backend: &mut dyn TextBackend,
    check: &dyn Fn() -> bool,
) -> String {
    let result = if input.len() > MAX_REQUEST_BYTES {
        failure(TextError::Limit("text request bytes"))
    } else {
        match from_json_str::<ShapeRequest>(input) {
            Ok(request) => match mo_text::shape(
                &request,
                bytes,
                backend,
                TextLimits::default(),
                FontLimits::default(),
                check,
            ) {
                Ok(text) => ShapeResponse::Shaped { text },
                Err(error) => failure(error),
            },
            Err(error) => ShapeResponse::Error {
                error: ShapeFailure::invalid(error.to_string()),
            },
        }
    };
    serde_json::to_string(&result).expect("integer shaping response is serializable")
}
