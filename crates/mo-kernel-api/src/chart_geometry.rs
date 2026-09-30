use mo_charts::sectors::SectorError;
use mo_common::from_json_str;
use mo_presentation_compile::chart_geometry::{self, ChartGeometryError};
pub use mo_presentation_compile::chart_geometry::{
    ChartGeometry, ChartGeometryLimits, ChartGeometryRequest,
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "status", rename_all = "camelCase", deny_unknown_fields)]
pub enum ChartGeometryResponse {
    Computed { geometry: ChartGeometry },
    Error { error: ChartGeometryFailure },
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ChartGeometryFailure {
    pub code: ChartGeometryFailureCode,
    pub point_index: Option<u32>,
    pub message: String,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum ChartGeometryFailureCode {
    InputInvalid,
    LimitExceeded,
    PrecisionExceeded,
    NumericRange,
    Cancelled,
}
fn failure(
    code: ChartGeometryFailureCode,
    point_index: Option<u32>,
    message: impl Into<String>,
) -> ChartGeometryResponse {
    ChartGeometryResponse::Error {
        error: ChartGeometryFailure {
            code,
            point_index,
            message: message.into(),
        },
    }
}
pub fn compile_chart_geometry(
    request: &ChartGeometryRequest,
    limits: ChartGeometryLimits,
    check: &dyn Fn() -> bool,
) -> ChartGeometryResponse {
    use ChartGeometryError as E;
    use ChartGeometryFailureCode as C;
    match chart_geometry::compile(request, limits, check) {
        Ok(geometry) => ChartGeometryResponse::Computed { geometry },
        Err(error) => {
            let point_index = match error {
                E::Sectors(SectorError::NegativeWeight(n) | SectorError::DuplicatePoint(n)) => {
                    Some(n)
                }
                _ => None,
            };
            let code = match error {
                E::Cancelled | E::Sectors(SectorError::Cancelled) => C::Cancelled,
                E::Limit(_) | E::Sectors(SectorError::Limit(_)) => C::LimitExceeded,
                E::Precision => C::PrecisionExceeded,
                E::Range => C::NumericRange,
                _ => C::InputInvalid,
            };
            failure(code, point_index, error.to_string())
        }
    }
}
pub fn compile_chart_geometry_json(input: &str, check: &dyn Fn() -> bool) -> String {
    use ChartGeometryFailureCode as C;
    let response = if check() {
        failure(C::Cancelled, None, "chart geometry cancelled")
    } else if input.len() > super::MAX_REQUEST_BYTES {
        failure(C::LimitExceeded, None, "chart geometry request bytes")
    } else {
        match from_json_str::<ChartGeometryRequest>(input) {
            Ok(request) => compile_chart_geometry(&request, ChartGeometryLimits::default(), check),
            Err(error) => failure(C::InputInvalid, None, error.to_string()),
        }
    };
    serde_json::to_string(&response).expect("bounded chart geometry response")
}
