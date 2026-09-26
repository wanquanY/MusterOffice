use super::{MAX_REQUEST_BYTES, TextAnalysisFailureCode};
use mo_common::from_json_str;
use mo_unicode::{
    UnicodeError, UnicodeLimits,
    line_break::{LineBreakAnalysis, LineBreakProperties},
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct LineBreakRequest {
    pub texts: Vec<String>,
    pub characters: Vec<u32>,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct LineBreakCharacterProperties {
    pub codepoint: u32,
    pub properties: LineBreakProperties,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(
    tag = "status",
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub enum LineBreakResponse {
    Analyzed {
        unicode_version: String,
        texts: Vec<LineBreakAnalysis>,
        characters: Vec<LineBreakCharacterProperties>,
    },
    Error {
        code: TextAnalysisFailureCode,
        message: String,
    },
}
fn failure(error: UnicodeError) -> LineBreakResponse {
    LineBreakResponse::Error {
        code: match error {
            UnicodeError::Cancelled => TextAnalysisFailureCode::Cancelled,
            UnicodeError::Limit(_) => TextAnalysisFailureCode::LimitExceeded,
        },
        message: error.to_string(),
    }
}
pub fn analyze_line_breaks(
    request: &LineBreakRequest,
    check: &dyn Fn() -> bool,
) -> LineBreakResponse {
    let compute = || -> Result<LineBreakResponse, UnicodeError> {
        if check() {
            return Err(UnicodeError::Cancelled);
        }
        if request.texts.len() > 1024 || request.characters.len() > 65536 {
            return Err(UnicodeError::Limit("texts or character queries"));
        }
        let mut total = 0usize;
        for text in &request.texts {
            if check() {
                return Err(UnicodeError::Cancelled);
            }
            if text.len() > 262144 {
                return Err(UnicodeError::Limit("UTF-8 bytes"));
            }
            let count = text.chars().count();
            if count > 65536 {
                return Err(UnicodeError::Limit("Unicode scalars"));
            }
            total += count;
            if total > 262144 {
                return Err(UnicodeError::Limit("aggregate scalars"));
            }
        }
        let mut characters = Vec::with_capacity(request.characters.len());
        for &codepoint in &request.characters {
            if check() {
                return Err(UnicodeError::Cancelled);
            }
            let Some(c) = char::from_u32(codepoint) else {
                return Ok(LineBreakResponse::Error {
                    code: TextAnalysisFailureCode::InputInvalid,
                    message: "property query is not a Unicode scalar".into(),
                });
            };
            characters.push(LineBreakCharacterProperties {
                codepoint,
                properties: mo_unicode::line_break::line_break_properties(c),
            });
        }
        let mut texts = Vec::with_capacity(request.texts.len());
        for text in &request.texts {
            texts.push(mo_unicode::line_break::analyze_line_breaks(
                text,
                UnicodeLimits::default(),
                check,
            )?);
        }
        if check() {
            return Err(UnicodeError::Cancelled);
        }
        Ok(LineBreakResponse::Analyzed {
            unicode_version: mo_unicode::UNICODE_VERSION.into(),
            texts,
            characters,
        })
    };
    match compute() {
        Ok(result) => result,
        Err(error) => failure(error),
    }
}
pub fn analyze_line_breaks_json(input: &str) -> String {
    let response = if input.len() > MAX_REQUEST_BYTES {
        failure(UnicodeError::Limit("request JSON bytes"))
    } else {
        match from_json_str(input) {
            Ok(request) => analyze_line_breaks(&request, &|| false),
            Err(error) => LineBreakResponse::Error {
                code: TextAnalysisFailureCode::InputInvalid,
                message: error.to_string(),
            },
        }
    };
    serde_json::to_string(&response).expect("line break results use exact bounded integers")
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn invalid_later_input_and_batch_limits_have_no_partial_results() {
        for q in [
            LineBreakRequest {
                texts: vec!["a b".into()],
                characters: vec![65, 0xd800],
            },
            LineBreakRequest {
                texts: vec!["a".repeat(65536); 5],
                characters: vec![],
            },
            LineBreakRequest {
                texts: vec![String::new(); 1025],
                characters: vec![],
            },
            LineBreakRequest {
                texts: vec![],
                characters: vec![65; 65537],
            },
        ] {
            assert!(matches!(
                analyze_line_breaks(&q, &|| false),
                LineBreakResponse::Error { .. }
            ));
        }
    }
    #[test]
    fn strict_json_and_cancellation_do_not_publish_partial_batches() {
        for q in [
            r#"{"texts":[],"texts":[],"characters":[]}"#,
            r#"{"texts":["\ud800"],"characters":[]}"#,
            r#"{"texts":[],"characters":[],"language":"system"}"#,
        ] {
            assert!(matches!(
                serde_json::from_str::<LineBreakResponse>(&analyze_line_breaks_json(q)).unwrap(),
                LineBreakResponse::Error {
                    code: TextAnalysisFailureCode::InputInvalid,
                    ..
                }
            ));
        }
        let q = LineBreakRequest {
            texts: vec!["a b".into(), "中 文".into()],
            characters: vec![65],
        };
        let n = std::cell::Cell::new(0);
        assert!(matches!(
            analyze_line_breaks(&q, &|| {
                n.set(n.get() + 1);
                false
            }),
            LineBreakResponse::Analyzed { .. }
        ));
        for stop in 1..=n.get() {
            let at = std::cell::Cell::new(0);
            assert!(matches!(
                analyze_line_breaks(&q, &|| {
                    at.set(at.get() + 1);
                    at.get() == stop
                }),
                LineBreakResponse::Error {
                    code: TextAnalysisFailureCode::Cancelled,
                    ..
                }
            ));
        }
    }
}
