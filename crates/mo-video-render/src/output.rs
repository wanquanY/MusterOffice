use crate::{LinearImage, RenderError, Result, cancel};
use mo_color::{
    linear_to_srgb,
    rgb::{Primaries, REFERENCE_WHITE_NITS},
};
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
        if self.hdr && options.tone_map.is_none() {
            return Err(RenderError::Unsupported(
                "explicit HDR to SDR tone map required",
            ));
        }
        let white = match options.tone_map {
            None => None,
            Some(ToneMap::ReinhardLuminanceV1 { source_peak_nits }) => {
                if !(REFERENCE_WHITE_NITS..=10000.).contains(&source_peak_nits) {
                    return Err(RenderError::Invalid("tone-map white point"));
                }
                Some(source_peak_nits / REFERENCE_WHITE_NITS)
            }
        };
        let bytes = self
            .pixels
            .len()
            .checked_mul(4)
            .filter(|n| *n <= options.max_bytes)
            .ok_or(RenderError::Budget)?;
        let mut rgba = Vec::new();
        rgba.try_reserve_exact(bytes)
            .map_err(|_| RenderError::Budget)?;
        let matrix = Primaries::Bt2020.to(Primaries::Bt709);
        let luma = Primaries::Bt2020.luminance();
        let mut gamut_clipped_pixels = 0;
        let mut tone_mapped_pixels = 0;
        for row in self.pixels.chunks(self.width as usize) {
            cancel(check)?;
            for p in row {
                let a = f64::from(p[3]);
                if a == 0. {
                    rgba.extend([0; 4]);
                    continue;
                }
                let mut rgb = [
                    f64::from(p[0]) / a,
                    f64::from(p[1]) / a,
                    f64::from(p[2]) / a,
                ];
                if let Some(white) = white {
                    let y = rgb[0] * luma[0] + rgb[1] * luma[1] + rgb[2] * luma[2];
                    if y > 0. {
                        let mapped = y * (1. + y / (white * white)) / (1. + y);
                        rgb = rgb.map(|v| v * mapped / y);
                        tone_mapped_pixels += 1;
                    }
                }
                let rgb = matrix.apply(rgb);
                // Count appreciable clipping; floating roundoff at exact black/white is not a gamut excursion.
                gamut_clipped_pixels +=
                    usize::from(rgb.iter().any(|v| *v < -1e-6 || *v > 1. + 1e-6));
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
            gamut_clipped_pixels,
            tone_mapped_pixels,
        })
    }
}
fn unit_byte(v: f64) -> u8 {
    libm::floor(v.clamp(0., 1.) * 255. + 0.5) as u8
}
