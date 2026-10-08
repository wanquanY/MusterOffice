//! Color boundary ABI. Coefficients come from the same Rust color transforms
//! as CPU rendering; browser transports words without rebuilding color math.
use mo_video_render::{OutputAlpha, RenderError, Result, SdrTransform, ToneMap};

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct ColorParameters {
    size: [u32; 4],
    row0: [f32; 4],
    row1: [f32; 4],
    row2: [f32; 4],
    tone: [f32; 4],
}
impl ColorParameters {
    fn new(
        width: u32,
        height: u32,
        alpha: OutputAlpha,
        matrix: [[f64; 3]; 3],
        luma: [f64; 3],
        white: Option<f64>,
    ) -> Result<Self> {
        if width == 0 || height == 0 || width > 16384 || height > 16384 {
            return Err(RenderError::Invalid("GPU color extent"));
        }
        let row = |v: [f64; 3]| [v[0] as f32, v[1] as f32, v[2] as f32, 0.];
        Ok(Self {
            size: [
                width,
                height,
                u32::from(alpha == OutputAlpha::PremultipliedEncoded),
                0,
            ],
            row0: row(matrix[0]),
            row1: row(matrix[1]),
            row2: row(matrix[2]),
            tone: [
                luma[0] as f32,
                luma[1] as f32,
                luma[2] as f32,
                white.unwrap_or(0.) as f32,
            ],
        })
    }
    pub fn srgb_output(
        width: u32,
        height: u32,
        hdr: bool,
        tone: Option<ToneMap>,
        alpha: OutputAlpha,
    ) -> Result<Self> {
        let params = SdrTransform::new(hdr, tone)?.parameters();
        Self::new(
            width,
            height,
            alpha,
            params.matrix.0,
            params.luma,
            params.white,
        )
    }
    pub fn words(self) -> [u32; 20] {
        bytemuck::cast(self)
    }
}
pub fn srgb_output_shader() -> String {
    format!(
        "{}\n{}",
        include_str!("color.wgsl"),
        include_str!("output.wgsl")
    )
}
