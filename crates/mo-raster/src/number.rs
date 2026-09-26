use crate::{PixelScale, RasterError};

pub(crate) struct Scale {
    n: i128,
    d: i128,
}
impl Scale {
    pub fn new(s: PixelScale) -> Result<Self, RasterError> {
        if s.numerator == 0 || s.denominator == 0 {
            return Err(RasterError::Invalid("zero pixel scale"));
        }
        let (mut a, mut b) = (s.numerator, s.denominator);
        while b != 0 {
            (a, b) = (b, a % b);
        }
        Ok(Self {
            n: i128::from(s.numerator / a),
            d: i128::from(s.denominator / a),
        })
    }
    fn numerator(&self, raw: i128) -> Result<i128, RasterError> {
        let n = raw.checked_mul(self.n).ok_or(RasterError::Range)?;
        if n.unsigned_abs() > (32768 * (self.d << 32)) as u128 {
            return Err(RasterError::Range);
        }
        Ok(n)
    }
    /// Exact rational -> normal float32, nearest ties-even. The bounded scale
    /// range ensures a nonzero magnitude >= 2^-64; no subnormals are needed.
    pub fn value(&self, raw: i128) -> Result<f32, RasterError> {
        let signed = self.numerator(raw)?;
        if signed == 0 {
            return Ok(0.0);
        }
        let n = signed.unsigned_abs();
        let d = (self.d << 32) as u128;
        let mut e = d.leading_zeros() as i32 - n.leading_zeros() as i32;
        let below = if e >= 0 { n < (d << e) } else { (n << -e) < d };
        if below {
            e -= 1;
        }
        let shifted = n << (23 - e);
        let (mut m, rem) = (shifted / d, shifted % d);
        if rem * 2 > d || (rem * 2 == d && m % 2 == 1) {
            m += 1;
        }
        if m == 1 << 24 {
            m >>= 1;
            e += 1;
        }
        let sign = if signed < 0 { 1 << 31 } else { 0 };
        Ok(f32::from_bits(
            sign | ((e + 127) as u32) << 23 | (m as u32 - (1 << 23)),
        ))
    }
    /// Outward Q32 intervals contain both exact rational and float32. Integer
    /// comparison overestimates the error by at most two raw Q32 units.
    pub fn error(&self, raw: i128, actual: f32) -> Result<i128, RasterError> {
        let n = self.numerator(raw)?;
        let lo = n.div_euclid(self.d);
        let hi = lo + i128::from(n.rem_euclid(self.d) != 0);
        let (a, b) = float_interval(actual);
        Ok((a - hi).abs().max((b - lo).abs()))
    }
}
pub(crate) fn float_interval(v: f32) -> (i128, i128) {
    if v == 0.0 {
        return (0, 0);
    }
    let bits = v.to_bits();
    let e = ((bits >> 23) & 255) as i32 - 127;
    let m = i128::from((bits & 0x7fffff) | (1 << 23));
    let shift = e + 9;
    let (lo, hi) = if shift >= 0 {
        (m << shift, m << shift)
    } else {
        let d = 1i128 << -shift;
        (m / d, m / d + i128::from(m % d != 0))
    };
    if bits >> 31 != 0 {
        (-hi, -lo)
    } else {
        (lo, hi)
    }
}
pub(crate) fn add_sub(a: i128, b: i128, c: i128) -> Result<i128, RasterError> {
    a.checked_sub(c)
        .and_then(|v| v.checked_add(b))
        .or_else(|| b.checked_sub(c).and_then(|v| v.checked_add(a)))
        .or_else(|| a.checked_add(b).and_then(|v| v.checked_sub(c)))
        .ok_or(RasterError::Range)
}
