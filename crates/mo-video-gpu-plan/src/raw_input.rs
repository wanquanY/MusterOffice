//! A raw-plane input contract. WebCodecs copies unchanged code values; shared
//! Rust owns pixel layout, chroma reconstruction, display EOTF and gamut policy.
use mo_color::rgb::{Primaries, REFERENCE_WHITE_NITS, Transfer};
use mo_media::CodecColorTags;
use mo_video_render::{
    HlgDisplay, InputColor, Matrix, PixelLayout, PixelPacking, RenderError, Result, SignalRange,
};
use serde::{Deserialize, Serialize};
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct BrowserColorSpace {
    pub primaries: String,
    pub transfer: String,
    pub matrix: String,
    pub full_range: bool,
}
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct BrowserFrameInput {
    pub width: u32,
    pub height: u32,
    pub format: String,
    pub color_space: BrowserColorSpace,
    /// Required for subsampled pixels. Must come from bitstream metadata or an
    /// explicit recorded source interpretation, not a guessed resolution rule.
    pub chroma_location: Option<String>,
    pub hlg_display: Option<BrowserHlgDisplay>,
}
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct BrowserHlgDisplay {
    pub peak_nits: f64,
    pub system_gamma: f64,
}
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CopyPlane {
    pub offset: u32,
    pub stride: u32,
}
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RawInputPlan {
    pub profile: &'static str,
    pub source: BrowserFrameInput,
    pub byte_length: u32,
    pub has_hdr: bool,
    pub planes: Vec<CopyPlane>,
    pub parameters: Vec<u32>,
}
impl BrowserFrameInput {
    pub fn compile(self) -> Result<RawInputPlan> {
        let (w, h) = (self.width, self.height);
        if w == 0 || h == 0 || w > 16384 || h > 16384 {
            return Err(RenderError::Invalid("raw input extent"));
        }
        let (native, opaque) = match self.format.as_str() {
            "RGBA" => ("rgba".into(), false),
            "BGRA" => ("bgra".into(), false),
            "RGBX" => ("rgba".into(), true),
            "BGRX" => ("bgra".into(), true),
            "NV12" => ("nv12".into(), true),
            f => {
                let (prefix, rest) = if let Some(r) = f.strip_prefix("I420A") {
                    ("yuva420p", r)
                } else if let Some(r) = f.strip_prefix("I420") {
                    ("yuv420p", r)
                } else if let Some(r) = f.strip_prefix("I422") {
                    ("yuv422p", r)
                } else if let Some(r) = f.strip_prefix("I444A") {
                    ("yuva444p", r)
                } else if let Some(r) = f.strip_prefix("I444") {
                    ("yuv444p", r)
                } else {
                    return Err(RenderError::Unsupported("WebCodecs pixel format"));
                };
                let suffix = match rest {
                    "" => "",
                    "P10" => "10le",
                    "P12" => "12le",
                    _ => return Err(RenderError::Unsupported("WebCodecs bit depth")),
                };
                (format!("{prefix}{suffix}"), !prefix.starts_with("yuva"))
            }
        };
        let layout = PixelLayout::parse(&native)?;
        let c = &self.color_space;
        let tags = CodecColorTags {
            primaries: match c.primaries.as_str() {
                "bt709" => 1,
                "bt470bg" => 5,
                "smpte170m" => 6,
                "bt2020" => 9,
                "smpte432" => 12,
                _ => return Err(RenderError::Unsupported("browser primaries")),
            },
            transfer: match c.transfer.as_str() {
                "bt709" => 1,
                "smpte170m" => 6,
                "iec61966-2-1" => 13,
                "linear" => 8,
                "pq" | "smpte2084" => 16,
                "hlg" | "arib-std-b67" => 18,
                _ => return Err(RenderError::Unsupported("browser transfer")),
            },
            matrix: match c.matrix.as_str() {
                "rgb" => 0,
                "bt709" => 1,
                "bt470bg" => 5,
                "smpte170m" => 6,
                "bt2020-ncl" => 9,
                _ => return Err(RenderError::Unsupported("browser matrix")),
            },
            range: if c.full_range { 2 } else { 1 },
            chroma_location: match self.chroma_location.as_deref() {
                None => 0,
                Some("left") => 1,
                Some("center") => 2,
                Some("topLeft") => 3,
                Some("top") => 4,
                Some("bottomLeft") => 5,
                Some("bottom") => 6,
                _ => return Err(RenderError::Unsupported("chroma location")),
            },
        };
        let color = InputColor::from_codec_tags(
            tags,
            2,
            !opaque,
            self.hlg_display.as_ref().map(|d| HlgDisplay {
                peak_nits: d.peak_nits,
                system_gamma: d.system_gamma,
            }),
        )?;
        let (kind, sx, sy, semi, shapes, order) = match layout.packing {
            PixelPacking::Packed {
                channels, order, ..
            } => {
                if color.matrix != Matrix::Rgb {
                    return Err(RenderError::Invalid("packed RGB matrix"));
                }
                (
                    0,
                    1,
                    1,
                    false,
                    vec![(w, h, channels as u32)],
                    order.map(|v| v as u32),
                )
            }
            PixelPacking::Yuv {
                sx,
                sy,
                semi,
                alpha,
            } => {
                color.matrix.coefficients()?;
                if (sx > 1 || sy > 1) && color.chroma.is_none() {
                    return Err(RenderError::Unsupported(
                        "subsampled chroma location required",
                    ));
                }
                let cw = w.div_ceil(sx);
                let ch = h.div_ceil(sy);
                let mut shapes = vec![(w, h, 1), (cw, ch, if semi { 2 } else { 1 })];
                if !semi {
                    shapes.push((cw, ch, 1));
                }
                if alpha {
                    shapes.push((w, h, 1));
                }
                (1, sx, sy, semi, shapes, [0; 4])
            }
            _ => return Err(RenderError::Unsupported("WebCodecs planar RGB")),
        };
        // 14 vec4 blocks, exact little-endian u32 transport (224 bytes).
        let mut words = vec![0u32; 56];
        words[0..4].copy_from_slice(&[w, h, kind, layout.bits]);
        words[4..8].copy_from_slice(&[sx, sy, layout.bytes as u32, u32::from(!opaque)]);
        let mut planes = Vec::new();
        let mut offset = 0u32;
        for (i, (pw, ph, channels)) in shapes.into_iter().enumerate() {
            let stride = (pw * channels * layout.bytes as u32).div_ceil(4) * 4;
            words[8 + i * 4..12 + i * 4].copy_from_slice(&[offset, stride, channels, 0]);
            planes.push(CopyPlane { offset, stride });
            offset = offset
                .checked_add(stride.checked_mul(ph).ok_or(RenderError::Budget)?)
                .ok_or(RenderError::Budget)?;
        }
        if offset > 256 << 20 {
            return Err(RenderError::Budget);
        }
        words[7] |= u32::from(semi) << 8;
        words[24..28].copy_from_slice(&order);
        for (i, row) in color
            .encoding
            .primaries
            .to(Primaries::Bt2020)
            .0
            .iter()
            .enumerate()
        {
            for (j, v) in row.iter().enumerate() {
                words[28 + i * 4 + j] = (*v as f32).to_bits();
            }
        }
        let max = f64::from((1u32 << layout.bits) - 1);
        let scale = f64::from(1u32 << (layout.bits - 8));
        let signal = if color.range == SignalRange::Full {
            [0., 1. / max, f64::from(1u32 << (layout.bits - 1)), 1. / max]
        } else {
            [
                16. * scale,
                1. / (219. * scale),
                128. * scale,
                1. / (224. * scale),
            ]
        };
        for (i, v) in signal.into_iter().enumerate() {
            words[40 + i] = (v as f32).to_bits();
        }
        let (ox, oy) = color.chroma.map_or((0., 0.), |v| v.offset(sx, sy));
        let (kr, kb) = if kind == 1 {
            color.matrix.coefficients()?
        } else {
            (0., 0.)
        };
        for (i, v) in [ox, oy, kr, kb].into_iter().enumerate() {
            words[44 + i] = (v as f32).to_bits();
        }
        let (transfer, peak, gamma) = match color.encoding.transfer {
            Transfer::Linear => (0., 0., 0.),
            Transfer::Srgb => (1., 0., 0.),
            Transfer::Bt1886 => (2., 0., 0.),
            Transfer::Gamma22 => (3., 0., 0.),
            Transfer::Gamma28 => (4., 0., 0.),
            Transfer::Pq => (5., 0., 0.),
            Transfer::Hlg {
                peak_nits,
                system_gamma,
            } => (6., peak_nits, system_gamma),
        };
        for (i, v) in [transfer, peak, gamma, REFERENCE_WHITE_NITS]
            .into_iter()
            .enumerate()
        {
            words[48 + i] = (v as f32).to_bits();
        }
        for (i, v) in color.encoding.primaries.luminance().into_iter().enumerate() {
            words[52 + i] = (v as f32).to_bits();
        }
        Ok(RawInputPlan {
            profile: "musteroffice.video.raw-input/1-draft",
            source: self,
            byte_length: offset,
            has_hdr: color.encoding.transfer.is_hdr(),
            planes,
            parameters: words,
        })
    }
}
pub const RAW_INPUT_SHADER: &str = include_str!("raw-input.wgsl");
