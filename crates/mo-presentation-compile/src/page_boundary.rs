//! Uniform page sampling rounds only the output allocation, never the page.
use mo_geometry::{Fixed, PathCommand as C, Point};
use mo_presentation_model::Size;
use mo_raster::{RasterError, RasterViewport};

/// Returns whether a fractional final pixel needs an explicit document clip.
/// Raster limits/scale validity are checked by the ordinary raster preflight.
pub(crate) fn validate(size: Size, viewport: &RasterViewport) -> Result<bool, RasterError> {
    if viewport.origin
        != (Point {
            x: Fixed::ZERO,
            y: Fixed::ZERO,
        })
    {
        return Err(RasterError::Invalid("page viewport origin must be zero"));
    }
    let denominator = i128::from(viewport.scale.denominator);
    let numerator = i128::from(viewport.scale.numerator);
    if denominator <= 0 || numerator <= 0 {
        return Err(RasterError::Invalid("page viewport scale"));
    }
    let mut fractional = false;
    for (extent, pixels) in [(size.width, viewport.width), (size.height, viewport.height)] {
        let scaled = i128::from(extent.get()) * numerator;
        if scaled <= 0 || (scaled + denominator - 1) / denominator != i128::from(pixels) {
            return Err(RasterError::Invalid(
                "page viewport must cover the ceiling of the uniformly scaled page",
            ));
        }
        fractional |= scaled % denominator != 0;
    }
    Ok(fractional)
}

pub(crate) fn path(size: Size) -> [C; 5] {
    let zero = Fixed::ZERO;
    let w = Fixed::emu(size.width);
    let h = Fixed::emu(size.height);
    [
        C::Move {
            to: Point { x: zero, y: zero },
        },
        C::Line {
            to: Point { x: w, y: zero },
        },
        C::Line {
            to: Point { x: w, y: h },
        },
        C::Line {
            to: Point { x: zero, y: h },
        },
        C::Close,
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use mo_common::Emu;
    use mo_raster::PixelScale;

    #[test]
    fn native_page_quantization_does_not_require_a_giant_integer_ratio_frame() {
        let mut v = RasterViewport {
            width: 800,
            height: 450,
            origin: Point {
                x: Fixed::ZERO,
                y: Fixed::ZERO,
            },
            scale: PixelScale {
                numerator: 1,
                denominator: 10000,
            },
            coordinate_tolerance: Fixed::from_raw(1 << 24),
            background: [0; 4],
        };
        let size = Size {
            width: Emu::new(7_999_730),
            height: Emu::new(4_499_610),
        };
        assert!(validate(size, &v).unwrap());
        assert!(
            !validate(
                Size {
                    width: Emu::new(8_000_000),
                    height: Emu::new(4_500_000)
                },
                &v
            )
            .unwrap()
        );
        v.width = 799;
        assert!(validate(size, &v).is_err());
        v.width = 801;
        assert!(validate(size, &v).is_err());
        v.width = 800;
        v.origin.x = Fixed::from_raw(1);
        assert!(validate(size, &v).is_err());
    }
}
