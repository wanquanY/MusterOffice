use super::{ShapeFailure, paragraph::parse, text::shape_failure};
use mo_text::backend::TextBackend;
pub use mo_text::interaction::{ParagraphInteractionRequest, ParagraphInteractionResult};
use schemars::JsonSchema;
use serde::Serialize;
#[derive(Debug, Clone, Serialize, JsonSchema)]
#[serde(tag = "status", rename_all = "camelCase", deny_unknown_fields)]
pub enum ParagraphInteractionResponse {
    Evaluated {
        result: Box<ParagraphInteractionResult>,
    },
    Error {
        error: ShapeFailure,
    },
}
pub fn paragraph_interaction_json(
    input: &str,
    bundle: &[u8],
    backend: &mut dyn TextBackend,
    check: &dyn Fn() -> bool,
) -> String {
    let result = parse::<ParagraphInteractionRequest>(input).and_then(|q| {
        mo_text::interaction::paragraph_interaction(&q, bundle, backend, check)
            .map_err(shape_failure)
    });
    let response = match result {
        Ok(result) => ParagraphInteractionResponse::Evaluated {
            result: Box::new(result),
        },
        Err(error) => ParagraphInteractionResponse::Error { error },
    };
    serde_json::to_string(&response).expect("bounded integer paragraph interaction response")
}
