use crate::{Fixed, GeometryError, Point, wide};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
const ONE: Fixed = Fixed::from_raw(1 << 32);
const ZERO: Point = Point {
    x: Fixed::ZERO,
    y: Fixed::ZERO,
};

/// Evaluated affine transform, not author rotation/flip/viewport semantics.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Affine {
    /// Dimensionless Q32 in row-major order: xx, xy, yx, yy.
    pub linear: [Fixed; 4],
    /// Q32 in the same coordinate unit as the input point.
    pub translation: Point,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PointEstimate {
    pub point: Point,
    /// Nonnegative outward error per axis, raw Q32 coordinate units.
    pub error: Point,
}
impl Affine {
    pub const IDENTITY: Self = Self {
        linear: [ONE, Fixed::ZERO, Fixed::ZERO, ONE],
        translation: ZERO,
    };
    pub fn map(self, point: Point) -> Result<PointEstimate, GeometryError> {
        self.map_estimate(PointEstimate { point, error: ZERO })
    }
    /// Apply only the linear part to p-origin, with exact wide subtraction.
    pub fn map_vector_from(self, p: Point, origin: Point) -> Result<Point, GeometryError> {
        let a = self.linear.map(Fixed::raw);
        Ok(Point {
            x: Fixed::from_raw(
                wide::dot_delta(
                    a[0],
                    p.x.raw(),
                    origin.x.raw(),
                    a[1],
                    p.y.raw(),
                    origin.y.raw(),
                )?
                .0,
            ),
            y: Fixed::from_raw(
                wide::dot_delta(
                    a[2],
                    p.x.raw(),
                    origin.x.raw(),
                    a[3],
                    p.y.raw(),
                    origin.y.raw(),
                )?
                .0,
            ),
        })
    }
    pub fn map_estimate(self, p: PointEstimate) -> Result<PointEstimate, GeometryError> {
        self.map_in_view(p, ZERO)
    }
    /// Subtract the viewport in the exact expression, before narrowing the root
    /// coordinate. A huge common document origin must not cause false overflow.
    pub fn map_in_view(
        self,
        p: PointEstimate,
        origin: Point,
    ) -> Result<PointEstimate, GeometryError> {
        let a = self.linear.map(Fixed::raw);
        let values = [p.point.x.raw(), p.point.y.raw()];
        let errors = [p.error.x.raw(), p.error.y.raw()];
        let mut out = [Fixed::ZERO; 2];
        let mut bounds = [Fixed::ZERO; 2];
        for k in 0..2 {
            let (v, rounded) = wide::dot_in_view(
                a[2 * k],
                values[0],
                a[2 * k + 1],
                values[1],
                [self.translation.x.raw(), self.translation.y.raw()][k],
                [origin.x.raw(), origin.y.raw()][k],
            )?;
            out[k] = Fixed::from_raw(v);
            bounds[k] = Fixed::from_raw(
                wide::error_up(a[2 * k], errors[0], a[2 * k + 1], errors[1])?
                    .checked_add(i128::from(rounded))
                    .ok_or(GeometryError::Numeric)?,
            );
        }
        Ok(PointEstimate {
            point: Point {
                x: out[0],
                y: out[1],
            },
            error: Point {
                x: bounds[0],
                y: bounds[1],
            },
        })
    }
    pub fn rebase(self, origin: Point) -> Result<Self, GeometryError> {
        Ok(Self {
            linear: self.linear,
            translation: Point {
                x: self.translation.x.checked_sub(origin.x)?,
                y: self.translation.y.checked_sub(origin.y)?,
            },
        })
    }
    /// Quantized matrix for reuse. For certified results, compare evaluated
    /// points to the original transform chain; coefficient error can amplify.
    pub fn compose(self, inner: Self) -> Result<Self, GeometryError> {
        let a = self.linear.map(Fixed::raw);
        let b = inner.linear.map(Fixed::raw);
        let mut c = [Fixed::ZERO; 4];
        for row in 0..2 {
            for col in 0..2 {
                c[row * 2 + col] = Fixed::from_raw(
                    wide::dot(a[row * 2], b[col], a[row * 2 + 1], b[2 + col], 0)?.0,
                );
            }
        }
        Ok(Self {
            linear: c,
            translation: self.map(inner.translation)?.point,
        })
    }
}
impl Fixed {
    /// Ceil of a nonnegative fixed quantity multiplied by an unsigned ratio.
    pub fn ratio_up(self, numerator: u32, denominator: u32) -> Result<Self, GeometryError> {
        wide::ratio_up(self.raw(), numerator, denominator).map(Self::from_raw)
    }
    /// |a+b-c| without losing a representable answer to intermediate overflow.
    pub fn sum_deviation(a: Self, b: Self, c: Self) -> Result<Self, GeometryError> {
        wide::deviation(a.raw(), b.raw(), c.raw()).map(Self::from_raw)
    }
}
