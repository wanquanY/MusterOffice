use crate::{Affine, RenderError, Result};
/// Canonical inverse raster coefficients: signed Q32.32, nearest-ties-even.
/// Evaluating at a pixel center uses (2*x+1) and yields signed Q30.33 positions.
/// Both CPU and GPU use these integers for coverage and nearest-pixel decisions;
/// float conversion happens only for the local bilinear fractional weights.
#[derive(Debug, Clone, Copy)]
pub struct RasterCoordinates {
    coefficients: [i64; 6],
}
impl RasterCoordinates {
    pub fn new(inverse: Affine, canvas: (u32, u32)) -> Result<Self> {
        if canvas.0 == 0 || canvas.1 == 0 || canvas.0 > 16384 || canvas.1 > 16384 {
            return Err(RenderError::Invalid("raster coordinate extent"));
        }
        let mut coefficients = [0; 6];
        for (out, v) in coefficients.iter_mut().zip(inverse.coefficients()) {
            let fixed = (v * 4294967296.).round_ties_even();
            if !fixed.is_finite() || fixed <= i64::MIN as f64 || fixed >= i64::MAX as f64 {
                return Err(RenderError::Unsupported("raster coordinate precision"));
            }
            *out = fixed as i64;
        }
        let [a, b, c, d, x, y] = coefficients.map(i128::from);
        if a * d == b * c {
            return Err(RenderError::Unsupported("raster coordinate singularity"));
        }
        let w = i128::from(canvas.0 * 2 - 1);
        let h = i128::from(canvas.1 * 2 - 1);
        for bound in [
            a.abs() * w + c.abs() * h + x.abs() * 2,
            b.abs() * w + d.abs() * h + y.abs() * 2,
        ] {
            if bound > i128::from(i64::MAX) {
                return Err(RenderError::Unsupported("raster coordinate range"));
            }
        }
        Ok(Self { coefficients })
    }
    pub fn coefficients(self) -> [i64; 6] {
        self.coefficients
    }
    // Caller loops within the admitted canvas. The triangle bound above proves
    // every product and sum fits, including negative/off-canvas coordinates.
    pub(crate) fn position(self, x: u32, y: u32) -> (i64, i64) {
        let [a, b, c, d, tx, ty] = self.coefficients;
        let x = i64::from(x) * 2 + 1;
        let y = i64::from(y) * 2 + 1;
        (a * x + c * y + tx * 2, b * x + d * y + ty * 2)
    }
}
pub(crate) const UNIT: i64 = 1 << 33;

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn centers_boundaries_mirrors_and_range_are_exact() {
        let c = RasterCoordinates::new(Affine::identity(), (4, 4)).unwrap();
        assert_eq!(c.position(0, 0), (UNIT / 2, UNIT / 2));
        assert_eq!(c.position(3, 3), (UNIT * 3 + UNIT / 2, UNIT * 3 + UNIT / 2));
        let c = RasterCoordinates::new(Affine::new([-1., 0., 0., 1., 4., -0.5]).unwrap(), (4, 4))
            .unwrap();
        assert_eq!(c.position(0, 0), (UNIT * 3 + UNIT / 2, 0));
        assert_eq!(c.position(3, 1), (UNIT / 2, UNIT));
        assert!(
            RasterCoordinates::new(Affine::new([1e12, 0., 0., 1., 0., 0.]).unwrap(), (4, 4))
                .is_err()
        );
        assert!(
            RasterCoordinates::new(Affine::new([1e-12, 0., 0., 1., 0., 0.]).unwrap(), (4, 4))
                .is_err()
        );
    }
}
