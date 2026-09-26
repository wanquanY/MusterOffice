use mo_common::from_json_str;
pub use mo_raster::{PathRasterRequest, RasterInfo};
use mo_raster::{RasterBackend, RasterError};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum RasterFailureCode {
    InputInvalid,
    LimitExceeded,
    CoordinateRange,
    PrecisionExceeded,
    Cancelled,
    ComponentFailure,
    ComponentInvalid,
    HostFailure,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RasterFailure {
    pub code: RasterFailureCode,
    pub message: String,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "status", rename_all = "camelCase", deny_unknown_fields)]
pub enum PathRasterResponse {
    Rendered { info: Box<RasterInfo> },
    Error { error: RasterFailure },
}
pub(crate) fn failure(e: RasterError) -> RasterFailure {
    use RasterFailureCode::*;
    let code = match e {
        RasterError::Invalid(_) => InputInvalid,
        RasterError::Limit(_) => LimitExceeded,
        RasterError::Range => CoordinateRange,
        RasterError::Precision => PrecisionExceeded,
        RasterError::Cancelled => Cancelled,
        RasterError::Component(_) => ComponentFailure,
        RasterError::ComponentInvalid(_) => ComponentInvalid,
        RasterError::Host(_) => HostFailure,
    };
    RasterFailure {
        code,
        message: e.to_string(),
    }
}
/// Pixel bytes have a separate bounded binary channel, never a JSON number array.
pub fn render_paths_json(
    input: &str,
    backend: &mut dyn RasterBackend,
    check: &dyn Fn() -> bool,
) -> (String, Vec<u8>) {
    let result = if input.len() > super::MAX_REQUEST_BYTES {
        Err(failure(RasterError::Limit("raster request bytes")))
    } else {
        from_json_str::<PathRasterRequest>(input)
            .map_err(|e| RasterFailure {
                code: RasterFailureCode::InputInvalid,
                message: e.to_string(),
            })
            .and_then(|request| mo_raster::render(&request, backend, check).map_err(failure))
    };
    let (response, pixels) = match result {
        Ok(image) => (
            PathRasterResponse::Rendered {
                info: Box::new(image.info),
            },
            image.pixels,
        ),
        Err(error) => (PathRasterResponse::Error { error }, vec![]),
    };
    (
        serde_json::to_string(&response).expect("bounded raster metadata"),
        pixels,
    )
}
