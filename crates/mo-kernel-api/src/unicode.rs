use super::MAX_REQUEST_BYTES;
use mo_common::from_json_str;
use mo_unicode::{Properties, TextSegmentation, UnicodeError, UnicodeLimits};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TextAnalysisRequest {
    pub texts: Vec<String>,
    pub characters: Vec<u32>,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CharacterProperties {
    pub codepoint: u32,
    pub properties: Properties,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(
    tag = "status",
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub enum TextAnalysisResponse {
    Analyzed {
        unicode_version: String,
        texts: Vec<TextSegmentation>,
        characters: Vec<CharacterProperties>,
    },
    Error {
        code: TextAnalysisFailureCode,
        message: String,
    },
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum TextAnalysisFailureCode {
    InputInvalid,
    LimitExceeded,
    Cancelled,
}
fn failure(error: UnicodeError) -> TextAnalysisResponse {
    TextAnalysisResponse::Error {
        code: match error {
            UnicodeError::Cancelled => TextAnalysisFailureCode::Cancelled,
            UnicodeError::Limit(_) => TextAnalysisFailureCode::LimitExceeded,
        },
        message: error.to_string(),
    }
}
pub fn analyze_text(
    request: &TextAnalysisRequest,
    check: &dyn Fn() -> bool,
) -> TextAnalysisResponse {
    if check() {
        return failure(UnicodeError::Cancelled);
    }
    if request.texts.len() > 256 || request.characters.len() > 65536 {
        return failure(UnicodeError::Limit("texts or character queries"));
    }
    let mut texts = Vec::new();
    let mut characters = Vec::new();
    let mut scalars = 0usize;
    for text in &request.texts {
        let segment = match mo_unicode::segment(text, UnicodeLimits::default(), check) {
            Ok(s) => s,
            Err(error) => return failure(error),
        };
        scalars += segment
            .boundaries
            .last()
            .expect("end sentinel")
            .scalar_offset as usize;
        if scalars > 262144 {
            return failure(UnicodeError::Limit("aggregate scalars"));
        }
        texts.push(segment);
    }
    for &codepoint in &request.characters {
        if check() {
            return failure(UnicodeError::Cancelled);
        }
        let Some(c) = char::from_u32(codepoint) else {
            return TextAnalysisResponse::Error {
                code: TextAnalysisFailureCode::InputInvalid,
                message: "property query is not a Unicode scalar".into(),
            };
        };
        characters.push(CharacterProperties {
            codepoint,
            properties: mo_unicode::properties(c),
        });
    }
    TextAnalysisResponse::Analyzed {
        unicode_version: mo_unicode::UNICODE_VERSION.into(),
        texts,
        characters,
    }
}
pub fn analyze_text_json(input: &str) -> String {
    let result = if input.len() > MAX_REQUEST_BYTES {
        failure(UnicodeError::Limit("request JSON bytes"))
    } else {
        match from_json_str(input) {
            Ok(request) => analyze_text(&request, &|| false),
            Err(error) => TextAnalysisResponse::Error {
                code: TextAnalysisFailureCode::InputInvalid,
                message: error.to_string(),
            },
        }
    };
    serde_json::to_string(&result).expect("Unicode response uses exact bounded integers")
}
