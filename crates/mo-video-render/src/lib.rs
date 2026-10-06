//! Video-only linear-light reference compositor. No native codec/GPU/host linkage.
mod backend;
mod color;
mod composite;
mod coordinates;
mod cpu;
mod frame;
mod geometry;
mod layout;
mod output;
mod sdr;
mod yuv_output;
pub use backend::*;
pub use color::*;
pub use composite::*;
pub use coordinates::RasterCoordinates;
pub use frame::{NormalizedFrame, normalize_frame};
pub use geometry::*;
pub use output::*;
pub use sdr::OutputColorStats;
pub use yuv_output::*;

pub const PROFILE: &str = "musteroffice.video.linear2020-d65-203nits-premul-cpu-v2-draft";
#[derive(Debug, thiserror::Error)]
pub enum RenderError {
    #[error("invalid video render input: {0}")]
    Invalid(&'static str),
    #[error("unsupported video render capability: {0}")]
    Unsupported(&'static str),
    #[error("video render budget exceeded")]
    Budget,
    #[error("video render cancelled")]
    Cancelled,
    #[error(transparent)]
    Color(#[from] mo_color::rgb::RgbError),
    #[error(transparent)]
    Time(#[from] mo_time::TimeError),
    #[error("video render resource: {0}")]
    Resource(String),
}
pub type Result<T> = std::result::Result<T, RenderError>;
#[derive(Debug, Clone, Copy)]
pub struct PixelLimits {
    pub max_pixels: usize,
    pub max_bytes: usize,
}
impl Default for PixelLimits {
    fn default() -> Self {
        Self {
            max_pixels: 16_777_216,
            max_bytes: 256 << 20,
        }
    }
}
/// Owned linear BT.2020 D65 RGBA32F, relative to 203 nits, premultiplied alpha.
/// Components may be negative/out-of-gamut/HDR; they are not clamped until output.
#[derive(Debug)]
pub struct LinearImage {
    width: u32,
    height: u32,
    pixels: Vec<[f32; 4]>,
    hdr: bool,
}
impl LinearImage {
    pub fn width(&self) -> u32 {
        self.width
    }
    pub fn height(&self) -> u32 {
        self.height
    }
    pub fn pixels(&self) -> &[[f32; 4]] {
        &self.pixels
    }
    pub fn has_hdr(&self) -> bool {
        self.hdr
    }
    pub fn from_pixels(
        width: u32,
        height: u32,
        pixels: Vec<[f32; 4]>,
        hdr: bool,
        limits: PixelLimits,
        check: &dyn Fn() -> bool,
    ) -> Result<Self> {
        if pixel_count(width, height, limits)? != pixels.len() {
            return Err(RenderError::Invalid("linear pixel length"));
        }
        for row in pixels.chunks(4096) {
            cancel(check)?;
            if row.iter().any(|p| {
                p.iter().any(|v| !v.is_finite())
                    || !(0.0..=1.0).contains(&p[3])
                    || (p[3] == 0. && p[..3] != [0.; 3])
            }) {
                return Err(RenderError::Invalid("linear pixels or alpha"));
            }
        }
        Ok(Self {
            width,
            height,
            pixels,
            hdr,
        })
    }
    pub(crate) fn blank(
        width: u32,
        height: u32,
        limits: PixelLimits,
        check: &dyn Fn() -> bool,
    ) -> Result<Self> {
        cancel(check)?;
        let count = pixel_count(width, height, limits)?;
        let mut pixels = Vec::new();
        pixels
            .try_reserve_exact(count)
            .map_err(|_| RenderError::Budget)?;
        pixels.resize(count, [0.; 4]);
        cancel(check)?;
        Ok(Self {
            width,
            height,
            pixels,
            hdr: false,
        })
    }
}
pub(crate) fn pixel_count(w: u32, h: u32, limits: PixelLimits) -> Result<usize> {
    let n = u64::from(w) * u64::from(h);
    if w == 0
        || h == 0
        || w > 16384
        || h > 16384
        || n > limits.max_pixels as u64
        || n * 16 > limits.max_bytes as u64
        || n * 16 > 1 << 30
    {
        return Err(RenderError::Budget);
    }
    usize::try_from(n).map_err(|_| RenderError::Budget)
}
pub(crate) fn cancel(check: &dyn Fn() -> bool) -> Result<()> {
    if check() {
        Err(RenderError::Cancelled)
    } else {
        Ok(())
    }
}
