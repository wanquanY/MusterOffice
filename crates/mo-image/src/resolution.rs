//! Source declarations remain in encoded axes; only the derived physical pixel
//! size follows the already applied EXIF orientation. No host DPI is consulted.
use crate::{ImageError, ImageFormat};
use mo_common::Emu;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
mod parse;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Density {
    pub numerator: u32,
    #[schemars(range(min = 1))]
    pub denominator: u32,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum ResolutionUnit {
    AspectRatio,
    Inch,
    Centimetre,
    Metre,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum ResolutionSource {
    PngPhysical,
    Jfif,
    ExifIfd0,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ResolutionDeclaration {
    pub source: ResolutionSource,
    /// Absolute byte offset of the PNG chunk or JPEG marker in source bytes.
    pub source_offset: u32,
    pub x: Option<Density>,
    pub y: Option<Density>,
    /// None preserves an absent EXIF ResolutionUnit. Derivation applies the
    /// Exif specification's inch default without rewriting this declaration.
    pub unit: Option<ResolutionUnit>,
}
/// Reduced, positive rational EMU per normalized output pixel.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PixelExtent {
    pub numerator: Emu,
    #[schemars(range(min = 1))]
    pub denominator: u32,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "status", rename_all = "camelCase", deny_unknown_fields)]
pub enum PhysicalPixelSize {
    /// No absolute unit is present. Aspect-only declarations stay in the list.
    Unspecified,
    /// A declared density has zero magnitude and cannot define physical size.
    ZeroDensity,
    /// Absolute densities or relative axis ratios disagree; no source wins.
    Conflicting,
    Known {
        x: PixelExtent,
        y: PixelExtent,
    },
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ImageResolution {
    /// Raw fields in encoded axes. Missing EXIF fields remain null; derivation
    /// uses Exif's specified defaults (72, 72, inch), never a host DPI default.
    pub declarations: Vec<ResolutionDeclaration>,
    pub physical_pixel_size: PhysicalPixelSize,
}

fn extent(d: Density, unit: ResolutionUnit) -> PixelExtent {
    let emu = match unit {
        ResolutionUnit::Inch => 914_400,
        ResolutionUnit::Centimetre => 360_000,
        ResolutionUnit::Metre => 36_000_000,
        ResolutionUnit::AspectRatio => unreachable!("absolute unit required"),
    };
    // Maximum is 36,000,000 * u32::MAX: safely within i64, but not JS Number.
    let mut numerator = emu * u64::from(d.denominator);
    let mut denominator = u64::from(d.numerator);
    let (mut a, mut b) = (numerator, denominator);
    while b != 0 {
        (a, b) = (b, a % b);
    }
    numerator /= a;
    denominator /= a;
    PixelExtent {
        numerator: Emu::new(numerator as i64),
        denominator: denominator as u32,
    }
}
fn resolve(
    declarations: &[ResolutionDeclaration],
    orientation: u32,
) -> Result<PhysicalPixelSize, ImageError> {
    use PhysicalPixelSize::*;
    let mut known = None;
    let mut ratio = None;
    let (mut zero, mut conflict) = (false, false);
    for d in declarations {
        let defaults = Density {
            numerator: 72,
            denominator: 1,
        };
        let values = if d.source == ResolutionSource::ExifIfd0 {
            (
                Some(d.x.unwrap_or(defaults)),
                Some(d.y.unwrap_or(defaults)),
                Some(d.unit.unwrap_or(ResolutionUnit::Inch)),
            )
        } else {
            (d.x, d.y, d.unit)
        };
        let (Some(x), Some(y), Some(unit)) = values else {
            return Err(ImageError::Invalid("incomplete resolution declaration"));
        };
        if x.numerator == 0 || y.numerator == 0 {
            zero = true;
            continue;
        }
        // x/y density ratio. Cross products of four u32 factors fit u128.
        let r = (
            u128::from(x.numerator) * u128::from(y.denominator),
            u128::from(y.numerator) * u128::from(x.denominator),
        );
        if let Some((a, b)) = ratio {
            conflict |= a * r.1 != b * r.0;
        } else {
            ratio = Some(r);
        }
        if unit != ResolutionUnit::AspectRatio {
            let pair = (extent(x, unit), extent(y, unit));
            if let Some(previous) = known {
                conflict |= previous != pair;
            } else {
                known = Some(pair);
            }
        }
    }
    // All declarations are retained, including when several issues coexist.
    // This priority is deterministic and never chooses between source formats.
    Ok(if conflict {
        Conflicting
    } else if zero {
        ZeroDensity
    } else if let Some((x, y)) = known {
        let (x, y) = if orientation >= 5 { (y, x) } else { (x, y) };
        Known { x, y }
    } else {
        Unspecified
    })
}
pub(crate) fn read(
    bytes: &[u8],
    format: ImageFormat,
    orientation: u32,
    check: &dyn Fn() -> bool,
) -> Result<ImageResolution, ImageError> {
    let declarations = parse::read(bytes, format, check)?;
    Ok(ImageResolution {
        physical_pixel_size: resolve(&declarations, orientation)?,
        declarations,
    })
}

#[cfg(test)]
mod tests;
