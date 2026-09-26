use crate::{ExactValue, TimelineError};
use mo_common::RationalTime;
use num_bigint::BigInt;
use std::cmp::Ordering;

/// Bounded exact intermediates, independent of source timebase and sample rate.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Ratio {
    pub n: BigInt,
    pub d: BigInt,
}
impl Ratio {
    pub fn new(n: BigInt, d: BigInt, bits: u64) -> Result<Self, TimelineError> {
        if d <= BigInt::from(0) {
            return Err(TimelineError::Limit("invalid rational denominator"));
        }
        if n.bits() > bits || d.bits() > bits {
            return Err(TimelineError::Limit("exact time arithmetic"));
        }
        let mut a = if n < BigInt::from(0) { -&n } else { n.clone() };
        let mut b = d.clone();
        while b != BigInt::from(0) {
            (a, b) = (b.clone(), a % b);
        }
        Ok(Self {
            n: n / &a,
            d: d / a,
        })
    }
    pub fn time(t: RationalTime) -> Self {
        Self {
            n: t.ticks.get().into(),
            d: t.timescale.get().into(),
        }
    }
    pub fn integer(n: i64) -> Self {
        Self {
            n: n.into(),
            d: 1.into(),
        }
    }
    pub fn add(&self, b: &Self, bits: u64) -> Result<Self, TimelineError> {
        Self::new(&self.n * &b.d + &b.n * &self.d, &self.d * &b.d, bits)
    }
    pub fn sub(&self, b: &Self, bits: u64) -> Result<Self, TimelineError> {
        Self::new(&self.n * &b.d - &b.n * &self.d, &self.d * &b.d, bits)
    }
    pub fn mul(&self, b: &Self, bits: u64) -> Result<Self, TimelineError> {
        Self::new(&self.n * &b.n, &self.d * &b.d, bits)
    }
    pub fn div(&self, b: &Self, bits: u64) -> Result<Self, TimelineError> {
        Self::new(&self.n * &b.d, &self.d * &b.n, bits)
    }
    pub fn cmp(&self, b: &Self) -> Ordering {
        (&self.n * &b.d).cmp(&(&b.n * &self.d))
    }
    pub fn positive(&self) -> bool {
        self.n > BigInt::from(0)
    }
    pub fn fraction(&self, bits: u64) -> Result<Self, TimelineError> {
        Self::new(&self.n % &self.d, self.d.clone(), bits)
    }
    pub fn wire(&self) -> ExactValue {
        // Every computed value is reduced; source/sample times are normalized here.
        let r = Self::new(
            self.n.clone(),
            self.d.clone(),
            self.n.bits().max(self.d.bits()).max(1),
        )
        .expect("bounded ratio");
        ExactValue {
            numerator: r.n.to_string(),
            denominator: r.d.to_string(),
        }
    }
}
