use crate::{RenderError, Result};
use mo_media::{FrameInfo, StreamInfo};
#[derive(Debug, Clone, Copy)]
pub struct Affine {
    m: [f64; 6],
}
impl Affine {
    /// [a,b,c,d,tx,ty]: (x,y) -> (a*x+c*y+tx,b*x+d*y+ty).
    pub fn new(m: [f64; 6]) -> Result<Self> {
        if m.iter().any(|v| !v.is_finite()) {
            return Err(RenderError::Invalid("non-finite transform"));
        }
        Ok(Self { m })
    }
    pub fn identity() -> Self {
        Self {
            m: [1., 0., 0., 1., 0., 0.],
        }
    }
    /// Row-vector coefficients for an explicitly chosen backend precision.
    pub fn coefficients(self) -> [f64; 6] {
        self.m
    }
    pub fn map(self, x: f64, y: f64) -> (f64, f64) {
        let [a, b, c, d, tx, ty] = self.m;
        (a * x + c * y + tx, b * x + d * y + ty)
    }
    /// Apply rhs first, then self.
    pub fn compose(self, rhs: Self) -> Result<Self> {
        let [a, b, c, d, x, y] = self.m;
        let [e, f, g, h, u, v] = rhs.m;
        Self::new([
            a * e + c * f,
            b * e + d * f,
            a * g + c * h,
            b * g + d * h,
            a * u + c * v + x,
            b * u + d * v + y,
        ])
    }
    pub fn inverse(self) -> Result<Self> {
        let [a, b, c, d, x, y] = self.m;
        let det = a * d - b * c;
        if det == 0. || !det.is_finite() {
            return Err(RenderError::Invalid("singular transform"));
        }
        Self::new([
            d / det,
            -b / det,
            -c / det,
            a / det,
            (c * y - d * x) / det,
            (b * x - a * y) / det,
        ])
    }
}
#[derive(Debug, Clone, Copy)]
pub struct SourceGeometry {
    pub pixel_to_display: Affine,
}
impl SourceGeometry {
    pub fn square_pixels() -> Self {
        Self {
            pixel_to_display: Affine::identity(),
        }
    }
    pub fn from_media(frame: &FrameInfo, stream: &StreamInfo) -> Result<Self> {
        let sar = frame
            .sample_aspect_ratio
            .or(stream.sample_aspect_ratio)
            .ok_or(RenderError::Unsupported(
                "explicit pixel aspect ratio required",
            ))?;
        let matrix = stream.display_matrix.map(|m| {
            std::array::from_fn(|i| f64::from(m[i]) / if i % 3 == 2 { 1073741824. } else { 65536. })
        });
        Self::from_display_metadata((sar.numerator, sar.denominator), matrix)
    }
    /// Container row-vector matrix [a,b,u,c,d,v,x,y,w], already decoded from
    /// its fixed-point storage. Pixels are cropped to the visible rectangle but
    /// have NOT been rotated, flipped or rescaled by a platform image API.
    pub fn from_display_metadata(sar: (u32, u32), matrix: Option<[f64; 9]>) -> Result<Self> {
        if sar.0 == 0 || sar.1 == 0 {
            return Err(RenderError::Invalid("pixel aspect ratio"));
        }
        let aspect = Affine::new([f64::from(sar.0) / f64::from(sar.1), 0., 0., 1., 0., 0.])?;
        let display = if let Some(m) = matrix {
            if m.iter().any(|v| !v.is_finite()) {
                return Err(RenderError::Invalid("display matrix"));
            }
            if m[2] != 0. || m[5] != 0. || m[8] == 0. {
                return Err(RenderError::Unsupported("projective display matrix"));
            }
            let w = m[8];
            Affine::new([m[0] / w, m[1] / w, m[3] / w, m[4] / w, m[6] / w, m[7] / w])?
        } else {
            Affine::identity()
        };
        let pixel_to_display = display.compose(aspect)?;
        pixel_to_display.inverse()?;
        Ok(Self { pixel_to_display })
    }
    /// Preserve display rotation/SAR, fit within canvas, and center. Author
    /// transforms apply after this baseline; no source pixels are rewritten.
    pub fn contain(self, source: (u32, u32), canvas: (u32, u32)) -> Result<Affine> {
        if source.0 == 0 || source.1 == 0 || canvas.0 == 0 || canvas.1 == 0 {
            return Err(RenderError::Invalid("placement extent"));
        }
        let (w, h) = (f64::from(source.0), f64::from(source.1));
        let corners =
            [(0., 0.), (w, 0.), (0., h), (w, h)].map(|(x, y)| self.pixel_to_display.map(x, y));
        let left = corners.iter().map(|p| p.0).fold(f64::INFINITY, f64::min);
        let top = corners.iter().map(|p| p.1).fold(f64::INFINITY, f64::min);
        let right = corners
            .iter()
            .map(|p| p.0)
            .fold(f64::NEG_INFINITY, f64::max);
        let bottom = corners
            .iter()
            .map(|p| p.1)
            .fold(f64::NEG_INFINITY, f64::max);
        let scale =
            (f64::from(canvas.0) / (right - left)).min(f64::from(canvas.1) / (bottom - top));
        Affine::new([
            scale,
            0.,
            0.,
            scale,
            (f64::from(canvas.0) - (right - left) * scale) / 2. - left * scale,
            (f64::from(canvas.1) - (bottom - top) * scale) / 2. - top * scale,
        ])?
        .compose(self.pixel_to_display)
    }
}
pub(crate) fn author_transform(
    canvas: (u32, u32),
    translation: (i32, i32),
    scale: (i32, i32),
    angle: i32,
) -> Result<Affine> {
    let angle = angle.rem_euclid(360000);
    let (sin, cos) = match angle {
        0 => (0., 1.),
        90000 => (1., 0.),
        180000 => (0., -1.),
        270000 => (-1., 0.),
        _ => {
            let r = f64::from(angle) * std::f64::consts::PI / 180000.;
            (libm::sin(r), libm::cos(r))
        }
    };
    let sx = f64::from(scale.0) / 1e6;
    let sy = f64::from(scale.1) / 1e6;
    let a = cos * sx;
    let b = sin * sx;
    let c = -sin * sy;
    let d = cos * sy;
    let cx = f64::from(canvas.0) / 2.;
    let cy = f64::from(canvas.1) / 2.;
    Affine::new([
        a,
        b,
        c,
        d,
        cx + f64::from(translation.0) / 1000. - a * cx - c * cy,
        cy + f64::from(translation.1) / 1000. - b * cx - d * cy,
    ])
}
