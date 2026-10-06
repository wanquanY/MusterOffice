//! Exact remainder stepping; fractional filter lookup is quantized independently
//! at each sample. Never accumulate a floating or quantized rate.
use crate::{AudioError, Result};
use mo_time::{ExactRatio as R, Rounding};
use num_bigint::BigInt;

#[derive(Debug, Clone)]
enum Fraction {
    Small {
        remainder: u128,
        step: u128,
        denominator: u128,
    },
    Big {
        remainder: BigInt,
        step: BigInt,
        denominator: BigInt,
    },
}
#[derive(Debug, Clone)]
pub struct PhaseClock {
    whole: i64,
    step_whole: i64,
    fraction: Fraction,
}
impl PhaseClock {
    pub fn new(start: &R, step: &R, bits: u64) -> Result<Self> {
        start.check_budget(bits)?;
        step.check_budget(bits)?;
        let mut a = start.denominator().clone();
        let mut b = step.denominator().clone();
        while b != BigInt::from(0) {
            (a, b) = (b.clone(), a % b);
        }
        let d = start.denominator() / a * step.denominator();
        if d.bits() > bits {
            return Err(AudioError::Budget);
        }
        let whole = start.round_integer(Rounding::Floor)?;
        let step_whole = step.round_integer(Rounding::Floor)?;
        let rem = (start.numerator() - BigInt::from(whole) * start.denominator())
            * (&d / start.denominator());
        let delta = (step.numerator() - BigInt::from(step_whole) * step.denominator())
            * (&d / step.denominator());
        let fraction = if d.bits() <= 94 {
            Fraction::Small {
                remainder: u128::try_from(rem).map_err(|_| AudioError::Invalid("phase"))?,
                step: u128::try_from(delta).map_err(|_| AudioError::Invalid("phase"))?,
                denominator: u128::try_from(d).map_err(|_| AudioError::Invalid("phase"))?,
            }
        } else {
            Fraction::Big {
                remainder: rem,
                step: delta,
                denominator: d,
            }
        };
        Ok(Self {
            whole,
            step_whole,
            fraction,
        })
    }
    /// Exact floor index and floor-quantized Q0.32 fractional lookup phase.
    /// Fractional error is < 2^-32 samples and never accumulates across calls.
    pub fn position(&self) -> (i64, u32) {
        let fraction = match &self.fraction {
            Fraction::Small {
                remainder,
                denominator,
                ..
            } => (remainder << 32) / denominator,
            Fraction::Big {
                remainder,
                denominator,
                ..
            } => u128::try_from((remainder << 32) / denominator).expect("proper fractional phase"),
        };
        (self.whole, fraction as u32)
    }
    pub fn value(&self) -> f64 {
        let (whole, fraction) = self.position();
        whole as f64 + f64::from(fraction) / 4294967296.
    }
    pub fn advance(&mut self) -> Result<()> {
        let next_whole = |carry: i64| {
            i64::try_from(i128::from(self.whole) + i128::from(self.step_whole) + i128::from(carry))
                .map_err(|_| AudioError::Budget)
        };
        self.whole = match &mut self.fraction {
            Fraction::Small {
                remainder,
                step,
                denominator,
            } => {
                let sum = *remainder + *step;
                let carry = i64::from(sum >= *denominator);
                let whole = next_whole(carry)?;
                *remainder = if carry == 1 { sum - *denominator } else { sum };
                whole
            }
            Fraction::Big {
                remainder,
                step,
                denominator,
            } => {
                let sum = &*remainder + &*step;
                let carry = i64::from(sum >= *denominator);
                let whole = next_whole(carry)?;
                *remainder = if carry == 1 { sum - &*denominator } else { sum };
                whole
            }
        };
        Ok(())
    }
}
