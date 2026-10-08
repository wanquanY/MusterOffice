//! One byte-exact GPU ABI used by native wgpu and browser WebGPU executors.
use mo_video_render::{RasterCoordinates, RenderError, Result, Sampling};

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct PaintParameters {
    sizes: [u32; 4],
    row_x: [u32; 4],
    row_y: [u32; 4],
    offset: [u32; 4],
    values: [f32; 4],
    solid: [f32; 4],
    options: [u32; 4],
}
impl PaintParameters {
    pub fn new(
        target: (u32, u32),
        source: (u32, u32),
        coordinates: RasterCoordinates,
        opacity: f64,
        sampling: Sampling,
        solid: Option<[f32; 4]>,
    ) -> Result<Self> {
        if [target.0, target.1, source.0, source.1]
            .iter()
            .any(|n| *n == 0 || *n > 16384)
            || !opacity.is_finite()
            || !(0.0..=1.0).contains(&opacity)
            || solid.is_some_and(|p| {
                p.iter()
                    .any(|v| !v.is_finite() || v.abs() > crate::MAX_ABS_COMPONENT)
                    || !(0.0..=1.0).contains(&p[3])
            })
        {
            return Err(RenderError::Invalid("GPU paint parameters"));
        }
        let m = coordinates
            .coefficients()
            .map(|v| [v as u32, (v as u64 >> 32) as u32]);
        Ok(Self {
            sizes: [target.0, target.1, source.0, source.1],
            row_x: [m[0][0], m[0][1], m[2][0], m[2][1]],
            row_y: [m[1][0], m[1][1], m[3][0], m[3][1]],
            offset: [m[4][0], m[4][1], m[5][0], m[5][1]],
            values: [opacity as f32, 0., 0., 0.],
            solid: solid.unwrap_or([0.; 4]),
            options: [
                u32::from(solid.is_some()),
                u32::from(matches!(sampling, Sampling::Bilinear)),
                0,
                0,
            ],
        })
    }
    /// Transport u32 words as exact JSON integers. The platform executor writes
    /// these words little-endian; it does not rebuild Q32.32 or color math in JS.
    pub fn words(self) -> [u32; 28] {
        bytemuck::cast(self)
    }
}
