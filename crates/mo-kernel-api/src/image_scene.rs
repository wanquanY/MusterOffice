use crate::raster::{RasterFailure, RasterFailureCode, failure};
use mo_common::from_json_str;
use mo_raster::{ImageResource, PreparedImages, RasterBackend, RasterError};
pub use mo_render::ImageSceneRasterInfo;
use mo_render::SceneRasterRequest;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ImageSceneRasterRequest {
    pub raster: SceneRasterRequest,
    pub images: Vec<ImageResource>,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "status", rename_all = "camelCase", deny_unknown_fields)]
pub enum ImageSceneRasterResponse {
    Rendered { info: Box<ImageSceneRasterInfo> },
    Error { error: RasterFailure },
}
pub fn render_image_scene_json(
    input: &str,
    bytes: &[u8],
    backend: &mut dyn RasterBackend,
    check: &dyn Fn() -> bool,
) -> (String, Vec<u8>) {
    let result =
        if input.len() > crate::MAX_REQUEST_BYTES || bytes.len() > mo_raster::MAX_PIXEL_BYTES {
            Err(failure(RasterError::Limit("image scene request bytes")))
        } else {
            from_json_str::<ImageSceneRasterRequest>(input)
                .map_err(|e| RasterFailure {
                    code: RasterFailureCode::InputInvalid,
                    message: e.to_string(),
                })
                .and_then(|q| {
                    let images = PreparedImages::new(&q.images, bytes, check).map_err(failure)?;
                    mo_render::render_images(&q.raster, &images, backend, check).map_err(failure)
                })
        };
    let (response, pixels) = match result {
        Ok(image) => (
            ImageSceneRasterResponse::Rendered {
                info: Box::new(image.info),
            },
            image.pixels,
        ),
        Err(error) => (ImageSceneRasterResponse::Error { error }, vec![]),
    };
    (
        serde_json::to_string(&response).expect("bounded image scene metadata"),
        pixels,
    )
}
