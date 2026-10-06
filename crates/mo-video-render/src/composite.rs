use crate::{
    Affine, LinearImage, PixelLimits, RenderError, Result, SourceGeometry,
    backend::{CompositionBackend, CompositionResult, Paint, PaintSource},
    cancel,
    cpu::CpuBackend,
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
struct Work<'a, B: CompositionBackend> {
    backend: &'a mut B,
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
    let mut backend = CpuBackend;
    let result = render_with_backend(plan, resources, sampling, limits, check, &mut backend)?;
    let mut image = result.surface;
    image.hdr = result.has_hdr;
    Ok(RenderedVideo {
        image,
        stats: result.stats,
    })
}
/// Shared validation, resource binding, geometry and group semantics. Backends
/// must discard any encoded work on error; they may not publish partial output.
pub fn render_with_backend<B: CompositionBackend>(
    plan: &VideoFramePlan,
    resources: &mut dyn VisualResources,
    sampling: Sampling,
    limits: RenderLimits,
    check: &dyn Fn() -> bool,
    backend: &mut B,
) -> Result<CompositionResult<B::Surface>> {
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
        backend,
        limits,
        stats: RenderStats::default(),
        temporary: 0,
        check,
        sampling,
    };
    let (surface, has_hdr) = work.group(
        &plan.layers,
        (plan.canvas.width, plan.canvas.height),
        resources,
    )?;
    cancel(check)?;
    Ok(CompositionResult {
        surface,
        has_hdr,
        width: plan.canvas.width,
        height: plan.canvas.height,
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
impl<B: CompositionBackend> Work<'_, B> {
    fn group(
        &mut self,
        nodes: &[VisualNode],
        canvas: (u32, u32),
        resources: &mut dyn VisualResources,
    ) -> Result<(B::Surface, bool)> {
        let bytes = crate::pixel_count(canvas.0, canvas.1, self.limits.pixels)? * 16;
        self.temporary = self
            .temporary
            .checked_add(bytes)
            .filter(|v| *v <= self.limits.max_temporary_bytes)
            .ok_or(RenderError::Budget)?;
        self.stats.peak_temporary_bytes = self.stats.peak_temporary_bytes.max(self.temporary);
        let mut out = self
            .backend
            .create_surface(canvas, self.limits.pixels, self.check)?;
        let mut hdr = false;
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
                    self.paint(
                        &mut out,
                        PaintSource::Solid(p),
                        canvas,
                        canvas,
                        effect,
                        opacity,
                    )?;
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
                        PaintSource::Image(&source.image),
                        canvas,
                        (source.image.width, source.image.height),
                        effect.compose(placement)?,
                        opacity,
                    )?;
                    hdr |= source.image.hdr;
                }
                VisualSource::Group { layers, .. } => {
                    let (group, group_hdr) = self.group(layers, canvas, resources)?;
                    self.paint(
                        &mut out,
                        PaintSource::Surface(&group),
                        canvas,
                        canvas,
                        effect,
                        opacity,
                    )?;
                    hdr |= group_hdr;
                    drop(group);
                    self.temporary -= bytes;
                }
                VisualSource::Text { .. } => unreachable!("preflight rejects text"),
            }
        }
        cancel(self.check)?;
        Ok((out, hdr))
    }
    #[allow(clippy::too_many_arguments)]
    fn paint(
        &mut self,
        out: &mut B::Surface,
        source: PaintSource<'_, B::Surface>,
        target_size: (u32, u32),
        source_size: (u32, u32),
        matrix: Affine,
        opacity: f64,
    ) -> Result<()> {
        self.stats.pixel_visits += target_size.0 as usize * target_size.1 as usize;
        self.backend.paint(
            out,
            Paint {
                source,
                target_size,
                source_size,
                coordinates: crate::RasterCoordinates::new(matrix.inverse()?, target_size)?,
                opacity,
                sampling: self.sampling,
            },
            self.check,
        )
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
