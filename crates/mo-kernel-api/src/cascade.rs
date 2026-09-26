use super::{MAX_REQUEST_BYTES, ShapeFailure, text::shape_failure};
use mo_common::from_json_str;
pub use mo_text::cascade::{CascadeLimits, CascadeRequest, CascadeResult};
use mo_text::{TextError, backend::TextBackend};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "status", rename_all = "camelCase", deny_unknown_fields)]
pub enum CascadeResponse {
    Evaluated { result: CascadeResult },
    Error { error: ShapeFailure },
}
pub fn shape_cascade_json(
    input: &str,
    bundle: &[u8],
    backend: &mut dyn TextBackend,
    check: &dyn Fn() -> bool,
) -> String {
    let response = if input.len() > MAX_REQUEST_BYTES {
        CascadeResponse::Error {
            error: shape_failure(TextError::Limit("font cascade request bytes")),
        }
    } else {
        match from_json_str::<CascadeRequest>(input) {
            Ok(request) => match mo_text::cascade::shape_cascade(
                &request,
                bundle,
                backend,
                CascadeLimits::default(),
                check,
            ) {
                Ok(result) => CascadeResponse::Evaluated { result },
                Err(error) => CascadeResponse::Error {
                    error: shape_failure(error),
                },
            },
            Err(error) => CascadeResponse::Error {
                error: ShapeFailure::invalid(error.to_string()),
            },
        }
    };
    serde_json::to_string(&response).expect("integer cascade response is serializable")
}
