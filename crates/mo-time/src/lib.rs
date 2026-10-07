//! Bounded exact arithmetic shared by office time domains. No clocks, I/O or media state.
mod interval;
mod wire;
pub use interval::{ExactInterval, SampleSide};
use mo_common::RationalTime;
use num_bigint::BigInt;
use std::cmp::Ordering;

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum TimeError {
    #[error("rational denominator must be positive")]
    Denominator,
    #[error("exact arithmetic exceeds its bit budget (1..=4096)")]
    Budget,
    #[error("division by zero")]
    DivisionByZero,
    #[error("exact time cannot be represented by the requested wire unit")]
    Representation,
    #[error("exact interval must have strictly increasing endpoints")]
    Interval,
}

/// Equality preserves the input representation; compare() compares instants.
/// Arithmetic results are reduced. Private fields prevent invalid denominators.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExactRatio {
    n: BigInt,
    d: BigInt,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Rounding {
    Floor,
    Ceil,
    TowardZero,
    NearestTiesEven,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QuantizedTime {
    pub value: RationalTime,
    /// Exact original value minus the quantized value; never silently discarded.
    pub residual: ExactRatio,
}

impl ExactRatio {
    pub fn new(n: BigInt, d: BigInt, bits: u64) -> Result<Self, TimeError> {
        if d <= BigInt::from(0) {
            return Err(TimeError::Denominator);
        }
        Self::budget(&n, &d, bits)?;
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
    fn budget(n: &BigInt, d: &BigInt, bits: u64) -> Result<(), TimeError> {
        if !(1..=4096).contains(&bits) || n.bits() > bits || d.bits() > bits {
            return Err(TimeError::Budget);
        }
        Ok(())
    }
    fn operands(&self, b: &Self, bits: u64) -> Result<(), TimeError> {
        Self::budget(&self.n, &self.d, bits)?;
        Self::budget(&b.n, &b.d, bits)
    }
    pub fn from_time(t: RationalTime) -> Self {
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
    pub fn numerator(&self) -> &BigInt {
        &self.n
    }
    pub fn denominator(&self) -> &BigInt {
        &self.d
    }
    pub fn check_budget(&self, bits: u64) -> Result<(), TimeError> {
        Self::budget(&self.n, &self.d, bits)
    }
    pub fn add(&self, b: &Self, bits: u64) -> Result<Self, TimeError> {
        self.operands(b, bits)?;
        Self::new(&self.n * &b.d + &b.n * &self.d, &self.d * &b.d, bits)
    }
    pub fn sub(&self, b: &Self, bits: u64) -> Result<Self, TimeError> {
        self.operands(b, bits)?;
        Self::new(&self.n * &b.d - &b.n * &self.d, &self.d * &b.d, bits)
    }
    pub fn mul(&self, b: &Self, bits: u64) -> Result<Self, TimeError> {
        self.operands(b, bits)?;
        Self::new(&self.n * &b.n, &self.d * &b.d, bits)
    }
    pub fn div(&self, b: &Self, bits: u64) -> Result<Self, TimeError> {
        self.operands(b, bits)?;
        if b.n == BigInt::from(0) {
            return Err(TimeError::DivisionByZero);
        }
        let (n, d) = (&self.n * &b.d, &self.d * &b.n);
        if d < BigInt::from(0) {
            Self::new(-n, -d, bits)
        } else {
            Self::new(n, d, bits)
        }
    }
    pub fn compare(&self, b: &Self) -> Ordering {
        (&self.n * &b.d).cmp(&(&b.n * &self.d))
    }
    pub fn positive(&self) -> bool {
        self.n > BigInt::from(0)
    }
    /// Signed remainder after truncation toward zero, not Euclidean modulo.
    pub fn fraction(&self, bits: u64) -> Result<Self, TimeError> {
        Self::budget(&self.n, &self.d, bits)?;
        Self::new(&self.n % &self.d, self.d.clone(), bits)
    }
    pub fn normalized(&self) -> Self {
        Self::new(
            self.n.clone(),
            self.d.clone(),
            self.n.bits().max(self.d.bits()).max(1),
        )
        .expect("bounded exact ratio invariant")
    }
    pub fn to_time_exact(&self) -> Result<RationalTime, TimeError> {
        let v = self.normalized();
        let ticks = i64::try_from(v.n).map_err(|_| TimeError::Representation)?;
        let scale = u32::try_from(v.d).map_err(|_| TimeError::Representation)?;
        RationalTime::new(ticks, scale).map_err(|_| TimeError::Representation)
    }
    pub fn round_integer(&self, rounding: Rounding) -> Result<i64, TimeError> {
        let mut q = &self.n / &self.d;
        let r = &self.n % &self.d;
        if r != BigInt::from(0) {
            let negative = r < BigInt::from(0);
            let step = if negative { -1 } else { 1 };
            let adjust = match rounding {
                Rounding::TowardZero => false,
                Rounding::Floor => negative,
                Rounding::Ceil => !negative,
                Rounding::NearestTiesEven => {
                    let twice = (if negative { -r } else { r }) * 2;
                    twice > self.d || (twice == self.d && &q % 2 != BigInt::from(0))
                }
            };
            if adjust {
                q += step;
            }
        }
        i64::try_from(q).map_err(|_| TimeError::Representation)
    }
    /// Convert an absolute instant at a device/API boundary, preserving the exact error.
    pub fn quantize(
        &self,
        scale: u32,
        rounding: Rounding,
        bits: u64,
    ) -> Result<QuantizedTime, TimeError> {
        RationalTime::new(0, scale).map_err(|_| TimeError::Representation)?;
        let ticks = self
            .mul(&Self::integer(i64::from(scale)), bits)?
            .round_integer(rounding)?;
        let value = RationalTime::new(ticks, scale).map_err(|_| TimeError::Representation)?;
        let residual = self.sub(&Self::from_time(value), bits)?;
        Ok(QuantizedTime { value, residual })
    }
    /// Absolute output-frame time: n * frame-rate denominator / numerator.
    pub fn frame_time(
        index: i64,
        numerator: u32,
        denominator: u32,
        bits: u64,
    ) -> Result<Self, TimeError> {
        if numerator == 0 || denominator == 0 {
            return Err(TimeError::Denominator);
        }
        Self::new(BigInt::from(index) * denominator, numerator.into(), bits)
    }
}
