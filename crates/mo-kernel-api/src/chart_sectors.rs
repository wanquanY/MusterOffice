use mo_charts::sectors;
pub use mo_charts::sectors::{SectorLayout, SectorLimits, SectorRequest};
use mo_common::from_json_str;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "status", rename_all = "camelCase", deny_unknown_fields)]
pub enum ChartSectorsResponse {
    Computed { layout: SectorLayout },
    Error { error: ChartSectorFailure },
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ChartSectorFailure {
    pub code: ChartSectorFailureCode,
    pub point_index: Option<u32>,
    pub message: String,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum ChartSectorFailureCode {
    InputInvalid,
    LimitExceeded,
    Cancelled,
}
fn failure(code: ChartSectorFailureCode, message: impl Into<String>) -> ChartSectorsResponse {
    ChartSectorsResponse::Error {
        error: ChartSectorFailure {
            code,
            point_index: None,
            message: message.into(),
        },
    }
}
pub fn layout_chart_sectors(
    request: &SectorRequest,
    limits: SectorLimits,
    check: &dyn Fn() -> bool,
) -> ChartSectorsResponse {
    match sectors::layout(request, limits, check) {
        Ok(layout) => ChartSectorsResponse::Computed { layout },
        Err(error) => {
            let point_index = match error {
                sectors::SectorError::NegativeWeight(index)
                | sectors::SectorError::DuplicatePoint(index) => Some(index),
                _ => None,
            };
            let code = match error {
                sectors::SectorError::Limit(_) => ChartSectorFailureCode::LimitExceeded,
                sectors::SectorError::Cancelled => ChartSectorFailureCode::Cancelled,
                _ => ChartSectorFailureCode::InputInvalid,
            };
            ChartSectorsResponse::Error {
                error: ChartSectorFailure {
                    code,
                    point_index,
                    message: error.to_string(),
                },
            }
        }
    }
}
pub fn layout_chart_sectors_json(input: &str, check: &dyn Fn() -> bool) -> String {
    let response = if check() {
        failure(
            ChartSectorFailureCode::Cancelled,
            "sector computation cancelled",
        )
    } else if input.len() > super::MAX_REQUEST_BYTES {
        failure(
            ChartSectorFailureCode::LimitExceeded,
            "sector request bytes",
        )
    } else {
        match from_json_str::<SectorRequest>(input) {
            Ok(request) => layout_chart_sectors(&request, SectorLimits::default(), check),
            Err(error) => failure(ChartSectorFailureCode::InputInvalid, error.to_string()),
        }
    };
    serde_json::to_string(&response).expect("bounded chart sector response")
}
