//! Outward integer intervals at Q96. Only bounded, typed author values enter.
//! The extra fractional bits preserve author transform uncertainty separately
//! from the Q32 evaluated Draw IR; no platform transcendental functions.
use crate::CompileError;
use mo_geometry::Fixed;
use num_bigint::BigInt;

const BITS: usize = 96;
#[derive(Clone, Debug)]
pub(crate) struct Interval {
    pub lo: BigInt,
    pub hi: BigInt,
}
fn ceil_shift(n: BigInt, bits: usize) -> BigInt {
    -((-n) >> bits)
}
impl Interval {
    pub fn integer(n: i64) -> Self {
        let v = BigInt::from(n) << BITS;
        Self {
            lo: v.clone(),
            hi: v,
        }
    }
    pub fn fixed(n: Fixed) -> Self {
        let v = BigInt::from(n.raw()) << 64usize;
        Self::raw(v.clone(), v)
    }
    pub fn upper_q32(&self) -> Result<Fixed, CompileError> {
        i128::try_from(ceil_shift(self.hi.clone(), 64))
            .map(Fixed::from_raw)
            .map_err(|_| CompileError::Range)
    }
    pub fn raw(lo: BigInt, hi: BigInt) -> Self {
        debug_assert!(lo <= hi);
        Self { lo, hi }
    }
    pub fn add(&self, b: &Self) -> Self {
        Self::raw(&self.lo + &b.lo, &self.hi + &b.hi)
    }
    pub fn neg(&self) -> Self {
        Self::raw(-&self.hi, -&self.lo)
    }
    pub fn sub(&self, b: &Self) -> Self {
        self.add(&b.neg())
    }
    pub fn mul(&self, b: &Self) -> Self {
        let p = [
            &self.lo * &b.lo,
            &self.lo * &b.hi,
            &self.hi * &b.lo,
            &self.hi * &b.hi,
        ];
        let lo = p.iter().min().expect("four products") >> BITS;
        let hi = ceil_shift(p.iter().max().expect("four products").clone(), BITS);
        Self::raw(lo, hi)
    }
    pub fn divide(&self, d: i64) -> Self {
        assert!(d > 0, "internal positive interval divisor");
        fn floor(n: &BigInt, d: i64) -> BigInt {
            let q = n / d;
            if n < &BigInt::from(0) && n % d != BigInt::from(0) {
                q - 1
            } else {
                q
            }
        }
        Self::raw(floor(&self.lo, d), -floor(&(-&self.hi), d))
    }
    pub fn ratio(n: i64, d: i64) -> Self {
        Self::integer(n).divide(d)
    }
    pub fn signed(self, negative: bool) -> Self {
        if negative { self.neg() } else { self }
    }
    /// Nearest midpoint, ties away from zero, plus outward maximum deviation.
    /// A caller must carry the returned bound into its geometry/raster budget.
    pub fn q32(&self) -> Result<(Fixed, Fixed), CompileError> {
        let sum = &self.lo + &self.hi;
        let divisor = BigInt::from(1) << 65usize;
        let half = &divisor >> 1usize;
        let q = if sum < BigInt::from(0) {
            -((-sum + half) / &divisor)
        } else {
            (sum + half) / &divisor
        };
        let raw = i128::try_from(q.clone()).map_err(|_| CompileError::Range)?;
        let scaled = q << 64usize;
        let distance = (&scaled - &self.lo).max(&self.hi - &scaled);
        let bound = i128::try_from(ceil_shift(distance, 64)).map_err(|_| CompileError::Range)?;
        Ok((Fixed::from_raw(raw), Fixed::from_raw(bound)))
    }
}
