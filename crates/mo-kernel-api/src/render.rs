use crate::raster::{RasterFailure, RasterFailureCode, failure};
use mo_common::from_json_str;
use mo_raster::{RasterBackend, RasterError};
pub use mo_render::PROFILE as SCENE_RASTER_PROFILE;
pub use mo_render::{SceneRasterInfo, SceneRasterRequest};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "status", rename_all = "camelCase", deny_unknown_fields)]
pub enum SceneRasterResponse {
    Rendered { info: Box<SceneRasterInfo> },
    Error { error: RasterFailure },
}
pub fn render_scene_json(
    input: &str,
    backend: &mut dyn RasterBackend,
    check: &dyn Fn() -> bool,
) -> (String, Vec<u8>) {
    let result = if input.len() > super::MAX_REQUEST_BYTES {
        Err(failure(RasterError::Limit("scene request bytes")))
    } else {
        from_json_str::<SceneRasterRequest>(input)
            .map_err(|e| RasterFailure {
                code: RasterFailureCode::InputInvalid,
                message: e.to_string(),
            })
            .and_then(|request| mo_render::render(&request, backend, check).map_err(failure))
    };
    let (response, pixels) = match result {
        Ok(image) => (
            SceneRasterResponse::Rendered {
                info: Box::new(image.info),
            },
            image.pixels,
        ),
        Err(error) => (SceneRasterResponse::Error { error }, vec![]),
    };
    (
        serde_json::to_string(&response).expect("bounded scene metadata"),
        pixels,
    )
}
