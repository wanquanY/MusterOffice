//! Propagate path and affine input uncertainty into the device pixel budget.
use crate::interval::Interval as I;
use mo_geometry::{Affine, Fixed, PathCommand as C, Point};
use mo_raster::{RasterError, RasterViewport};
use num_bigint::BigInt;
const ZERO: Point = Point {
    x: Fixed::ZERO,
    y: Fixed::ZERO,
};
/// Shared page-path budget in local Q32 EMU. Geometry compilation reserves a
/// quarter of the device tolerance; curve interpolation uses a quarter of this
/// local budget, leaving room for numeric conversion and downstream lowering.
pub(crate) fn local_tolerance<'a>(
    placements: impl Iterator<Item = (&'a Affine, &'a crate::AffineUncertainty)>,
    viewport: &RasterViewport,
    check: &dyn Fn() -> bool,
) -> Result<Fixed, RasterError> {
    let mut norm = BigInt::from(1u64 << 32);
    for (affine, uncertainty) in placements {
        if check() {
            return Err(RasterError::Cancelled);
        }
        for row in 0..2 {
            let mut value = BigInt::from(0);
            for col in 0..2 {
                let i = 2 * row + col;
                let a = BigInt::from(affine.linear[i].raw());
                value += if a < BigInt::from(0) { -a } else { a };
                value += uncertainty.linear[i].raw();
            }
            norm = norm.max(value);
        }
    }
    let raw = ((BigInt::from(viewport.coordinate_tolerance.raw()) * viewport.scale.denominator)
        << 32usize)
        / (norm * viewport.scale.numerator * 4u32);
    let raw = i128::try_from(raw).map_err(|_| RasterError::Range)?;
    if raw <= 0 {
        return Err(RasterError::Precision);
    }
    Ok(Fixed::from_raw(raw))
}

pub(crate) fn curve_fits_local(error: [Fixed; 2], tolerance: Fixed) -> bool {
    error.iter().all(|v| v.raw() <= tolerance.raw() / 4)
}
fn absolute(v: Fixed) -> I {
    let value = I::fixed(v);
    if v.raw() < 0 { value.neg() } else { value }
}
fn pixels(value: &I, v: &RasterViewport) -> Result<Fixed, RasterError> {
    value
        .mul(&I::integer(i64::from(v.scale.numerator)))
        .divide(i64::from(v.scale.denominator))
        .upper_q32()
        .map_err(|_| RasterError::Range)
}
pub(crate) fn axes(p: Point) -> [Fixed; 2] {
    [p.x, p.y]
}
fn points(c: &C) -> ([Point; 3], usize) {
    match *c {
        C::Move { to } | C::Line { to } => ([to, ZERO, ZERO], 1),
        C::Quadratic { control, to } => ([control, to, ZERO], 2),
        C::Cubic {
            control1,
            control2,
            to,
        } => ([control1, control2, to], 3),
        C::Close => ([ZERO; 3], 0),
    }
}
pub(crate) fn geometry_budget(
    error: [I; 2],
    affine: &Affine,
    uncertainty: &crate::AffineUncertainty,
    v: &RasterViewport,
) -> Result<Fixed, RasterError> {
    let mut result = Fixed::ZERO;
    for row in 0..2 {
        let terms: [I; 2] = std::array::from_fn(|i| {
            absolute(affine.linear[2 * row + i])
                .add(&I::fixed(uncertainty.linear[2 * row + i]))
                .mul(&error[i])
        });
        result = result.max(pixels(&terms[0].add(&terms[1]), v)?);
    }
    Ok(result)
}
pub(crate) fn matrix_budget(
    commands: &[C],
    uncertainty: &crate::AffineUncertainty,
    v: &RasterViewport,
    check: &dyn Fn() -> bool,
) -> Result<Fixed, RasterError> {
    matrix_budget_at(commands, ZERO, uncertainty, v, check)
}
pub(crate) fn matrix_budget_at(
    commands: &[C],
    origin: Point,
    uncertainty: &crate::AffineUncertainty,
    v: &RasterViewport,
    check: &dyn Fn() -> bool,
) -> Result<Fixed, RasterError> {
    let mut result = Fixed::ZERO;
    for command in commands {
        if check() {
            return Err(RasterError::Cancelled);
        }
        let (points, n) = points(command);
        for point in &points[..n] {
            let point = point.translate(origin).map_err(|_| RasterError::Range)?;
            for row in 0..2 {
                let mut error = I::fixed(axes(uncertainty.translation)[row]);
                for (i, x) in axes(point).iter().enumerate() {
                    error =
                        error.add(&I::fixed(uncertainty.linear[2 * row + i]).mul(&absolute(*x)));
                }
                result = result.max(pixels(&error, v)?);
            }
        }
    }
    Ok(result)
}
