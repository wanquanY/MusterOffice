use crate::{PageError, PageFeature, PagePaintDefaults};
use mo_common::ObjectId;
use mo_geometry::Fixed;
use mo_presentation_model::{Color, Fill, LineCap, LineJoin, Stroke, Theme};
use mo_raster::{StrokeCap, StrokeJoin, StrokeStyle};

#[derive(Clone, Copy)]
pub(crate) struct Paint {
    pub color: [u8; 4],
    pub stroke: Option<StrokeStyle>,
    pub miter_error: Fixed,
}
impl Paint {
    pub fn fill(color: [u8; 4]) -> Self {
        Self {
            color,
            stroke: None,
            miter_error: Fixed::ZERO,
        }
    }
}
pub(crate) struct PaintContext<'a> {
    pub defaults: &'a PagePaintDefaults,
    pub theme: Option<&'a Theme>,
}
impl PaintContext<'_> {
    fn color(&self, color: &Color, object: Option<&ObjectId>) -> Result<[u8; 4], PageError> {
        let rgba = match color {
            Color::Srgb { rgba } => *rgba,
            Color::Theme { slot } => *self
                .theme
                .and_then(|t| t.colors.get(slot))
                .or_else(|| self.defaults.theme_colors.get(slot))
                .ok_or_else(|| PageError::MissingThemeColor {
                    object: object.cloned(),
                    slot: *slot,
                })?,
        };
        Ok([rgba.red, rgba.green, rgba.blue, rgba.alpha])
    }
    pub fn fill(&self, fill: &Fill, object: Option<&ObjectId>) -> Result<Option<Paint>, PageError> {
        match fill {
            Fill::None => Ok(None),
            Fill::Solid { color } => self.color(color, object).map(Paint::fill).map(Some),
        }
    }
    pub fn stroke(&self, stroke: &Stroke, object: &ObjectId) -> Result<Option<Paint>, PageError> {
        let Stroke::Solid {
            color,
            width,
            cap,
            join,
        } = stroke
        else {
            return Ok(None);
        };
        let (Some(cap), Some(join)) = (cap, join) else {
            return Err(PageError::Unsupported {
                object: Some(object.clone()),
                feature: PageFeature::UnresolvedStrokeParameters,
            });
        };
        let mut miter_error = Fixed::ZERO;
        let join = match *join {
            LineJoin::Round {} => StrokeJoin::Round {},
            LineJoin::Bevel {} => StrokeJoin::Bevel {},
            LineJoin::Miter { limit } => {
                let limit = limit.ok_or_else(|| PageError::Unsupported {
                    object: Some(object.clone()),
                    feature: PageFeature::UnresolvedStrokeParameters,
                })?;
                // u32 * 2^32 fits i128. Nearest ties-away at the author -> Q32
                // boundary; report the exact rational error rounded outwards.
                let numerator = i128::from(limit) * (1i128 << 32);
                let raw = (numerator + 50000) / 100000;
                miter_error = Fixed::from_raw(((raw * 100000 - numerator).abs() + 99999) / 100000);
                StrokeJoin::Miter {
                    limit: Fixed::from_raw(raw),
                }
            }
        };
        Ok(Some(Paint {
            color: self.color(color, Some(object))?,
            miter_error,
            stroke: Some(StrokeStyle {
                width: Fixed::emu(*width),
                cap: match cap {
                    LineCap::Flat => StrokeCap::Butt,
                    LineCap::Round => StrokeCap::Round,
                    LineCap::Square => StrokeCap::Square,
                },
                join,
            }),
        }))
    }
}
