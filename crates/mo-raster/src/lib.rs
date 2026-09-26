//! Domain-independent path-to-pixel computation. Host owns authority and isolation.
mod brush;
mod clip;
#[cfg(test)]
mod clip_tests;
mod compile;
mod composite;
#[cfg(test)]
mod composite_tests;
mod gradient;
mod office_gradient;
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
mod profiles;
mod stroke;
#[cfg(test)]
mod stroke_tests;
#[cfg(test)]
mod tests;
mod types;
pub use brush::*;
pub use compile::{CompiledRaster, compile};
use mo_common::{ByteLength, Digest};
pub use profiles::{accepts_profile, profile_for_frame};
use sha2::{Digest as _, Sha256};
use thiserror::Error;
pub use types::*;

pub const MAX_FRAME_WORDS: usize = 2617354;
pub const MAX_CLIP_FRAME_WORDS: usize = 2789389;
pub const MAX_COMPOSITE_FRAME_WORDS: usize = 2854990;
pub const MAX_GRADIENT_PLANE_FRAME_WORDS: usize = 2883662;
pub const MAX_ELLIPTIC_GRADIENT_FRAME_WORDS: usize = 2895950;
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
pub struct BackendReply {
    pub status: u32,
    pub pixels: Vec<u8>,
}
pub trait RasterBackend {
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
        cancel(check)?;
        if reply.status > 4 || (reply.status != 0 && !reply.pixels.is_empty()) {
            return Err(RasterError::ComponentInvalid("status or failure bytes"));
        }
        if reply.status != 0 {
            return Err(RasterError::Component(reply.status));
        }
        if reply.pixels.len() != compiled.pixel_bytes() {
            return Err(RasterError::ComponentInvalid("pixel length"));
        }
        let mut hash = Sha256::new();
        for chunk in reply.pixels.chunks(16384) {
            cancel(check)?;
            if chunk
                .chunks_exact(4)
                .any(|p| p[..3].iter().any(|v| *v > p[3]))
            {
                return Err(RasterError::ComponentInvalid("premultiplied channels"));
            }
            hash.update(chunk);
        }
        cancel(check)?;
        let mut frame_hash = Sha256::new();
        for words in compiled.frame().chunks(4096) {
            cancel(check)?;
            for word in words {
                frame_hash.update(word.to_le_bytes());
            }
        }
        cancel(check)?;
        Ok(RasterImage {
            info: RasterInfo {
                profile: profile_for_frame(compiled.frame()[1])
                    .ok_or(RasterError::ComponentInvalid("compiled frame version"))?
                    .into(),
                width: compiled.width(),
                height: compiled.height(),
                byte_length: ByteLength::new(reply.pixels.len() as u64),
                sha256: Digest::from_sha256(hash.finalize().into()),
                frame_sha256: Digest::from_sha256(frame_hash.finalize().into()),
                work: compiled.work,
            },
            pixels: reply.pixels,
        })
    })();
    if matches!(
        &result,
        Err(RasterError::Host(_)
            | RasterError::Cancelled
            | RasterError::ComponentInvalid(_)
            | RasterError::Component(2 | 4))
    ) {
        backend.invalidate();
    }
    result
}
