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

/// Fit inside the host's physical pixel box using exact document extents.
/// Only allocation dimensions round up; the uniform page scale stays rational.
pub fn fit_page_viewport(
    size: Size,
    width: u32,
    height: u32,
    coordinate_tolerance: Fixed,
    background: [u8; 4],
) -> Result<RasterViewport, RasterError> {
    let w = u64::try_from(size.width.get()).map_err(|_| RasterError::Invalid("page width"))?;
    let h = u64::try_from(size.height.get()).map_err(|_| RasterError::Invalid("page height"))?;
    if w == 0 || h == 0 || width == 0 || height == 0 {
        return Err(RasterError::Invalid("page or viewport extent"));
    }
    let (width, height) = (width.min(8192), height.min(8192));
    let (extent, maximum) =
        if u128::from(width) * u128::from(h) <= u128::from(height) * u128::from(w) {
            (w, width)
        } else {
            (h, height)
        };
    let dimensions = |n: u32| {
        (
            (u128::from(w) * u128::from(n)).div_ceil(u128::from(extent)),
            (u128::from(h) * u128::from(n)).div_ceil(u128::from(extent)),
        )
    };
    // At most 13 iterations, including square 8K displays whose area exceeds
    // the renderer's allocation limit. Never enlarge a rounded previous frame.
    let (mut low, mut high) = (1, maximum);
    while low < high {
        let mid = low + (high - low).div_ceil(2);
        let (w, h) = dimensions(mid);
        if w * h * 4 <= mo_raster::MAX_PIXEL_BYTES as u128 {
            low = mid;
        } else {
            high = mid - 1;
        }
    }
    let (w, h) = dimensions(low);
    let (mut a, mut b) = (extent, u64::from(low));
    while b != 0 {
        (a, b) = (b, a % b);
    }
    let viewport = RasterViewport {
        width: u32::try_from(w).map_err(|_| RasterError::Range)?,
        height: u32::try_from(h).map_err(|_| RasterError::Range)?,
        origin: Point {
            x: Fixed::ZERO,
            y: Fixed::ZERO,
        },
        scale: mo_raster::PixelScale {
            numerator: (u64::from(low) / a) as u32,
            denominator: u32::try_from(extent / a)
                .map_err(|_| RasterError::Limit("page scale denominator"))?,
        },
        coordinate_tolerance,
        background,
    };
    viewport.validate()?;
    validate(size, &viewport)?;
    Ok(viewport)
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

#[cfg(test)]
mod fit_tests {
    use super::*;
    use mo_common::Emu;
    #[test]
    fn fit_preserves_exact_geometry_and_honors_both_axes_and_pixel_budget() {
        for (w, h) in [(16, 9), (9, 16), (7_999_730, 4_499_610), (1, 1)] {
            let size = Size {
                width: Emu::new(w),
                height: Emu::new(h),
            };
            for (width, height) in [
                (1, 1),
                (127, 83),
                (83, 127),
                (8192, 8192),
                (u32::MAX, u32::MAX),
            ] {
                let v = fit_page_viewport(size, width, height, Fixed::from_raw(1 << 20), [0; 4])
                    .unwrap();
                assert!(v.width <= width && v.height <= height);
                assert!(u64::from(v.width) * u64::from(v.height) <= 16777216);
                validate(size, &v).unwrap();
                let again =
                    fit_page_viewport(size, width, height, v.coordinate_tolerance, v.background)
                        .unwrap();
                assert_eq!(
                    serde_json::to_value(v).unwrap(),
                    serde_json::to_value(again).unwrap()
                );
            }
        }
    }
    #[test]
    fn fit_rejects_empty_boxes_and_unrepresentable_scale() {
        for (w, h, width, height) in [
            (0, 1, 10, 10),
            (1, 1, 0, 10),
            (1, 1, 10, 0),
            (i64::MAX, 1, 1, 1),
        ] {
            assert!(
                fit_page_viewport(
                    Size {
                        width: Emu::new(w),
                        height: Emu::new(h)
                    },
                    width,
                    height,
                    Fixed::from_raw(1 << 20),
                    [0; 4]
                )
                .is_err()
            );
        }
    }
}
