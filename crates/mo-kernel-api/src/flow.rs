use super::{ShapeFailure, paragraph::parse, text::shape_failure};
use mo_text::backend::TextBackend;
pub use mo_text::flow::{ParagraphLayoutRequest, ParagraphLayoutResult};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "status", rename_all = "camelCase", deny_unknown_fields)]
pub enum ParagraphLayoutResponse {
    Evaluated { result: Box<ParagraphLayoutResult> },
    Error { error: ShapeFailure },
}
pub fn layout_paragraph_json(
    input: &str,
    bundle: &[u8],
    backend: &mut dyn TextBackend,
    check: &dyn Fn() -> bool,
) -> String {
    let result = parse::<ParagraphLayoutRequest>(input).and_then(|q| {
        mo_text::flow::layout_paragraph(&q, bundle, backend, check).map_err(shape_failure)
    });
    let response = match result {
        Ok(result) => ParagraphLayoutResponse::Evaluated {
            result: Box::new(result),
        },
        Err(error) => ParagraphLayoutResponse::Error { error },
    };
    serde_json::to_string(&response).expect("bounded integer paragraph layout response")
}
