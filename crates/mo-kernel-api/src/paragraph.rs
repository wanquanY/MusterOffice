use super::{MAX_REQUEST_BYTES, ShapeFailure, text::shape_failure};
use mo_common::from_json_str;
use mo_text::{TextError, backend::TextBackend};
pub use mo_text::{
    itemize::{ItemizationRequest, ItemizationResult},
    lines::{LineShapeRequest, LineShapeResult},
    paragraph::{ParagraphShapeRequest, ParagraphShapeResult},
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "status", rename_all = "camelCase", deny_unknown_fields)]
pub enum ItemizationResponse {
    Itemized { result: ItemizationResult },
    Error { error: ShapeFailure },
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "status", rename_all = "camelCase", deny_unknown_fields)]
pub enum ParagraphShapeResponse {
    Evaluated { result: Box<ParagraphShapeResult> },
    Error { error: ShapeFailure },
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "status", rename_all = "camelCase", deny_unknown_fields)]
pub enum LineShapeResponse {
    Evaluated { result: Box<LineShapeResult> },
    Error { error: ShapeFailure },
}
pub fn shape_lines_json(
    input: &str,
    bundle: &[u8],
    backend: &mut dyn TextBackend,
    check: &dyn Fn() -> bool,
) -> String {
    let result = parse::<LineShapeRequest>(input).and_then(|r| {
        mo_text::lines::shape_lines(&r, bundle, backend, check).map_err(shape_failure)
    });
    let response = match result {
        Ok(result) => LineShapeResponse::Evaluated {
            result: Box::new(result),
        },
        Err(error) => LineShapeResponse::Error { error },
    };
    serde_json::to_string(&response).expect("bounded integer line shaping response")
}
pub(super) fn parse<T: serde::de::DeserializeOwned>(input: &str) -> Result<T, ShapeFailure> {
    if input.len() > MAX_REQUEST_BYTES {
        return Err(shape_failure(TextError::Limit("paragraph request bytes")));
    }
    from_json_str(input).map_err(|e| ShapeFailure::invalid(e.to_string()))
}
pub fn itemize_paragraph_json(input: &str) -> String {
    let result = parse::<ItemizationRequest>(input).and_then(|r| {
        mo_text::itemize::itemize(&r, mo_text::itemize::ItemizationLimits::default(), &|| {
            false
        })
        .map_err(shape_failure)
    });
    let response = match result {
        Ok(result) => ItemizationResponse::Itemized { result },
        Err(error) => ItemizationResponse::Error { error },
    };
    serde_json::to_string(&response).expect("bounded integer itemization response")
}
pub fn shape_paragraph_json(
    input: &str,
    bundle: &[u8],
    backend: &mut dyn TextBackend,
    check: &dyn Fn() -> bool,
) -> String {
    let result = parse::<ParagraphShapeRequest>(input).and_then(|r| {
        mo_text::paragraph::shape_paragraph(&r, bundle, backend, check).map_err(shape_failure)
    });
    let response = match result {
        Ok(result) => ParagraphShapeResponse::Evaluated {
            result: Box::new(result),
        },
        Err(error) => ParagraphShapeResponse::Error { error },
    };
    serde_json::to_string(&response).expect("bounded integer paragraph response")
}
