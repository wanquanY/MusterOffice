use mo_common::from_json_str;
use mo_presentation_compile::CompileError;
pub use mo_presentation_compile::{PagePlacementRequest, PagePlacements};
use mo_presentation_model::{ValidationCode, ValidationReport};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum PlacementFailureCode {
    InputInvalid,
    LimitExceeded,
    CoordinateRange,
    Cancelled,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PlacementFailure {
    pub code: PlacementFailureCode,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub report: Option<ValidationReport>,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "status", rename_all = "camelCase", deny_unknown_fields)]
pub enum PagePlacementResponse {
    Evaluated { result: Box<PagePlacements> },
    Error { error: PlacementFailure },
}
pub fn page_placements_json(input: &str, check: &dyn Fn() -> bool) -> String {
    let result = if input.len() > super::MAX_REQUEST_BYTES {
        Err(PlacementFailure {
            code: PlacementFailureCode::LimitExceeded,
            message: "placement request bytes".into(),
            report: None,
        })
    } else {
        from_json_str::<PagePlacementRequest>(input)
            .map_err(|e| PlacementFailure {
                code: PlacementFailureCode::InputInvalid,
                message: e.to_string(),
                report: None,
            })
            .and_then(|q| {
                mo_presentation_compile::page_placements(&q, check).map_err(|e| {
                    let message = e.to_string();
                    let (code, report) = match e {
                        CompileError::Document(r) => (
                            if r.issues
                                .iter()
                                .any(|i| i.code == ValidationCode::LimitExceeded)
                            {
                                PlacementFailureCode::LimitExceeded
                            } else {
                                PlacementFailureCode::InputInvalid
                            },
                            Some(r),
                        ),
                        CompileError::Invalid(_) => (PlacementFailureCode::InputInvalid, None),
                        CompileError::Limit(_) => (PlacementFailureCode::LimitExceeded, None),
                        CompileError::Range => (PlacementFailureCode::CoordinateRange, None),
                        CompileError::Cancelled => (PlacementFailureCode::Cancelled, None),
                    };
                    PlacementFailure {
                        code,
                        message,
                        report,
                    }
                })
            })
    };
    let response = match result {
        Ok(result) => PagePlacementResponse::Evaluated {
            result: Box::new(result),
        },
        Err(error) => PagePlacementResponse::Error { error },
    };
    serde_json::to_string(&response).expect("bounded placement response")
}
