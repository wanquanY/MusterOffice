//! Domain-independent path-to-pixel computation. Host owns authority and isolation.
mod brush;
mod clip;
#[cfg(test)]
mod clip_tests;
mod compile;
mod completion;
mod reply_validation;
pub use reply_validation::{
    RasterCompletionReply, RasterReplyValidation, VALIDATION_BYTES_PER_UNIT, ValidatedRasterReply,
};
#[cfg(test)]
mod completion_tests;
mod composite;
#[cfg(test)]
mod composite_tests;
mod gradient;
mod office_gradient;
mod opacity;
#[cfg(test)]
mod opacity_tests;
pub use office_gradient::office_gamma_eligible;
mod gradient_stops;
pub use gradient_stops::GradientStops;
mod paint_budget;
pub use paint_budget::{MAX_DRAWS, MAX_GRADIENT_INPUT_STOPS, PaintBudget};
mod gradient_elliptic;
#[cfg(test)]
mod gradient_elliptic_tests;
mod gradient_plane;
#[cfg(test)]
mod gradient_plane_tests;
mod gradient_rate;
#[cfg(test)]
mod gradient_rect_tests;
mod paint_matrix;
mod paint_precision;
pub use gradient_plane::{
    GradientAxisTile, GradientField, GradientPlane, GradientPlaneUncertainty,
};
mod image;
#[cfg(test)]
mod image_tests;
pub use image::*;
#[cfg(test)]
mod gradient_tests;
mod number;
pub mod picking;
mod profiles;
mod stroke;
#[cfg(test)]
mod stroke_tests;
#[cfg(test)]
mod tests;
mod types;
pub use brush::*;
pub use compile::{CompiledRaster, compile};
pub use profiles::{accepts_profile, profile_for_frame};
use thiserror::Error;
pub use types::*;

pub const MAX_FRAME_WORDS: usize = 2617354;
pub const MAX_CLIP_FRAME_WORDS: usize = 2789389;
pub const MAX_COMPOSITE_FRAME_WORDS: usize = 2854990;
pub const MAX_GRADIENT_PLANE_FRAME_WORDS: usize = 2883662;
pub const MAX_ELLIPTIC_GRADIENT_FRAME_WORDS: usize = 2895950;
pub const MAX_OPACITY_GROUP_FRAME_WORDS: usize = MAX_ELLIPTIC_GRADIENT_FRAME_WORDS + 1 + 4096 * 3;
pub const MAX_SNAPSHOT_SCOPE_FRAME_WORDS: usize = MAX_OPACITY_GROUP_FRAME_WORDS + 64;
pub const SNAPSHOT_SCOPE_PROFILE: &str =
    "skia-8d6d37b-scoped-snapshots-srgb-premul-rgba8-v14-draft";
pub const OPACITY_GROUP_PROFILE: &str = "skia-8d6d37b-isolated-opacity-srgb-premul-rgba8-v13-draft";
pub const ELLIPTIC_GRADIENT_PROFILE: &str =
    "skia-8d6d37b-elliptic-gradient-fields-srgb-premul-rgba8-v12-draft";
pub const MAX_RECT_GRADIENT_FRAME_WORDS: usize = 2887758;
pub const RECT_GRADIENT_PROFILE: &str =
    "skia-8d6d37b-rectangular-gradient-fields-srgb-premul-rgba8-v11-draft";
pub const OFFICE_GRADIENT_PROFILE: &str =
    "skia-8d6d37b-office-gamma1875-srgb-premul-rgba8-v10-draft";
pub const GRADIENT_PLANE_PROFILE: &str =
    "skia-8d6d37b-mapped-gradient-fields-srgb-premul-rgba8-v9-draft";
