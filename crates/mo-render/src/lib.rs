//! Evaluated Draw IR and certified lowering to the shared CPU raster backend.
//! Authoring semantics and host authority remain outside this crate.
#[cfg(test)]
mod clip_tests;
mod compile;
mod image;
pub use image::*;
#[cfg(test)]
mod image_tests;
#[cfg(test)]
mod tests;
mod types;
pub use compile::{CompiledScene, compile};
use mo_raster::{RasterBackend, RasterError};
pub use types::*;
pub const PROFILE: &str = "affine-q32-shared-paths-certified-cpu-v1-draft";
pub struct SceneImage {
    pub info: SceneRasterInfo,
    pub pixels: Vec<u8>,
}
impl CompiledScene {
    /// Complete the exact prepared scene without rerunning geometry lowering.
    pub fn complete(
        self,
        reply: mo_raster::BackendReply,
        check: &dyn Fn() -> bool,
    ) -> Result<SceneImage, RasterError> {
        let image = self.raster.complete(reply, check)?;
        Ok(scene_image(image, self.work))
    }
}
fn scene_image(image: mo_raster::RasterImage, work: SceneWork) -> SceneImage {
    SceneImage {
        info: SceneRasterInfo {
            profile: PROFILE.into(),
            raster: image.info,
            work,
        },
        pixels: image.pixels,
    }
}
pub fn render(
    request: &SceneRasterRequest,
    backend: &mut dyn RasterBackend,
    check: &dyn Fn() -> bool,
) -> Result<SceneImage, RasterError> {
    let compiled = compile(request, check)?;
    render_compiled(compiled, backend, check)
}
/// Execute a previously verified scene. Batch storage remains privately owned.
pub fn render_compiled(
    compiled: CompiledScene,
    backend: &mut dyn RasterBackend,
    check: &dyn Fn() -> bool,
) -> Result<SceneImage, RasterError> {
    let image = mo_raster::render_compiled(compiled.raster, backend, check)?;
    Ok(scene_image(image, compiled.work))
}
fn cancel(check: &dyn Fn() -> bool) -> Result<(), RasterError> {
    if check() {
        Err(RasterError::Cancelled)
    } else {
        Ok(())
    }
}
