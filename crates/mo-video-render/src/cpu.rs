use crate::backend::{CompositionBackend, Paint, PaintSource};
use crate::coordinates::UNIT;
use crate::{LinearImage, PixelLimits, RenderError, Result, Sampling, cancel};
pub(crate) struct CpuBackend;
impl CompositionBackend for CpuBackend {
    type Surface = LinearImage;
    type Image = std::sync::Arc<LinearImage>;
    fn create_surface(
        &mut self,
        size: (u32, u32),
        limits: PixelLimits,
        check: &dyn Fn() -> bool,
    ) -> Result<LinearImage> {
        LinearImage::blank(size.0, size.1, limits, check)
    }
    fn paint(
        &mut self,
        out: &mut LinearImage,
        paint: Paint<'_, LinearImage>,
        check: &dyn Fn() -> bool,
    ) -> Result<()> {
        let (source, solid) = match paint.source {
            PaintSource::Solid(color) => (None, color),
            PaintSource::Image(image) => (Some(image.as_ref()), [0.; 4]),
            PaintSource::Surface(image) => (Some(image), [0.; 4]),
        };
        let (w, h) = paint.source_size;
        for y in 0..out.height as usize {
            cancel(check)?;
            for x in 0..out.width as usize {
                let (sx, sy) = paint.coordinates.position(x as u32, y as u32);
                if sx < 0 || sy < 0 || sx >= i64::from(w) * UNIT || sy >= i64::from(h) * UNIT {
                    continue;
                }
                let p = source.map_or(solid, |s| sample(s, sx, sy, paint.sampling));
                let src = p.map(|v| f64::from(v) * paint.opacity);
                let dst = &mut out.pixels[y * out.width as usize + x];
                let inv = 1. - src[3];
                for i in 0..4 {
                    dst[i] = (src[i] + f64::from(dst[i]) * inv) as f32;
                }
                if dst.iter().any(|v| !v.is_finite()) {
                    return Err(RenderError::Invalid("composition overflow"));
                }
            }
        }
        Ok(())
    }
}

fn sample(s: &LinearImage, x: i64, y: i64, sampling: Sampling) -> [f32; 4] {
    let at = |x: usize, y: usize| s.pixels[y * s.width as usize + x];
    if matches!(sampling, Sampling::Nearest) {
        return at((x / UNIT) as usize, (y / UNIT) as usize);
    }
    let x = (x - UNIT / 2).clamp(0, i64::from(s.width - 1) * UNIT);
    let y = (y - UNIT / 2).clamp(0, i64::from(s.height - 1) * UNIT);
    let x0 = (x / UNIT) as usize;
    let y0 = (y / UNIT) as usize;
    let x1 = (x0 + 1).min(s.width as usize - 1);
    let y1 = (y0 + 1).min(s.height as usize - 1);
    let fx = (x % UNIT) as f64 / UNIT as f64;
    let fy = (y % UNIT) as f64 / UNIT as f64;
    let a = at(x0, y0);
    let b = at(x1, y0);
    let c = at(x0, y1);
    let d = at(x1, y1);
    std::array::from_fn(|i| {
        ((f64::from(a[i]) * (1. - fx) + f64::from(b[i]) * fx) * (1. - fy)
            + (f64::from(c[i]) * (1. - fx) + f64::from(d[i]) * fx) * fy) as f32
    })
}
