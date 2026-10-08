//! Bounded SDR television output: explicit matte, inverse ideal BT.1886,
//! BT.709 YCbCr, limited range and left-cosited 4:2:0 with antialias filtering.
use crate::{
    LinearImage, OutputColorStats, RenderError, Result, ToneMap, cancel, sdr::SdrTransform,
};
use mo_color::{rgb::Primaries, srgb_to_linear};

pub const I420_PROFILE: &str = "musteroffice.video.bt709-bt1886-limited-left-lanczos2-v1-draft";
#[derive(Debug, Clone, Copy)]
pub struct I420Options {
    /// An explicitly chosen opaque sRGB matte. Alpha is composited in linear light.
    pub background_srgb: [u8; 3],
    pub tone_map: Option<ToneMap>,
    /// Total owned Y/U/V output bytes, independent of input and scratch memory.
    pub max_bytes: usize,
    pub max_temporary_bytes: usize,
}
#[derive(Debug)]
pub struct I420Image {
    width: u32,
    height: u32,
    planes: [Vec<u8>; 3],
    pub colors: OutputColorStats,
    pub chroma_clipped_samples: usize,
    pub scratch_bytes: usize,
}
impl I420Image {
    pub fn width(&self) -> u32 {
        self.width
    }
    pub fn height(&self) -> u32 {
        self.height
    }
    pub fn planes(&self) -> [&[u8]; 3] {
        [&self.planes[0], &self.planes[1], &self.planes[2]]
    }
    pub fn strides(&self) -> [usize; 3] {
        [
            self.width as usize,
            self.width as usize / 2,
            self.width as usize / 2,
        ]
    }
    pub fn byte_length(&self) -> usize {
        self.planes.iter().map(Vec::len).sum()
    }
}
fn zeros<T: Clone + Default>(count: usize) -> Result<Vec<T>> {
    let mut result = Vec::new();
    result
        .try_reserve_exact(count)
        .map_err(|_| RenderError::Budget)?;
    result.resize(count, T::default());
    Ok(result)
}
fn byte(value: f64, low: f64, high: f64) -> u8 {
    libm::floor(value.clamp(low, high) + 0.5) as u8
}
fn sinc(x: f64) -> f64 {
    if x == 0. {
        1.
    } else {
        let t = std::f64::consts::PI * x;
        libm::sin(t) / t
    }
}
/// Downsampling by 2: radius-2 Lanczos on the output lattice (radius 4 input
/// pixels). Horizontal phase 0 is left-cosited; vertical phase 1/2 is centered.
fn weights(phase: f64) -> [f64; 8] {
    let mut values = std::array::from_fn(|i| {
        let d = i as f64 - 3. - phase;
        if d.abs() >= 4. {
            0.
        } else {
            sinc(d / 2.) * sinc(d / 4.)
        }
    });
    let total: f64 = values.iter().sum();
    for v in &mut values {
        *v /= total;
    }
    values
}
/// Shared horizontal (left-cosited) and vertical (centered) antialias kernels
/// for CPU and GPU export. Executors transport these coefficients unchanged.
pub fn i420_chroma_weights() -> ([f64; 8], [f64; 8]) {
    (weights(0.), weights(0.5))
}
impl LinearImage {
    pub fn to_i420(&self, options: I420Options, check: &dyn Fn() -> bool) -> Result<I420Image> {
        cancel(check)?;
        let (w, h) = (self.width as usize, self.height as usize);
        if !w.is_multiple_of(2) || !h.is_multiple_of(2) {
            return Err(RenderError::Unsupported(
                "I420 requires even dimensions; resize or pad explicitly",
            ));
        }
        let transform = SdrTransform::new(self.hdr, options.tone_map)?;
        let count = w.checked_mul(h).ok_or(RenderError::Budget)?;
        if count
            .checked_add(count / 2)
            .is_none_or(|n| n > options.max_bytes)
        {
            return Err(RenderError::Budget);
        }
        // One full-width row and eight half-width filtered rows, each Cb+Cr f64.
        let scratch_bytes = w.checked_mul(16 + 8 * 8).ok_or(RenderError::Budget)?;
        if scratch_bytes > options.max_temporary_bytes {
            return Err(RenderError::Budget);
        }
        let mut out = I420Image {
            width: self.width,
            height: self.height,
            planes: [zeros(count)?, zeros(count / 4)?, zeros(count / 4)?],
            colors: OutputColorStats::default(),
            chroma_clipped_samples: 0,
            scratch_bytes,
        };
        let mut row: Vec<[f64; 2]> = zeros(w)?;
        let mut ring: Vec<[f64; 2]> = zeros(8 * (w / 2))?;
        let mut tags = [usize::MAX; 8];
        let bg = Primaries::Bt709.to(Primaries::Bt2020).apply(
            options
                .background_srgb
                .map(|v| srgb_to_linear(f64::from(v) / 255.)),
        );
        let (horizontal, vertical) = i420_chroma_weights();
        for cy in 0..h / 2 {
            cancel(check)?;
            let rows: [usize; 8] =
                std::array::from_fn(|i| (2 * cy + i).saturating_sub(3).min(h - 1));
            for y in rows {
                if tags[y % 8] == y {
                    continue;
                }
                cancel(check)?;
                for (x, uv) in row.iter_mut().enumerate() {
                    let p = self.pixels[y * w + x];
                    let rgb =
                        std::array::from_fn(|i| f64::from(p[i]) + (1. - f64::from(p[3])) * bg[i]);
                    let rgb = transform
                        .map(rgb, &mut out.colors)
                        .map(|v| libm::pow(v.clamp(0., 1.), 1. / 2.4));
                    let luma = 0.2126 * rgb[0] + 0.7152 * rgb[1] + 0.0722 * rgb[2];
                    out.planes[0][y * w + x] = byte(16. + 219. * luma, 16., 235.);
                    *uv = [
                        (rgb[2] - luma) / (2. * (1. - 0.0722)),
                        (rgb[0] - luma) / (2. * (1. - 0.2126)),
                    ];
                }
                for cx in 0..w / 2 {
                    let mut uv = [0.; 2];
                    for (i, weight) in horizontal.iter().enumerate() {
                        let x = (2 * cx + i).saturating_sub(3).min(w - 1);
                        for ch in 0..2 {
                            uv[ch] += row[x][ch] * weight;
                        }
                    }
                    ring[(y % 8) * (w / 2) + cx] = uv;
                }
                tags[y % 8] = y;
            }
            for cx in 0..w / 2 {
                let mut uv = [0.; 2];
                for (i, y) in rows.iter().enumerate() {
                    for ch in 0..2 {
                        uv[ch] += ring[(y % 8) * (w / 2) + cx][ch] * vertical[i];
                    }
                }
                for (ch, value) in uv.iter().enumerate() {
                    out.chroma_clipped_samples +=
                        usize::from(*value < -0.5 - 1e-6 || *value > 0.5 + 1e-6);
                    out.planes[ch + 1][cy * (w / 2) + cx] = byte(128. + 224. * value, 16., 240.);
                }
            }
        }
        cancel(check)?;
        Ok(out)
    }
}
