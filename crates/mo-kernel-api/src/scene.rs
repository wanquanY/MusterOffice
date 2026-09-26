use super::{ShapeFailure, paragraph::parse, text::shape_failure};
use mo_text::backend::TextBackend;
pub use mo_text::scene::{ParagraphPathsRequest, ParagraphPathsResult};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "status", rename_all = "camelCase", deny_unknown_fields)]
pub enum ParagraphPathsResponse {
    Evaluated { result: Box<ParagraphPathsResult> },
    Error { error: ShapeFailure },
}
pub fn paragraph_paths_json(
    input: &str,
    bundle: &[u8],
    backend: &mut dyn TextBackend,
    check: &dyn Fn() -> bool,
) -> String {
    let result = parse::<ParagraphPathsRequest>(input).and_then(|q| {
        mo_text::scene::paragraph_paths(&q, bundle, backend, check).map_err(shape_failure)
    });
    let response = match result {
        Ok(result) => ParagraphPathsResponse::Evaluated {
            result: Box::new(result),
        },
        Err(error) => ParagraphPathsResponse::Error { error },
    };
    serde_json::to_string(&response).expect("bounded integer paragraph paths response")
}
