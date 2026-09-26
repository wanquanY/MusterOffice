//! Image resources travel through the same scene lowering and raster executor.
use crate::{SceneRasterInfo, SceneRasterRequest, SceneWork, compile::compile_with};
use mo_common::Digest;
use mo_raster::{CompiledImageRaster, ImageWork, PreparedImages, RasterBackend, RasterError};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

pub struct CompiledImageScene<'a> {
    raster: CompiledImageRaster<'a>,
    work: SceneWork,
}
impl CompiledImageScene<'_> {
    pub fn raster(&self) -> &CompiledImageRaster<'_> {
        &self.raster
    }
    pub fn work(&self) -> &SceneWork {
        &self.work
    }
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ImageSceneRasterInfo {
    pub scene: SceneRasterInfo,
    /// Image affine error is independent of the path-transform error in scene.
    pub images: ImageWork,
    pub resources_sha256: Digest,
}
pub struct ImageSceneRaster {
    pub info: ImageSceneRasterInfo,
    pub pixels: Vec<u8>,
}
pub fn compile_images<'a>(
    request: &SceneRasterRequest,
    images: &'a PreparedImages<'a>,
    check: &dyn Fn() -> bool,
) -> Result<CompiledImageScene<'a>, RasterError> {
    let (raster, work) = compile_with(request, check, &|paths, check| {
        let raster = mo_raster::compile_images(paths, images, check)?;
        let bound = raster.raster_work().coordinate_error_bound;
        Ok((raster, bound))
    })?;
    Ok(CompiledImageScene { raster, work })
}
pub fn render_images(
    request: &SceneRasterRequest,
    images: &PreparedImages<'_>,
    backend: &mut dyn RasterBackend,
    check: &dyn Fn() -> bool,
) -> Result<ImageSceneRaster, RasterError> {
    render_compiled_images(compile_images(request, images, check)?, backend, check)
}
pub fn render_compiled_images(
    compiled: CompiledImageScene<'_>,
    backend: &mut dyn RasterBackend,
    check: &dyn Fn() -> bool,
) -> Result<ImageSceneRaster, RasterError> {
    let image = mo_raster::render_compiled_images(compiled.raster, backend, check)?;
    Ok(ImageSceneRaster {
        info: ImageSceneRasterInfo {
            scene: SceneRasterInfo {
                profile: crate::PROFILE.into(),
                raster: image.raster.info,
                work: compiled.work,
            },
            images: image.work,
            resources_sha256: image.resources_sha256,
        },
        pixels: image.raster.pixels,
    })
}
