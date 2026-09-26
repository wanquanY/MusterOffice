//! Outward operations used by native elliptic paths. Finite binary64 inputs are
//! decoded as dyadic rationals, never multiplied or rounded by the host FPU.
use crate::{CompileError, interval::Interval as I};
use num_bigint::BigInt;

pub(crate) fn floor_ratio(n: &BigInt, d: &BigInt) -> BigInt {
    debug_assert!(d > &BigInt::from(0));
    let q = n / d;
    if n < &BigInt::from(0) && n % d != BigInt::from(0) {
        q - 1
    } else {
        q
    }
}
impl I {
    pub fn binary64(n: f64) -> Result<Self, CompileError> {
        if !n.is_finite() {
            return Err(CompileError::Range);
        }
        let bits = n.to_bits();
        let exponent = ((bits >> 52) & 2047) as i32;
        let fraction = bits & ((1u64 << 52) - 1);
        let (mantissa, shift) = if exponent == 0 {
            (fraction, -1074 + 96)
        } else {
            (fraction | (1u64 << 52), exponent - 1023 - 52 + 96)
        };
        let mut value = BigInt::from(mantissa);
        if bits >> 63 != 0 {
            value = -value;
        }
        Ok(if shift >= 0 {
            value <<= shift as usize;
            Self::raw(value.clone(), value)
        } else {
            let shift = (-shift) as usize;
            Self::raw(&value >> shift, -((-value) >> shift))
        })
    }
    pub fn div_positive(&self, divisor: &Self) -> Result<Self, CompileError> {
        if divisor.lo <= BigInt::from(0) {
            return Err(CompileError::Range);
        }
        // With a positive denominator, choose each extremum by numerator sign.
        // This avoids four divisions and temporary vectors per interval ratio.
        let low_denominator = if self.lo < BigInt::from(0) {
            &divisor.lo
        } else {
            &divisor.hi
        };
        let high_denominator = if self.hi < BigInt::from(0) {
            &divisor.hi
        } else {
            &divisor.lo
        };
        Ok(Self::raw(
            floor_ratio(&(&self.lo << 96usize), low_denominator),
            -floor_ratio(&(-&self.hi << 96usize), high_denominator),
        ))
    }
    pub fn sqrt_nonnegative(&self) -> Result<Self, CompileError> {
        if self.lo < BigInt::from(0) {
            return Err(CompileError::Range);
        }
        let lo = (&self.lo << 96usize).sqrt();
        let n = &self.hi << 96usize;
        let mut hi = n.sqrt();
        if &hi * &hi < n {
            hi += 1;
        }
        Ok(Self::raw(lo, hi))
    }
    pub fn abs_upper(&self) -> Self {
        let hi = self.hi.clone().max(-&self.lo);
        Self::raw(BigInt::from(0), hi)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn binary64_decode_keeps_exact_dyadics_and_encloses_subnormals() {
        let a = I::binary64(0.1).unwrap();
        assert_eq!(a.lo, BigInt::from(3602879701896397u64) << 41usize);
        assert_eq!(a.lo, a.hi);
        for (value, lo, hi) in [
            (f64::from_bits(1), 0, 1),
            (-f64::from_bits(1), -1, 0),
            (-0.0, 0, 0),
        ] {
            let a = I::binary64(value).unwrap();
            assert_eq!(a.lo, BigInt::from(lo));
            assert_eq!(a.hi, BigInt::from(hi));
        }
        assert!(I::binary64(f64::INFINITY).is_err());
        let value = I::binary64(f64::MAX).unwrap();
        assert_eq!(value.lo, value.hi);
    }
    #[test]
    fn interval_ratio_and_square_root_round_both_sides_outward() {
        let root = I::integer(2).sqrt_nonnegative().unwrap();
        let exact = BigInt::from(2) << 192usize;
        assert!(&root.lo * &root.lo <= exact);
        assert!(&root.hi * &root.hi >= exact);
        assert_eq!(&root.hi - &root.lo, BigInt::from(1));
        let range = I::raw(I::integer(-3).lo, I::integer(2).hi);
        let divisor = I::raw(I::integer(2).lo, I::integer(3).hi);
        let ratio = range.div_positive(&divisor).unwrap();
        assert_eq!(ratio.lo, I::ratio(-3, 2).lo);
        assert_eq!(ratio.hi, I::integer(1).hi);
        assert!(range.sqrt_nonnegative().is_err());
        assert!(range.div_positive(&I::integer(0)).is_err());
    }
}
