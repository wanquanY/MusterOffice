use crate::{
    source_page::{SourcePageError, SourcePagePaintBinding},
    source_text_page::placement,
};
use mo_geometry::{Affine, Fixed, GeometryError, PathCommand, Point};
use mo_raster::RasterViewport;
use num_bigint::BigInt;

pub(super) struct Transform<'a> {
    pub object: &'a SourcePagePaintBinding,
    pub viewport: &'a RasterViewport,
    pub uncertainty: Fixed,
}
impl Transform<'_> {
    /// Uses the exact same rebased affine and precision certificate as glyph
    /// placement and native overflow masks. Points remain in page EMU.
    pub fn points<const N: usize>(
        &self,
        points: [Point; N],
        check: &dyn Fn() -> bool,
    ) -> Result<([Point; N], Fixed), SourcePageError> {
        let commands: Vec<_> = points
            .iter()
            .enumerate()
            .map(|(i, &to)| {
                if i == 0 {
                    PathCommand::Move { to }
                } else {
                    PathCommand::Line { to }
                }
            })
            .collect();
        let (affine, position, geometry) = placement::transform(
            &commands,
            Point {
                x: Fixed::ZERO,
                y: Fixed::ZERO,
            },
            self.uncertainty,
            self.object,
            self.viewport,
            check,
        )?;
        let mut error = Fixed::ZERO;
        let mut result = points;
        for point in &mut result {
            if check() {
                return Err(mo_raster::RasterError::Cancelled.into());
            }
            let p = affine.map(*point)?;
            *point = p.point;
            error = error.max(p.error.x).max(p.error.y);
        }
        let error = position
            .checked_add(geometry)?
            .ratio_up(
                self.viewport.scale.denominator,
                self.viewport.scale.numerator,
            )?
            .checked_add(error)?;
        Ok((result, error))
    }
    pub fn inverse(&self, point: Point) -> Result<Option<Point>, SourcePageError> {
        let placement = self
            .object
            .placement
            .as_ref()
            .ok_or(SourcePageError::Invalid("text interaction placement"))?;
        let origin = Point {
            x: Fixed::ZERO.checked_sub(placement.anchor.x)?,
            y: Fixed::ZERO.checked_sub(placement.anchor.y)?,
        };
        let affine = Affine {
            linear: placement.affine.linear,
            translation: placement.affine.map(origin)?.point,
        };
        inverse(affine, point).map_err(Into::into)
    }
}
fn rounded(mut n: BigInt, mut d: BigInt) -> Result<Fixed, GeometryError> {
    if d < BigInt::from(0) {
        n = -n;
        d = -d;
    }
    let mut q = &n / &d;
    let r = &n % &d;
    let magnitude = if r < BigInt::from(0) { -r } else { r };
    if magnitude * 2 >= d {
        q += if n < BigInt::from(0) { -1 } else { 1 };
    }
    i128::try_from(q)
        .map(Fixed::from_raw)
        .map_err(|_| GeometryError::Numeric)
}
/// Inverse viewport mapping without first rounding a scale coefficient.
pub(super) fn page_point(viewport: &RasterViewport, point: Point) -> Result<Point, GeometryError> {
    let denominator = BigInt::from(viewport.scale.numerator);
    let numerator = BigInt::from(viewport.scale.denominator);
    let axis = |p: Fixed, origin: Fixed| {
        rounded(
            BigInt::from(p.raw()) * &numerator + BigInt::from(origin.raw()) * &denominator,
            denominator.clone(),
        )
    };
    Ok(Point {
        x: axis(point.x, viewport.origin.x)?,
        y: axis(point.y, viewport.origin.y)?,
    })
}
/// Invert the evaluated Q32 transform as one exact rational expression. Never
/// round inverse coefficients before multiplying a possibly large page point.
fn inverse(affine: Affine, point: Point) -> Result<Option<Point>, GeometryError> {
    let [a, b, c, d] = affine.linear.map(|v| BigInt::from(v.raw()));
    let det = &a * &d - &b * &c;
    if det == BigInt::from(0) {
        return Ok(None);
    }
    let x = BigInt::from(point.x.raw()) - BigInt::from(affine.translation.x.raw());
    let y = BigInt::from(point.y.raw()) - BigInt::from(affine.translation.y.raw());
    let scale = BigInt::from(1u64 << 32);
    Ok(Some(Point {
        x: rounded((&d * &x - &b * &y) * &scale, det.clone())?,
        y: rounded((&a * &y - &c * &x) * scale, det)?,
    }))
}
#[cfg(test)]
mod tests {
    use super::*;
    fn f(n: i128) -> Fixed {
        Fixed::from_raw(n << 32)
    }
    #[test]
    fn exact_inverse_handles_rotation_reflection_large_origins_and_singular_scales() {
        for linear in [[2, 0, 0, 3], [0, -2, 3, 0], [-2, 1, 0, 3]] {
            let a = Affine {
                linear: linear.map(f),
                translation: Point {
                    x: Fixed::from_raw(i128::MAX / 2),
                    y: Fixed::from_raw(i128::MIN / 2),
                },
            };
            let p = Point {
                x: f(17),
                y: f(-19),
            };
            assert_eq!(inverse(a, a.map(p).unwrap().point).unwrap(), Some(p));
        }
        let a = Affine {
            linear: [f(1), f(2), f(2), f(4)],
            ..Affine::IDENTITY
        };
        assert_eq!(
            inverse(
                a,
                Point {
                    x: f(100),
                    y: f(100)
                }
            )
            .unwrap(),
            None
        );
    }
}
