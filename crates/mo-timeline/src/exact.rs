use crate::{ExactValue, TimelineError};
use mo_common::RationalTime;
use mo_time::{ExactRatio, TimeError};
use num_bigint::BigInt;
use std::cmp::Ordering;

/// Presentation compatibility adapter; all arithmetic belongs to the shared time core.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Ratio(ExactRatio);
fn error(value: TimeError) -> TimelineError {
    TimelineError::Limit(match value {
        TimeError::Denominator | TimeError::DivisionByZero => "invalid rational denominator",
        TimeError::Budget | TimeError::Representation => "exact time arithmetic",
        TimeError::Interval => "invalid exact interval",
    })
}
impl Ratio {
    pub fn new(n: BigInt, d: BigInt, bits: u64) -> Result<Self, TimelineError> {
        ExactRatio::new(n, d, bits).map(Self).map_err(error)
    }
    pub fn time(t: RationalTime) -> Self {
        Self(ExactRatio::from_time(t))
    }
    pub fn integer(n: i64) -> Self {
        Self(ExactRatio::integer(n))
    }
    pub fn numerator(&self) -> &BigInt {
        self.0.numerator()
    }
    pub fn denominator(&self) -> &BigInt {
        self.0.denominator()
    }
    pub fn add(&self, b: &Self, bits: u64) -> Result<Self, TimelineError> {
        self.0.add(&b.0, bits).map(Self).map_err(error)
    }
    pub fn sub(&self, b: &Self, bits: u64) -> Result<Self, TimelineError> {
        self.0.sub(&b.0, bits).map(Self).map_err(error)
    }
    pub fn mul(&self, b: &Self, bits: u64) -> Result<Self, TimelineError> {
        self.0.mul(&b.0, bits).map(Self).map_err(error)
    }
    pub fn div(&self, b: &Self, bits: u64) -> Result<Self, TimelineError> {
        // Preserve the presentation domain's existing positive-divisor contract.
        if !b.positive() {
            return Err(TimelineError::Limit("invalid rational denominator"));
        }
        self.0.div(&b.0, bits).map(Self).map_err(error)
    }
    pub fn cmp(&self, b: &Self) -> Ordering {
        self.0.compare(&b.0)
    }
    pub fn positive(&self) -> bool {
        self.0.positive()
    }
    pub fn fraction(&self, bits: u64) -> Result<Self, TimelineError> {
        self.0.fraction(bits).map(Self).map_err(error)
    }
    pub fn wire(&self) -> ExactValue {
        let value = self.0.normalized();
        ExactValue {
            numerator: value.numerator().to_string(),
            denominator: value.denominator().to_string(),
        }
    }
}
