use crate::{RenderError, Result};
use mo_media::DecodedFrame;
#[derive(Debug, Clone, Copy)]
pub enum Packing {
    Packed {
        channels: usize,
        alpha: bool,
        order: [usize; 4],
    },
    PlanarRgb {
        alpha: bool,
    },
    Yuv {
        sx: u32,
        sy: u32,
        semi: bool,
        alpha: bool,
    },
}
pub struct Layout {
    pub packing: Packing,
    pub bits: u32,
    pub shift: u32,
    pub big_endian: bool,
    pub bytes: usize,
}
impl Layout {
    pub fn parse(format: &str) -> Result<Self> {
        for (name, channels, order) in [
            ("rgba", 4, [0, 1, 2, 3]),
            ("bgra", 4, [2, 1, 0, 3]),
            ("argb", 4, [1, 2, 3, 0]),
            ("abgr", 4, [3, 2, 1, 0]),
            ("rgb24", 3, [0, 1, 2, 0]),
            ("bgr24", 3, [2, 1, 0, 0]),
        ] {
            if format == name {
                return Ok(Self {
                    packing: Packing::Packed {
                        channels,
                        alpha: channels == 4,
                        order,
                    },
                    bits: 8,
                    shift: 0,
                    big_endian: false,
                    bytes: 1,
                });
            }
        }
        if format == "nv12" || format == "p010le" || format == "p010be" {
            return Ok(Self {
                packing: Packing::Yuv {
                    sx: 2,
                    sy: 2,
                    semi: true,
                    alpha: false,
                },
                bits: if format == "nv12" { 8 } else { 10 },
                shift: if format == "nv12" { 0 } else { 6 },
                big_endian: format.ends_with("be"),
                bytes: if format == "nv12" { 1 } else { 2 },
            });
        }
        for (prefix, packing) in [
            (
                "yuv420p",
                Packing::Yuv {
                    sx: 2,
                    sy: 2,
                    semi: false,
                    alpha: false,
                },
            ),
            (
                "yuv422p",
                Packing::Yuv {
                    sx: 2,
                    sy: 1,
                    semi: false,
                    alpha: false,
                },
            ),
            (
                "yuv444p",
                Packing::Yuv {
                    sx: 1,
                    sy: 1,
                    semi: false,
                    alpha: false,
                },
            ),
            (
                "yuva420p",
                Packing::Yuv {
                    sx: 2,
                    sy: 2,
                    semi: false,
                    alpha: true,
                },
            ),
            (
                "yuva444p",
                Packing::Yuv {
                    sx: 1,
                    sy: 1,
                    semi: false,
                    alpha: true,
                },
            ),
            ("gbrp", Packing::PlanarRgb { alpha: false }),
            ("gbrap", Packing::PlanarRgb { alpha: true }),
        ] {
            if format == prefix {
                return Ok(Self {
                    packing,
                    bits: 8,
                    shift: 0,
                    big_endian: false,
                    bytes: 1,
                });
            }
            for bits in [9, 10, 12, 14, 16] {
                for endian in ["le", "be"] {
                    if format == format!("{prefix}{bits}{endian}") {
                        return Ok(Self {
                            packing,
                            bits,
                            shift: 0,
                            big_endian: endian == "be",
                            bytes: 2,
                        });
                    }
                }
            }
        }
        Err(RenderError::Unsupported("pixel format"))
    }
    pub fn has_alpha(&self) -> bool {
        match self.packing {
            Packing::Packed { alpha, .. } => alpha,
            Packing::PlanarRgb { alpha } | Packing::Yuv { alpha, .. } => alpha,
        }
    }
    pub fn validate(
        &self,
        f: &DecodedFrame,
        w: u32,
        h: u32,
        check: &dyn Fn() -> bool,
    ) -> Result<()> {
        let full = (w, h, 1usize);
        let shape = match self.packing {
            Packing::Packed { channels, .. } => vec![(w, h, channels)],
            Packing::PlanarRgb { alpha } => vec![full; if alpha { 4 } else { 3 }],
            Packing::Yuv {
                sx,
                sy,
                semi,
                alpha,
            } => {
                let c = (w.div_ceil(sx), h.div_ceil(sy), if semi { 2 } else { 1 });
                let mut v = vec![full, c];
                if !semi {
                    v.push(c);
                }
                if alpha {
                    v.push(full);
                }
                v
            }
        };
        if f.planes.len() != shape.len() {
            return Err(RenderError::Invalid("plane count"));
        }
        for (p, (w, h, c)) in f.planes.iter().zip(shape) {
            let row = w as usize * c * self.bytes;
            if p.row_bytes != row || p.rows != h as usize || p.bytes.len() != row * h as usize {
                return Err(RenderError::Invalid("plane dimensions/length"));
            }
            for block in p.bytes.chunks(16384) {
                crate::cancel(check)?;
                if self.bytes == 2 {
                    for pair in block.chunks_exact(2) {
                        let code = if self.big_endian {
                            u16::from_be_bytes([pair[0], pair[1]])
                        } else {
                            u16::from_le_bytes([pair[0], pair[1]])
                        };
                        if (u32::from(code) >> self.shift) > ((1 << self.bits) - 1)
                            || (u32::from(code) & ((1 << self.shift) - 1)) != 0
                        {
                            return Err(RenderError::Invalid(
                                "sample exceeds declared bit depth or alignment",
                            ));
                        }
                    }
                }
            }
        }
        Ok(())
    }
    pub fn value(
        &self,
        f: &DecodedFrame,
        plane: usize,
        x: usize,
        y: usize,
        channels: usize,
        channel: usize,
    ) -> f64 {
        let p = &f.planes[plane];
        let offset = y * p.row_bytes + (x * channels + channel) * self.bytes;
        let code = if self.bytes == 1 {
            u16::from(p.bytes[offset])
        } else if self.big_endian {
            u16::from_be_bytes([p.bytes[offset], p.bytes[offset + 1]])
        } else {
            u16::from_le_bytes([p.bytes[offset], p.bytes[offset + 1]])
        };
        f64::from(code >> self.shift)
    }
}
