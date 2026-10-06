//! Shared display-light tone/gamut mapping. Output transfer and alpha policy
//! remain explicit at each consumer; sRGB and television signals are distinct.
use crate::{RenderError, Result, ToneMap};
use mo_color::rgb::{Matrix3, Primaries, REFERENCE_WHITE_NITS};
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct OutputColorStats {
    pub gamut_clipped_pixels: usize,
    pub tone_mapped_pixels: usize,
}
pub(crate) struct SdrTransform {
    white: Option<f64>,
    matrix: Matrix3,
    luma: [f64; 3],
}
impl SdrTransform {
    pub fn new(hdr: bool, tone: Option<ToneMap>) -> Result<Self> {
        if hdr && tone.is_none() {
            return Err(RenderError::Unsupported(
                "explicit HDR to SDR tone map required",
            ));
        }
        let white = match tone {
            None => None,
            Some(ToneMap::ReinhardLuminanceV1 { source_peak_nits }) => {
                if !(REFERENCE_WHITE_NITS..=10000.).contains(&source_peak_nits) {
                    return Err(RenderError::Invalid("tone-map white point"));
                }
                Some(source_peak_nits / REFERENCE_WHITE_NITS)
            }
        };
        Ok(Self {
            white,
            matrix: Primaries::Bt2020.to(Primaries::Bt709),
            luma: Primaries::Bt2020.luminance(),
        })
    }
    pub fn map(&self, mut rgb: [f64; 3], stats: &mut OutputColorStats) -> [f64; 3] {
        if let Some(white) = self.white {
            let y = rgb[0] * self.luma[0] + rgb[1] * self.luma[1] + rgb[2] * self.luma[2];
            if y > 0. {
                let mapped = y * (1. + y / (white * white)) / (1. + y);
                rgb = rgb.map(|v| v * mapped / y);
                stats.tone_mapped_pixels += 1;
            }
        }
        let rgb = self.matrix.apply(rgb);
        stats.gamut_clipped_pixels += usize::from(rgb.iter().any(|v| *v < -1e-6 || *v > 1. + 1e-6));
        rgb
    }
}
