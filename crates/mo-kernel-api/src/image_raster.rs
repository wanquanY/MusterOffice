use crate::raster::{RasterFailure, RasterFailureCode, failure};
use mo_common::{Digest, from_json_str};
use mo_raster::{
    ImageResource, ImageWork, PathRasterRequest, PreparedImages, RasterBackend, RasterError,
    RasterInfo,
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ImageRasterRequest {
    pub raster: PathRasterRequest,
    /// Contiguous packed planes in this order, supplied separately as bytes.
    pub images: Vec<ImageResource>,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ImageRasterInfo {
    pub raster: RasterInfo,
    pub images: ImageWork,
    pub resources_sha256: Digest,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "status", rename_all = "camelCase", deny_unknown_fields)]
pub enum ImageRasterResponse {
    Rendered { info: Box<ImageRasterInfo> },
    Error { error: RasterFailure },
}
pub fn render_image_paths_json(
    input: &str,
    bytes: &[u8],
    backend: &mut dyn RasterBackend,
    check: &dyn Fn() -> bool,
) -> (String, Vec<u8>) {
    let result =
        if input.len() > crate::MAX_REQUEST_BYTES || bytes.len() > mo_raster::MAX_PIXEL_BYTES {
            Err(failure(RasterError::Limit("image request bytes")))
        } else {
            from_json_str::<ImageRasterRequest>(input)
                .map_err(|e| RasterFailure {
                    code: RasterFailureCode::InputInvalid,
                    message: e.to_string(),
                })
                .and_then(|q| {
                    let images = PreparedImages::new(&q.images, bytes, check).map_err(failure)?;
                    mo_raster::render_images(&q.raster, &images, backend, check).map_err(failure)
                })
        };
    let (response, pixels) = match result {
        Ok(image) => (
            ImageRasterResponse::Rendered {
                info: Box::new(ImageRasterInfo {
                    raster: image.raster.info,
                    images: image.work,
                    resources_sha256: image.resources_sha256,
                }),
            },
            image.raster.pixels,
        ),
        Err(error) => (ImageRasterResponse::Error { error }, vec![]),
    };
    (
        serde_json::to_string(&response).expect("bounded image raster metadata"),
        pixels,
    )
}
