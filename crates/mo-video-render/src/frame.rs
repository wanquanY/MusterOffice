use crate::layout::{Layout, Packing};
use crate::{
    Alpha, InputColor, LinearImage, Matrix, PixelLimits, RenderError, Result, SignalRange, cancel,
};
use mo_color::rgb::{Primaries, RgbTransform};
use mo_media::{DecodedFrame, MediaKind};
#[derive(Debug)]
pub struct NormalizedFrame {
    pub image: LinearImage,
    pub clipped_signal_pixels: usize,
}
pub fn normalize_frame(
    frame: &DecodedFrame,
    color: InputColor,
    limits: PixelLimits,
    check: &dyn Fn() -> bool,
) -> Result<NormalizedFrame> {
    cancel(check)?;
    if frame.info.kind != MediaKind::Video {
        return Err(RenderError::Invalid("video frame required"));
    }
    if frame.info.interlaced {
        return Err(RenderError::Unsupported(
            "deinterlace before progressive compositing",
        ));
    }
    let w = frame
        .info
        .width
        .ok_or(RenderError::Invalid("frame width"))?;
    let h = frame
        .info
        .height
        .ok_or(RenderError::Invalid("frame height"))?;
    crate::pixel_count(w, h, limits)?;
    let layout = Layout::parse(&frame.info.format)?;
    layout.validate(frame, w, h, check)?;
    if layout.has_alpha() == matches!(color.alpha, Alpha::Opaque) {
        return Err(RenderError::Invalid("alpha/layout mismatch"));
    }
    match layout.packing {
        Packing::Yuv { sx, sy, .. } => {
            color.matrix.coefficients()?;
            if (sx > 1 || sy > 1) && color.chroma.is_none() {
                return Err(RenderError::Unsupported(
                    "subsampled chroma location required",
                ));
            }
        }
        _ => {
            if color.matrix != Matrix::Rgb {
                return Err(RenderError::Invalid("RGB pixels cannot apply a YUV matrix"));
            }
        }
    }
    let transform = RgbTransform::new(color.encoding, Primaries::Bt2020)?;
    let mut image = LinearImage::blank(w, h, limits, check)?;
    image.hdr = transform.is_hdr();
    let mut clipped = 0;
    let max = f64::from((1u32 << layout.bits) - 1);
    for y in 0..h as usize {
        cancel(check)?;
        for x in 0..w as usize {
            let (mut rgb, a) = read_pixel(frame, &layout, color, x, y, w, h, max)?;
            if a == 0. {
                continue;
            }
            if color.alpha == Alpha::PremultipliedEncoded {
                rgb = rgb.map(|v| v / a);
            }
            let linear = transform.decode(rgb)?;
            clipped += usize::from(linear.signal_clipped);
            let p = [
                (linear.rgb[0] * a) as f32,
                (linear.rgb[1] * a) as f32,
                (linear.rgb[2] * a) as f32,
                a as f32,
            ];
            if p.iter().any(|n| !n.is_finite()) {
                return Err(RenderError::Invalid("linear pixel overflow"));
            }
            image.pixels[y * w as usize + x] = p;
        }
    }
    cancel(check)?;
    Ok(NormalizedFrame {
        image,
        clipped_signal_pixels: clipped,
    })
}
#[allow(clippy::too_many_arguments)]
fn read_pixel(
    f: &DecodedFrame,
    l: &Layout,
    c: InputColor,
    x: usize,
    y: usize,
    w: u32,
    h: u32,
    max: f64,
) -> Result<([f64; 3], f64)> {
    let rgb_range = |code: f64| match c.range {
        SignalRange::Full => code / max,
        SignalRange::Limited => {
            (code - 16. * f64::from(1u32 << (l.bits - 8)))
                / (219. * f64::from(1u32 << (l.bits - 8)))
        }
    };
    match l.packing {
        Packing::Packed {
            channels,
            order,
            alpha,
        } => {
            let rgb = [0, 1, 2].map(|i| rgb_range(l.value(f, 0, x, y, channels, order[i])));
            let a = if alpha {
                l.value(f, 0, x, y, channels, order[3]) / max
            } else {
                1.
            };
            Ok((rgb, a))
        }
        Packing::PlanarRgb { alpha } => {
            let rgb = [2, 0, 1].map(|p| rgb_range(l.value(f, p, x, y, 1, 0)));
            Ok((
                rgb,
                if alpha {
                    l.value(f, 3, x, y, 1, 0) / max
                } else {
                    1.
                },
            ))
        }
        Packing::Yuv {
            sx,
            sy,
            semi,
            alpha,
        } => {
            let (ox, oy) = c.chroma.map_or((0., 0.), |v| v.offset(sx, sy));
            let cx = (x as f64 - ox) / f64::from(sx);
            let cy = (y as f64 - oy) / f64::from(sy);
            let cw = w.div_ceil(sx) as usize;
            let ch = h.div_ceil(sy) as usize;
            let cb = bilinear(f, l, 1, cx, cy, cw, ch, if semi { 2 } else { 1 }, 0);
            let cr = bilinear(
                f,
                l,
                if semi { 1 } else { 2 },
                cx,
                cy,
                cw,
                ch,
                if semi { 2 } else { 1 },
                usize::from(semi),
            );
            let yy = l.value(f, 0, x, y, 1, 0);
            let scale = f64::from(1u32 << (l.bits - 8));
            let (yy, cb, cr) = match c.range {
                SignalRange::Full => (
                    yy / max,
                    (cb - f64::from(1u32 << (l.bits - 1))) / max,
                    (cr - f64::from(1u32 << (l.bits - 1))) / max,
                ),
                SignalRange::Limited => (
                    (yy - 16. * scale) / (219. * scale),
                    (cb - 128. * scale) / (224. * scale),
                    (cr - 128. * scale) / (224. * scale),
                ),
            };
            let (kr, kb) = c.matrix.coefficients()?;
            let r = yy + 2. * (1. - kr) * cr;
            let b = yy + 2. * (1. - kb) * cb;
            let g = (yy - kr * r - kb * b) / (1. - kr - kb);
            Ok((
                [r, g, b],
                if alpha {
                    l.value(f, 3, x, y, 1, 0) / max
                } else {
                    1.
                },
            ))
        }
    }
}
#[allow(clippy::too_many_arguments)]
fn bilinear(
    f: &DecodedFrame,
    l: &Layout,
    p: usize,
    x: f64,
    y: f64,
    w: usize,
    h: usize,
    channels: usize,
    channel: usize,
) -> f64 {
    let x = x.clamp(0., (w - 1) as f64);
    let y = y.clamp(0., (h - 1) as f64);
    let x0 = libm::floor(x) as usize;
    let y0 = libm::floor(y) as usize;
    let x1 = (x0 + 1).min(w - 1);
    let y1 = (y0 + 1).min(h - 1);
    let fx = x - x0 as f64;
    let fy = y - y0 as f64;
    let a = l.value(f, p, x0, y0, channels, channel) * (1. - fx)
        + l.value(f, p, x1, y0, channels, channel) * fx;
    let b = l.value(f, p, x0, y1, channels, channel) * (1. - fx)
        + l.value(f, p, x1, y1, channels, channel) * fx;
    a * (1. - fy) + b * fy
}
