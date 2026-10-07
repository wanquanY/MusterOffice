use crate::{Affine, RenderError, RenderLimits, Result, Sampling};
use mo_video_compile::VideoFramePlan;

/// Physical pixel grid. Authoring coordinates and source sample times remain
/// those of the immutable plan. The host displays this grid at the project's
/// aspect ratio; fitting/letterboxing an application panel is a host concern.
/// Geometry retains the shared Q32.32 inverse-coefficient raster profile, so a
/// nearest sample exactly on an ideal edge follows that quantized coordinate.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RasterExtent {
    pub width: u32,
    pub height: u32,
}

#[derive(Debug, Clone, Copy)]
pub struct FrameRenderOptions {
    pub raster: RasterExtent,
    pub sampling: Sampling,
    pub limits: RenderLimits,
}
impl FrameRenderOptions {
    /// Original-resolution render, also used by the existing export entry.
    pub fn at_canvas(plan: &VideoFramePlan, sampling: Sampling, limits: RenderLimits) -> Self {
        Self {
            raster: RasterExtent {
                width: plan.canvas.width,
                height: plan.canvas.height,
            },
            sampling,
            limits,
        }
    }
}

/// All groups use this same project-to-raster map. Group effects conjugate the
/// map because the child surface is already rasterized; applying it twice would
/// change translations, pivots, crop boundaries and source placement.
#[derive(Clone, Copy)]
pub(crate) struct Viewport {
    pub canvas: (u32, u32),
    pub raster: (u32, u32),
    project_to_raster: Affine,
    raster_to_project: Affine,
    identity: bool,
}
impl Viewport {
    pub fn new(plan: &VideoFramePlan, raster: RasterExtent) -> Result<Self> {
        let canvas = (plan.canvas.width, plan.canvas.height);
        let raster = (raster.width, raster.height);
        if [canvas.0, canvas.1, raster.0, raster.1]
            .iter()
            .any(|v| *v == 0 || *v > 16384)
        {
            return Err(RenderError::Invalid("render viewport extent"));
        }
        let sx = f64::from(raster.0) / f64::from(canvas.0);
        let sy = f64::from(raster.1) / f64::from(canvas.1);
        Ok(Self {
            canvas,
            raster,
            project_to_raster: Affine::new([sx, 0., 0., sy, 0., 0.])?,
            // Direct ratios avoid unnecessary rounding via determinant inversion.
            raster_to_project: Affine::new([
                f64::from(canvas.0) / f64::from(raster.0),
                0.,
                0.,
                f64::from(canvas.1) / f64::from(raster.1),
                0.,
                0.,
            ])?,
            identity: canvas == raster,
        })
    }
    pub fn source(self, placement: Affine) -> Result<Affine> {
        if self.identity {
            Ok(placement)
        } else {
            self.project_to_raster.compose(placement)
        }
    }
    pub fn group(self, effect: Affine) -> Result<Affine> {
        if self.identity {
            Ok(effect)
        } else {
            self.project_to_raster
                .compose(effect)?
                .compose(self.raster_to_project)
        }
    }
}
