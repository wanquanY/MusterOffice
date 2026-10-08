//! Shared explicit television output ABI. Three bounded GPU passes retain the
//! CPU reference's matte, tone policy, transfer, matrix and chroma filter.
use crate::ColorParameters;
use mo_color::{rgb::Primaries, srgb_to_linear};
use mo_video_render::{OutputAlpha, RenderError, Result, ToneMap, i420_chroma_weights};

/// 176-byte uniform, all plane rows aligned to four bytes. Padding is zero and
/// is never part of visible sample values. No WebCodecs color conversion needed.
#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct I420Parameters {
    color: ColorParameters,
    matte: [f32; 4],
    horizontal: [[f32; 4]; 2],
    vertical: [[f32; 4]; 2],
    layout: [u32; 4],
}
impl I420Parameters {
    pub fn new(
        width: u32,
        height: u32,
        hdr: bool,
        tone: Option<ToneMap>,
        matte: [u8; 3],
    ) -> Result<Self> {
        if !(2..=8192).contains(&width)
            || !(2..=8192).contains(&height)
            || !width.is_multiple_of(2)
            || !height.is_multiple_of(2)
        {
            return Err(RenderError::Unsupported(
                "GPU I420 requires even 2..8192 dimensions",
            ));
        }
        let color = ColorParameters::srgb_output(width, height, hdr, tone, OutputAlpha::Straight)?;
        let bg = Primaries::Bt709
            .to(Primaries::Bt2020)
            .apply(matte.map(|v| srgb_to_linear(f64::from(v) / 255.)));
        let (horizontal, vertical) = i420_chroma_weights();
        let pack =
            |v: [f64; 8]| std::array::from_fn(|i| std::array::from_fn(|j| v[i * 4 + j] as f32));
        let y_stride = width.div_ceil(4) * 4;
        let c_stride = (width / 2).div_ceil(4) * 4;
        let u_offset = y_stride * height;
        let v_offset = u_offset + c_stride * (height / 2);
        Ok(Self {
            color,
            matte: [bg[0] as f32, bg[1] as f32, bg[2] as f32, 0.],
            horizontal: pack(horizontal),
            vertical: pack(vertical),
            layout: [y_stride, c_stride, u_offset, v_offset],
        })
    }
    pub fn words(self) -> [u32; 44] {
        let mut words = [0; 44];
        words.copy_from_slice(bytemuck::cast_slice(std::slice::from_ref(&self)));
        words
    }
}
pub fn i420_output_shader() -> String {
    format!(
        "{}\n{}",
        include_str!("color.wgsl"),
        include_str!("i420.wgsl")
    )
}