pub const COMPOSITE_PROFILE: &str = "skia-8d6d37b-prefix-snapshots-srgb-premul-rgba8-v8-draft";
pub const CLIP_PROFILE: &str = "skia-8d6d37b-shared-path-clips-srgb-premul-rgba8-v7-draft";
pub const MAX_PIXEL_BYTES: usize = 64 * 1024 * 1024;
pub const PROFILE: &str = "skia-8d6d37b-q32-world-brushes-srgb-premul-rgba8-v4-draft";
#[derive(Debug, Error)]
pub enum RasterError {
    #[error("invalid raster request: {0}")]
    Invalid(&'static str),
    #[error("raster limit: {0}")]
    Limit(&'static str),
    #[error("raster coordinate exceeds supported range")]
    Range,
    #[error("raster coordinate precision budget exceeded")]
    Precision,
    #[error("raster cancelled")]
    Cancelled,
    #[error("raster component status {0}")]
    Component(u32),
    #[error("invalid raster component reply: {0}")]
    ComponentInvalid(&'static str),
    #[error("raster host failure: {0}")]
    Host(&'static str),
}
impl RasterError {
    /// A failed/ambiguous component invocation cannot be reused. A caller that
    /// cancels before invoking the component can simply drop the compiled batch.
    pub fn invalidates_backend(&self) -> bool {
        matches!(
            self,
            Self::Host(_) | Self::Cancelled | Self::ComponentInvalid(_) | Self::Component(2 | 4)
        )
    }
}
pub struct BackendReply {
    pub status: u32,
    pub pixels: Vec<u8>,
}
impl BackendReply {
    /// Cheap status validation is also useful when discarding a stale frame:
    /// observed component faults must not be hidden by an owner conflict.
    pub fn check_status(&self) -> Result<(), RasterError> {
        if self.status > 4 || (self.status != 0 && !self.pixels.is_empty()) {
            return Err(RasterError::ComponentInvalid("status or failure bytes"));
        }
        if self.status != 0 {
            return Err(RasterError::Component(self.status));
        }
        Ok(())
    }
}
pub trait RasterBackend {
    /// Optional pure geometry extension using the renderer's device paths.
    fn pick(&mut self, _frame: &[u32]) -> Result<picking::PickingReply, RasterError> {
        Err(RasterError::Host("raster picking extension unavailable"))
    }
    fn raster(&mut self, frame: &[u32]) -> Result<BackendReply, RasterError>;
    /// Optional extension. Hosts must provide the image-capable backend when
    /// selecting image rendering; no rasterization/quality fallback is allowed.
    fn raster_images(
        &mut self,
        _frame: &[u32],
        _images: &[u8],
    ) -> Result<BackendReply, RasterError> {
        Err(RasterError::Host("raster image extension unavailable"))
    }
    fn invalidate(&mut self);
}
pub(crate) fn cancel(check: &dyn Fn() -> bool) -> Result<(), RasterError> {
    if check() {
        Err(RasterError::Cancelled)
    } else {
        Ok(())
    }
}
pub fn render(
    request: &PathRasterRequest,
    backend: &mut dyn RasterBackend,
    check: &dyn Fn() -> bool,
) -> Result<RasterImage, RasterError> {
    let compiled = compile(request, check)?;
    render_compiled(compiled, backend, check)
}
/// Execute a verified batch without recompiling its geometry. The constructor
/// and all batch storage are private to this crate; hosts cannot forge it.
pub fn render_compiled(
    compiled: CompiledRaster,
    backend: &mut dyn RasterBackend,
    check: &dyn Fn() -> bool,
) -> Result<RasterImage, RasterError> {
    execute(compiled, backend, None, check)
}
fn execute(
    compiled: CompiledRaster,
    backend: &mut dyn RasterBackend,
    images: Option<&PreparedImages<'_>>,
    check: &dyn Fn() -> bool,
) -> Result<RasterImage, RasterError> {
    cancel(check)?;
    let result = (|| {
        let reply = match images {
            Some(images) => backend.raster_images(compiled.frame(), images.bytes())?,
            None => backend.raster(compiled.frame())?,
        };
        compiled.complete(reply, check)
    })();
    if result
        .as_ref()
        .is_err_and(|error| error.invalidates_backend())
    {
        backend.invalidate();
    }
    result
}
