use crate::sdr::{OutputColorStats, SdrTransform};
use crate::{LinearImage, RenderError, Result, cancel};
use mo_color::linear_to_srgb;
#[derive(Debug, Clone, Copy)]
pub enum ToneMap {
    /// Versioned global luminance Reinhard curve with an explicit white point.
    /// This is not BT.2390 or a content-adaptive/perceptual quality claim.
    ReinhardLuminanceV1 { source_peak_nits: f64 },
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutputAlpha {
    Straight,
    PremultipliedEncoded,
}
#[derive(Debug, Clone, Copy)]
pub struct OutputOptions {
    pub tone_map: Option<ToneMap>,
    pub alpha: OutputAlpha,
    pub max_bytes: usize,
}
#[derive(Debug)]
pub struct SrgbImage {
    pub width: u32,
    pub height: u32,
    pub rgba: Vec<u8>,
    pub alpha: OutputAlpha,
    pub gamut_clipped_pixels: usize,
    pub tone_mapped_pixels: usize,
}
impl LinearImage {
    pub fn to_srgb8(&self, options: OutputOptions, check: &dyn Fn() -> bool) -> Result<SrgbImage> {
        cancel(check)?;
        let transform = SdrTransform::new(self.hdr, options.tone_map)?;
        let bytes = self
            .pixels
            .len()
            .checked_mul(4)
            .filter(|n| *n <= options.max_bytes)
            .ok_or(RenderError::Budget)?;
        let mut rgba = Vec::new();
        rgba.try_reserve_exact(bytes)
            .map_err(|_| RenderError::Budget)?;
        let mut stats = OutputColorStats::default();
        for row in self.pixels.chunks(self.width as usize) {
            cancel(check)?;
            for p in row {
                let a = f64::from(p[3]);
                if a == 0. {
                    rgba.extend([0; 4]);
                    continue;
                }
                let rgb = [
                    f64::from(p[0]) / a,
                    f64::from(p[1]) / a,
                    f64::from(p[2]) / a,
                ];
                let rgb = transform.map(rgb, &mut stats);
                let rgb = rgb.map(|v| linear_to_srgb(v).clamp(0., 1.));
                let alpha = unit_byte(a);
                for channel in rgb {
                    rgba.push(if options.alpha == OutputAlpha::Straight {
                        unit_byte(channel)
                    } else {
                        unit_byte(channel * a).min(alpha)
                    });
                }
                rgba.push(alpha);
            }
        }
        cancel(check)?;
        Ok(SrgbImage {
            width: self.width,
            height: self.height,
            rgba,
            alpha: options.alpha,
            gamut_clipped_pixels: stats.gamut_clipped_pixels,
            tone_mapped_pixels: stats.tone_mapped_pixels,
        })
    }
}
fn unit_byte(v: f64) -> u8 {
    libm::floor(v.clamp(0., 1.) * 255. + 0.5) as u8
}
