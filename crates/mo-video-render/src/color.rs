use crate::{RenderError, Result};
pub use mo_color::rgb::{Encoding, Primaries, Transfer};
use mo_media::{CodecColorTags, FrameInfo};
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SignalRange {
    Full,
    Limited,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Matrix {
    Rgb,
    Bt601,
    Bt709,
    Bt2020Ncl,
}
impl Matrix {
    pub fn coefficients(self) -> Result<(f64, f64)> {
        match self {
            Self::Bt601 => Ok((0.299, 0.114)),
            Self::Bt709 => Ok((0.2126, 0.0722)),
            Self::Bt2020Ncl => Ok((0.2627, 0.0593)),
            Self::Rgb => Err(RenderError::Invalid("RGB matrix for YUV planes")),
        }
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Alpha {
    Opaque,
    Straight,
    PremultipliedEncoded,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChromaLocation {
    Left,
    Center,
    TopLeft,
    Top,
    BottomLeft,
    Bottom,
}
impl ChromaLocation {
    pub fn offset(self, sx: u32, sy: u32) -> (f64, f64) {
        let x = match self {
            Self::Left | Self::TopLeft | Self::BottomLeft => 0.,
            _ => f64::from(sx - 1) / 2.,
        };
        let y = match self {
            Self::TopLeft | Self::Top => 0.,
            Self::BottomLeft | Self::Bottom => f64::from(sy - 1),
            _ => f64::from(sy - 1) / 2.,
        };
        (x, y)
    }
}
#[derive(Debug, Clone, Copy)]
pub struct InputColor {
    pub encoding: Encoding,
    pub matrix: Matrix,
    pub range: SignalRange,
    pub chroma: Option<ChromaLocation>,
    pub alpha: Alpha,
}
#[derive(Debug, Clone, Copy)]
pub struct HlgDisplay {
    pub peak_nits: f64,
    pub system_gamma: f64,
}
impl InputColor {
    /// Resolve declared color and alpha from the actual decoded pixel layout.
    pub fn from_frame(info: &FrameInfo, hlg: Option<HlgDisplay>) -> Result<Self> {
        Self::from_codec(
            info,
            crate::layout::Layout::parse(&info.format)?.has_alpha(),
            hlg,
        )
    }
    /// Reference-display interpretation: SDR 709/2020 uses ideal-black BT.1886,
    /// not inverse camera OETF. Resolve only known codec declarations. Missing metadata must be supplied
    /// by an explicit host interpretation; never guess from dimensions/extension.
    pub fn from_codec(info: &FrameInfo, has_alpha: bool, hlg: Option<HlgDisplay>) -> Result<Self> {
        Self::from_codec_tags(info.colors, info.alpha_mode, has_alpha, hlg)
    }
    /// Color interpretation shared by native and raw-plane GPU input adapters.
    pub fn from_codec_tags(
        c: CodecColorTags,
        alpha_mode: i32,
        has_alpha: bool,
        hlg: Option<HlgDisplay>,
    ) -> Result<Self> {
        let primaries = match c.primaries {
            1 => Primaries::Bt709,
            5 => Primaries::Bt470Bg,
            6 => Primaries::SmpteC,
            9 => Primaries::Bt2020,
            12 => Primaries::DisplayP3,
            _ => return Err(RenderError::Unsupported("unknown source primaries")),
        };
        let transfer = match c.transfer {
            1 | 6 | 14 | 15 => Transfer::Bt1886,
            4 => Transfer::Gamma22,
            5 => Transfer::Gamma28,
            8 => Transfer::Linear,
            13 => Transfer::Srgb,
            16 => Transfer::Pq,
            18 => {
                let d = hlg.ok_or(RenderError::Unsupported("HLG display parameters required"))?;
                Transfer::Hlg {
                    peak_nits: d.peak_nits,
                    system_gamma: d.system_gamma,
                }
            }
            _ => return Err(RenderError::Unsupported("unknown source transfer")),
        };
        transfer.validate()?;
        let matrix = match c.matrix {
            0 => Matrix::Rgb,
            1 => Matrix::Bt709,
            5 | 6 => Matrix::Bt601,
            9 => Matrix::Bt2020Ncl,
            _ => return Err(RenderError::Unsupported("source color matrix")),
        };
        let range = match c.range {
            1 => SignalRange::Limited,
            2 => SignalRange::Full,
            _ => return Err(RenderError::Unsupported("unknown signal range")),
        };
        let chroma = match c.chroma_location {
            0 => None,
            1 => Some(ChromaLocation::Left),
            2 => Some(ChromaLocation::Center),
            3 => Some(ChromaLocation::TopLeft),
            4 => Some(ChromaLocation::Top),
            5 => Some(ChromaLocation::BottomLeft),
            6 => Some(ChromaLocation::Bottom),
            _ => return Err(RenderError::Unsupported("chroma location")),
        };
        let alpha = if !has_alpha {
            Alpha::Opaque
        } else {
            match alpha_mode {
                1 => Alpha::PremultipliedEncoded,
                2 => Alpha::Straight,
                _ => return Err(RenderError::Unsupported("alpha interpretation required")),
            }
        };
        Ok(Self {
            encoding: Encoding {
                primaries,
                transfer,
            },
            matrix,
            range,
            chroma,
            alpha,
        })
    }
}
