use super::MAX_REQUEST_BYTES;
use mo_common::from_json_str;
use mo_unicode::bidi::{
    BidiError, BidiLimits, BidiParagraphRequest, BidiParagraphResult, BidiProperties,
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct BidiAnalysisRequest {
    pub paragraphs: Vec<BidiParagraphRequest>,
    pub characters: Vec<u32>,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct BidiCharacterProperties {
    pub codepoint: u32,
    pub properties: BidiProperties,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(
    tag = "status",
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub enum BidiAnalysisResponse {
    Analyzed {
        unicode_version: String,
        paragraphs: Vec<BidiParagraphResult>,
        characters: Vec<BidiCharacterProperties>,
    },
    Error {
        code: BidiFailureCode,
        message: String,
    },
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum BidiFailureCode {
    InputInvalid,
    LimitExceeded,
    Cancelled,
}
fn failure(error: BidiError) -> BidiAnalysisResponse {
    BidiAnalysisResponse::Error {
        code: match error {
            BidiError::Invalid(_) => BidiFailureCode::InputInvalid,
            BidiError::Limit(_) => BidiFailureCode::LimitExceeded,
            BidiError::Cancelled => BidiFailureCode::Cancelled,
        },
        message: error.to_string(),
    }
}
pub fn analyze_bidi(
    request: &BidiAnalysisRequest,
    check: &dyn Fn() -> bool,
) -> BidiAnalysisResponse {
    let compute = || -> Result<BidiAnalysisResponse, BidiError> {
        if check() {
            return Err(BidiError::Cancelled);
        }
        if request.paragraphs.len() > 1024 || request.characters.len() > 65536 {
            return Err(BidiError::Limit("paragraphs or properties"));
        }
        let mut total_scalars = 0usize;
        let mut total_lines = 0usize;
        let mut paragraphs = Vec::with_capacity(request.paragraphs.len());
        for paragraph in &request.paragraphs {
            if check() {
                return Err(BidiError::Cancelled);
            }
            if paragraph.text.len() > 262144 {
                return Err(BidiError::Limit("text bytes"));
            }
            total_scalars += paragraph.text.chars().count();
            total_lines += paragraph.line_ends.len().max(1);
            if total_scalars > 262144 || total_lines > 65536 {
                return Err(BidiError::Limit("aggregate scalars or lines"));
            }
            paragraphs.push(mo_unicode::bidi::analyze_paragraph(
                paragraph,
                BidiLimits::default(),
                check,
            )?);
        }
        let mut characters = Vec::with_capacity(request.characters.len());
        for &codepoint in &request.characters {
            if check() {
                return Err(BidiError::Cancelled);
            }
            let c = char::from_u32(codepoint)
                .ok_or(BidiError::Invalid("property query is not a Unicode scalar"))?;
            characters.push(BidiCharacterProperties {
                codepoint,
                properties: mo_unicode::bidi::bidi_properties(c),
            });
        }
        Ok(BidiAnalysisResponse::Analyzed {
            unicode_version: mo_unicode::UNICODE_VERSION.into(),
            paragraphs,
            characters,
        })
    };
    match compute() {
        Ok(response) => response,
        Err(error) => failure(error),
    }
}
pub fn analyze_bidi_json(input: &str) -> String {
    let response = if input.len() > MAX_REQUEST_BYTES {
        failure(BidiError::Limit("request JSON bytes"))
    } else {
        match from_json_str(input) {
            Ok(request) => analyze_bidi(&request, &|| false),
            Err(error) => BidiAnalysisResponse::Error {
                code: BidiFailureCode::InputInvalid,
                message: error.to_string(),
            },
        }
    };
    serde_json::to_string(&response).expect("bidi results use exact bounded integers")
}
