use super::{MAX_REQUEST_BYTES, ShapeFailure, shape_failure};
use mo_common::from_json_str;
pub use mo_text::metrics::{FontMetricsRequest, FontMetricsResult};
use mo_text::{TextError, backend::TextBackend};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "status", rename_all = "camelCase", deny_unknown_fields)]
pub enum FontMetricsResponse {
    Measured { metrics: FontMetricsResult },
    Error { error: ShapeFailure },
}
pub fn measure_font_json(
    input: &str,
    bytes: &[u8],
    backend: &mut dyn TextBackend,
    check: &dyn Fn() -> bool,
) -> String {
    let response = if input.len() > MAX_REQUEST_BYTES {
        FontMetricsResponse::Error {
            error: shape_failure(TextError::Limit("font metrics request bytes")),
        }
    } else {
        match from_json_str::<FontMetricsRequest>(input) {
            Ok(request) => match mo_text::metrics::measure(&request, bytes, backend, check) {
                Ok(metrics) => FontMetricsResponse::Measured { metrics },
                Err(error) => FontMetricsResponse::Error {
                    error: shape_failure(error),
                },
            },
            Err(error) => FontMetricsResponse::Error {
                error: ShapeFailure::invalid(error.to_string()),
            },
        }
    };
    serde_json::to_string(&response).expect("font metrics use bounded integers")
}
