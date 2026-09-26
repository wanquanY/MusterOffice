use super::{ShapeFailure, paragraph::parse, text::shape_failure};
use mo_text::backend::TextBackend;
pub use mo_text::geometry::{LineGeometryRequest, LineGeometryResult};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "status", rename_all = "camelCase", deny_unknown_fields)]
pub enum LineGeometryResponse {
    Evaluated { result: Box<LineGeometryResult> },
    Error { error: ShapeFailure },
}
pub fn layout_lines_json(
    input: &str,
    bundle: &[u8],
    backend: &mut dyn TextBackend,
    check: &dyn Fn() -> bool,
) -> String {
    let result = parse::<LineGeometryRequest>(input).and_then(|r| {
        mo_text::geometry::layout_lines(&r, bundle, backend, check).map_err(shape_failure)
    });
    let response = match result {
        Ok(result) => LineGeometryResponse::Evaluated {
            result: Box::new(result),
        },
        Err(error) => LineGeometryResponse::Error { error },
    };
    serde_json::to_string(&response).expect("bounded integer line geometry response")
}
