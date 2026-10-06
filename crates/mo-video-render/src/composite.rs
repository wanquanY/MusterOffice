use crate::{
    Affine, LinearImage, PixelLimits, RenderError, Result, SourceGeometry, cancel,
    geometry::author_transform,
};
use mo_color::rgb::{Encoding, Primaries, RgbTransform, Transfer};
use mo_time::{ExactRatio, Rounding};
use mo_video_compile::{MediaBinding, VideoFramePlan, VisualEffect, VisualNode, VisualSource};
use mo_video_model::{MediaStream, TimeRange, TimeSample};
use std::sync::Arc;
#[derive(Debug, Clone, Copy)]
pub enum Sampling {
    Nearest,
    Bilinear,
}
#[derive(Debug, Clone, Copy)]
pub struct RenderLimits {
    pub pixels: PixelLimits,
    pub max_nodes: usize,
    pub max_depth: usize,
    pub max_effects: usize,
    pub max_pixel_visits: usize,
    pub max_temporary_bytes: usize,
}
impl Default for RenderLimits {
    fn default() -> Self {
        Self {
            pixels: PixelLimits::default(),
            max_nodes: 4096,
            max_depth: 32,
            max_effects: 4096,
            max_pixel_visits: 100_000_000,
            max_temporary_bytes: 512 << 20,
        }
    }
}
pub struct ResolvedMedia {
    pub binding: MediaBinding,
    /// Required for a video frame, absent for a static image. Must cover the exact sample.
    pub interval: Option<TimeRange>,
    pub image: Arc<LinearImage>,
    pub geometry: SourceGeometry,
}
pub trait VisualResources {
    fn media(
        &mut self,
        binding: &MediaBinding,
        sample: &TimeSample,
        check: &dyn Fn() -> bool,
    ) -> Result<ResolvedMedia>;
}
#[derive(Debug, Default, Clone, Copy)]
pub struct RenderStats {
    pub nodes: usize,
    pub pixel_visits: usize,
    pub peak_temporary_bytes: usize,
}
pub struct RenderedVideo {
    pub image: LinearImage,
    pub stats: RenderStats,
}
struct Work<'a> {
    limits: RenderLimits,
    stats: RenderStats,
    temporary: usize,
    check: &'a dyn Fn() -> bool,
    sampling: Sampling,
}
/// Pure reference execution. A failed/cancelled call never returns partial pixels.
/// Resolver-owned normalized source frames have their own budget; this accounts
/// for current composition surfaces. It is not a real-time GPU performance profile.
pub fn render_frame(
    plan: &VideoFramePlan,
    resources: &mut dyn VisualResources,
    sampling: Sampling,
    limits: RenderLimits,
    check: &dyn Fn() -> bool,
) -> Result<RenderedVideo> {
    cancel(check)?;
    if limits.max_depth == 0
        || limits.max_depth > 64
        || limits.max_nodes == 0
        || limits.max_nodes > 100_000
        || limits.max_effects == 0
        || limits.max_effects > 100_000
    {
        return Err(RenderError::Invalid("render limits"));
    }
    let count = crate::pixel_count(plan.canvas.width, plan.canvas.height, limits.pixels)?;
    let mut nodes = 0;
    let mut effects = 0;
    admit(&plan.layers, 1, limits, &mut nodes, &mut effects, check)?;
    if count
        .checked_mul(nodes)
        .is_none_or(|v| v > limits.max_pixel_visits)
    {
        return Err(RenderError::Budget);
    }
    let mut work = Work {
        limits,
        stats: RenderStats::default(),
        temporary: 0,
        check,
        sampling,
    };
    let image = work.group(
        &plan.layers,
        (plan.canvas.width, plan.canvas.height),
        resources,
    )?;
    cancel(check)?;
    Ok(RenderedVideo {
        image,
        stats: work.stats,
    })
}
fn admit(
    nodes: &[VisualNode],
    depth: usize,
    limits: RenderLimits,
    count: &mut usize,
    effects: &mut usize,
    check: &dyn Fn() -> bool,
) -> Result<()> {
    if depth > limits.max_depth {
        return Err(RenderError::Budget);
    }
    for n in nodes {
        cancel(check)?;
        *count = count.checked_add(1).ok_or(RenderError::Budget)?;
        *effects = effects
            .checked_add(n.effects.len())
            .ok_or(RenderError::Budget)?;
        if *count > limits.max_nodes || *effects > limits.max_effects {
            return Err(RenderError::Budget);
        }
        for effect in &n.effects {
            match effect {
                VisualEffect::Transform {
                    scale_x_ppm,
                    scale_y_ppm,
                    opacity_ppm,
                    ..
                } => {
                    if *scale_x_ppm == 0 || *scale_y_ppm == 0 || *opacity_ppm > 1_000_000 {
                        return Err(RenderError::Invalid("transform parameters"));
                    }
                }
                VisualEffect::Opacity { factor } => {
                    factor.check_budget(4096)?;
                    if factor.compare(&ExactRatio::integer(0)).is_lt()
                        || factor.compare(&ExactRatio::integer(1)).is_gt()
                    {
                        return Err(RenderError::Invalid("opacity range"));
                    }
                }
            }
        }
        match &n.source {
            VisualSource::Group { layers, .. } => {
                admit(layers, depth + 1, limits, count, effects, check)?
            }
            VisualSource::Text { .. } => {
                return Err(RenderError::Unsupported("text layout/raster not connected"));
            }
            _ => (),
        }
    }
    Ok(())
}
impl Work<'_> {
    fn group(
        &mut self,
        nodes: &[VisualNode],
        canvas: (u32, u32),
        resources: &mut dyn VisualResources,
    ) -> Result<LinearImage> {
        let bytes = crate::pixel_count(canvas.0, canvas.1, self.limits.pixels)? * 16;
        self.temporary = self
            .temporary
            .checked_add(bytes)
            .filter(|v| *v <= self.limits.max_temporary_bytes)
            .ok_or(RenderError::Budget)?;
        self.stats.peak_temporary_bytes = self.stats.peak_temporary_bytes.max(self.temporary);
        let mut out = LinearImage::blank(canvas.0, canvas.1, self.limits.pixels, self.check)?;
        for node in nodes {
            cancel(self.check)?;
            self.stats.nodes += 1;
            let (effect, opacity) = effects(&node.effects, canvas)?;
            match &node.source {
                VisualSource::Solid { rgba } => {
                    let linear = RgbTransform::new(
                        Encoding {
                            primaries: Primaries::Bt709,
                            transfer: Transfer::Srgb,
                        },
                        Primaries::Bt2020,
                    )?
                    .decode([rgba[0], rgba[1], rgba[2]].map(|v| f64::from(v) / 255.))?
                    .rgb;
                    let a = f64::from(rgba[3]) / 255.;
                    let p = [
                        (linear[0] * a) as f32,
                        (linear[1] * a) as f32,
                        (linear[2] * a) as f32,
                        a as f32,
                    ];
                    self.paint(&mut out, None, p, effect, opacity)?;
                }
                VisualSource::Media { binding, mapping } => {
                    let source = resources.media(binding, &mapping.sample, self.check)?;
                    cancel(self.check)?;
                    if &source.binding != binding {
                        return Err(RenderError::Invalid("resolved resource binding"));
                    }
                    match binding.descriptor {
                        MediaStream::Video {
                            dimensions,
                            available,
                            ..
                        } => {
                            if !available.valid() || !mapping.sample.covers(available) {
                                return Err(RenderError::Invalid("source sample availability"));
                            }
                            if !source
                                .interval
                                .is_some_and(|i| i.valid() && mapping.sample.covers(i))
                            {
                                return Err(RenderError::Invalid("resolved video frame interval"));
                            }
                            if (source.image.width, source.image.height)
                                != (dimensions.width, dimensions.height)
                            {
                                return Err(RenderError::Invalid("resolved video dimensions"));
                            }
                        }
                        MediaStream::Image { dimensions } => {
                            if source.interval.is_some()
                                || (source.image.width, source.image.height)
                                    != (dimensions.width, dimensions.height)
                            {
                                return Err(RenderError::Invalid(
                                    "resolved image dimensions or interval",
                                ));
                            }
                        }
                        _ => return Err(RenderError::Invalid("audio in visual plan")),
                    }
                    let placement = source
                        .geometry
                        .contain((source.image.width, source.image.height), canvas)?;
                    self.paint(
                        &mut out,
                        Some(&source.image),
                        [0.; 4],
                        effect.compose(placement)?,
                        opacity,
                    )?;
                    out.hdr |= source.image.hdr;
                }
                VisualSource::Group { layers, .. } => {
                    let group = self.group(layers, canvas, resources)?;
                    self.paint(&mut out, Some(&group), [0.; 4], effect, opacity)?;
                    out.hdr |= group.hdr;
                    drop(group);
                    self.temporary -= bytes;
                }
                VisualSource::Text { .. } => unreachable!("preflight rejects text"),
            }
        }
        cancel(self.check)?;
        Ok(out)
    }
    fn paint(
        &mut self,
        out: &mut LinearImage,
        source: Option<&LinearImage>,
        solid: [f32; 4],
        matrix: Affine,
        opacity: f64,
    ) -> Result<()> {
        let inverse = matrix.inverse()?;
        let w = source.map_or(out.width, |s| s.width);
        let h = source.map_or(out.height, |s| s.height);
        for y in 0..out.height as usize {
            cancel(self.check)?;
            for x in 0..out.width as usize {
                self.stats.pixel_visits += 1;
                let (sx, sy) = inverse.map(x as f64 + 0.5, y as f64 + 0.5);
                if !sx.is_finite() || !sy.is_finite() {
                    return Err(RenderError::Invalid("sample coordinate overflow"));
                }
                if sx < 0. || sy < 0. || sx >= f64::from(w) || sy >= f64::from(h) {
                    continue;
                }
                let p = source.map_or(solid, |s| sample(s, sx, sy, self.sampling));
                let src = p.map(|v| f64::from(v) * opacity);
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
fn effects(effects: &[VisualEffect], canvas: (u32, u32)) -> Result<(Affine, f64)> {
    let mut matrix = Affine::identity();
    let mut opacity = 1.;
    for effect in effects {
        match effect {
            VisualEffect::Transform {
                x_milli_pixels,
                y_milli_pixels,
                scale_x_ppm,
                scale_y_ppm,
                rotation_milli_degrees,
                opacity_ppm,
            } => {
                matrix = author_transform(
                    canvas,
                    (*x_milli_pixels, *y_milli_pixels),
                    (*scale_x_ppm, *scale_y_ppm),
                    *rotation_milli_degrees,
                )?
                .compose(matrix)?;
                opacity *= f64::from(*opacity_ppm) / 1_000_000.;
            }
            VisualEffect::Opacity { factor } => {
                // Explicit Q32 render-boundary quantization, <= 2^-33 error per factor.
                let fixed = factor
                    .mul(&ExactRatio::integer(1i64 << 32), 4096)?
                    .round_integer(Rounding::NearestTiesEven)?;
                opacity *= fixed as f64 / 4294967296.;
            }
        }
    }
    Ok((matrix, opacity))
}
fn sample(s: &LinearImage, x: f64, y: f64, sampling: Sampling) -> [f32; 4] {
    let at = |x: usize, y: usize| s.pixels[y * s.width as usize + x];
    if matches!(sampling, Sampling::Nearest) {
        return at(libm::floor(x) as usize, libm::floor(y) as usize);
    }
    let x = (x - 0.5).clamp(0., f64::from(s.width - 1));
    let y = (y - 0.5).clamp(0., f64::from(s.height - 1));
    let x0 = libm::floor(x) as usize;
    let y0 = libm::floor(y) as usize;
    let x1 = (x0 + 1).min(s.width as usize - 1);
    let y1 = (y0 + 1).min(s.height as usize - 1);
    let fx = x - x0 as f64;
    let fy = y - y0 as f64;
    let a = at(x0, y0);
    let b = at(x1, y0);
    let c = at(x0, y1);
    let d = at(x1, y1);
    std::array::from_fn(|i| {
        ((f64::from(a[i]) * (1. - fx) + f64::from(b[i]) * fx) * (1. - fy)
            + (f64::from(c[i]) * (1. - fx) + f64::from(d[i]) * fx) * fy) as f32
    })
}
