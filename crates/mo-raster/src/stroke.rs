use crate::{PixelScale, RasterError, StrokeCap, StrokeJoin, StrokeStyle, number::Scale};
use mo_geometry::Fixed;
use std::collections::BTreeMap;

/// Intern evaluated stroke parameters. Width is in page/world EMU; geometry
/// transforms have already been applied before this stage. Zero means hairline.
#[derive(Default)]
pub(crate) struct Strokes {
    pub words: Vec<[u32; 4]>,
    indices: BTreeMap<[u32; 4], u32>,
    pub width_error: Fixed,
    pub miter_error: Fixed,
}
impl Strokes {
    pub fn inflation(&self, index: u32) -> f64 {
        if index == 0 {
            return 0.0;
        }
        let s = self.words[index as usize - 1];
        let width = f64::from(f32::from_bits(s[0]));
        if width == 0.0 && s[2] != 3 {
            return 1.0;
        }
        let factor = (if s[1] == 2 { 2.0f64 } else { 1.0 }).max(if s[2] == 0 {
            f64::from(f32::from_bits(s[3]))
        } else if s[2] == 3 {
            // Clip corners also extend sideways by at most the stroke radius.
            f64::from(f32::from_bits(s[3])) + 1.0
        } else {
            1.0
        });
        (if width == 0.0 { 1.0 } else { width }) * factor / 2.0
    }

    pub fn insert(
        &mut self,
        style: &StrokeStyle,
        scale: PixelScale,
        tolerance: Fixed,
    ) -> Result<u32, RasterError> {
        if style.width.raw() < 0 {
            return Err(RasterError::Invalid("negative stroke width"));
        }
        let scale = Scale::new(scale)?;
        let width = scale.value(style.width.raw())?;
        let error = Fixed::from_raw(scale.error(style.width.raw(), width)?);
        if error > tolerance {
            return Err(RasterError::Precision);
        }
        self.width_error = self.width_error.max(error);
        let (join, miter) = match style.join {
            StrokeJoin::Round {} => (1, 0.0),
            StrokeJoin::Bevel {} => (2, 0.0),
            StrokeJoin::Miter { limit } | StrokeJoin::MiterClip { limit } => {
                let clipped = matches!(style.join, StrokeJoin::MiterClip { .. });
                if clipped && limit.raw() < (1i128 << 32) {
                    return Err(RasterError::Invalid(
                        "clipped miter limit must be at least one",
                    ));
                }
                if limit.raw() < 0 || limit.raw() > 1024 * (1i128 << 32) {
                    return Err(RasterError::Invalid(
                        "stroke miter limit must be between zero and 1024",
                    ));
                }
                let ratio = Scale::new(PixelScale {
                    numerator: 1,
                    denominator: 1,
                })?;
                let value = ratio.value(limit.raw())?;
                self.miter_error = self
                    .miter_error
                    .max(Fixed::from_raw(ratio.error(limit.raw(), value)?));
                (if clipped { 3 } else { 0 }, value)
            }
        };
        let words = [
            width.to_bits(),
            match style.cap {
                StrokeCap::Butt => 0,
                StrokeCap::Round => 1,
                StrokeCap::Square => 2,
            },
            join,
            miter.to_bits(),
        ];
        if let Some(index) = self.indices.get(&words) {
            return Ok(*index);
        }
        if self.words.len() >= 4096 {
            return Err(RasterError::Limit("stroke styles"));
        }
        let index = self.words.len() as u32 + 1;
        self.words.push(words);
        self.indices.insert(words, index);
        Ok(index)
    }
}
