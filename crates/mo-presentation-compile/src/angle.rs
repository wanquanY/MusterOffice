//! Bounded exact local orientation. Integer documents keep the existing fast path.
use crate::{CompileError, interval::Interval, trig};
use mo_timeline::ExactValue;
use num_bigint::BigInt;
const MAX_BITS: u64 = 8192;
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum Angle {
    Integer(i64),
    Fraction { n: BigInt, d: BigInt },
}
impl Angle {
    pub fn integer(value: i64) -> Self {
        Self::Integer(value.rem_euclid(trig::TURN))
    }
    pub fn exact(value: &ExactValue) -> Result<Self, CompileError> {
        // Bound before parsing; even a forged internal value cannot allocate an
        // arbitrarily large integer. Public playback derives these values itself.
        if value.numerator.len() > 2470 || value.denominator.len() > 2470 {
            return Err(CompileError::Limit("angle digits"));
        }
        let parse = |s: &str| {
            s.parse::<BigInt>()
                .map_err(|_| CompileError::Invalid("exact angle"))
        };
        Self::ratio(parse(&value.numerator)?, parse(&value.denominator)?)
    }
    fn ratio(n: BigInt, d: BigInt) -> Result<Self, CompileError> {
        if d <= BigInt::from(0) {
            return Err(CompileError::Invalid("angle denominator"));
        }
        if n.bits() > MAX_BITS || d.bits() > MAX_BITS {
            return Err(CompileError::Limit("exact angle arithmetic"));
        }
        let turn = &d * trig::TURN;
        let mut n = n % &turn;
        if n < BigInt::from(0) {
            n += turn;
        }
        let mut a = n.clone();
        let mut b = d.clone();
        while b != BigInt::from(0) {
            (a, b) = (b.clone(), a % b);
        }
        let n = n / &a;
        let d = d / a;
        if d == BigInt::from(1) {
            return Ok(Self::Integer(
                i64::try_from(n).expect("reduced angle below one turn"),
            ));
        }
        if n.bits() > MAX_BITS || d.bits() > MAX_BITS {
            return Err(CompileError::Limit("exact angle arithmetic"));
        }
        Ok(Self::Fraction { n, d })
    }
    fn parts(&self) -> (BigInt, BigInt) {
        match self {
            Self::Integer(n) => ((*n).into(), 1.into()),
            Self::Fraction { n, d } => (n.clone(), d.clone()),
        }
    }
    pub fn compose(&self, own: &Self, reflected: bool) -> Result<Self, CompileError> {
        if let (Self::Integer(a), Self::Integer(b)) = (self, own) {
            return Ok(Self::integer(a + if reflected { -b } else { *b }));
        }
        let (a, b) = self.parts();
        let (c, d) = own.parts();
        // Form the least common denominator before multiplication. Repeated
        // samples/ancestors with the same timebase must not inflate the budget.
        let mut g = b.clone();
        let mut r = d.clone();
        while r != BigInt::from(0) {
            (g, r) = (r.clone(), g % r);
        }
        let left = &d / &g;
        let right = &b / &g;
        Self::ratio(
            a * left + if reflected { -c * &right } else { c * &right },
            right * d,
        )
    }
    pub fn exchanges_parent_axes(&self) -> bool {
        match self {
            Self::Integer(a) => (2700000..8100000).contains(a) || (13500000..18900000).contains(a),
            Self::Fraction { n, d } => {
                (n >= &(d * 2700000) && n < &(d * 8100000))
                    || (n >= &(d * 13500000) && n < &(d * 18900000))
            }
        }
    }
    pub fn cos_sin(&self, check: &dyn Fn() -> bool) -> Result<[Interval; 2], CompileError> {
        match self {
            Self::Integer(a) => trig::cos_sin(*a, check),
            Self::Fraction { n, d } => {
                crate::cancel(check)?;
                let quarter = d * (trig::TURN / 4);
                let quadrant = i64::try_from(n / &quarter).expect("bounded quadrant");
                let offset = n % &quarter;
                let swap = &offset * 2 > quarter;
                let reduced = if swap { quarter - offset } else { offset };
                let scaled = reduced << 96usize;
                let lo = &scaled / d;
                let hi = &lo + if &scaled % d == BigInt::from(0) { 0 } else { 1 };
                trig::reduced_cos_sin(quadrant, swap, Interval::raw(lo, hi), check)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn a(n: i64, d: i64) -> Angle {
        Angle::ratio(n.into(), d.into()).unwrap()
    }
    #[test]
    fn rational_thresholds_are_exact_and_reflected_composition_is_canonical() {
        for (boundary, after) in [
            (2700000, true),
            (8100000, false),
            (13500000, true),
            (18900000, false),
        ] {
            assert_eq!(a(boundary * 3 - 1, 3).exchanges_parent_axes(), !after);
            assert_eq!(a(boundary * 3, 3).exchanges_parent_axes(), after);
            assert_eq!(a(boundary * 3 + 1, 3).exchanges_parent_axes(), after);
        }
        assert_eq!(a(1, 3).compose(&a(2, 3), false).unwrap(), Angle::integer(1));
        assert_eq!(a(1, 3).compose(&a(2, 3), true).unwrap(), a(-1, 3));
        assert_eq!(a(-21600000 * 3 + 1, 3), a(1, 3));
    }
    #[test]
    fn fractions_that_reduce_to_cardinals_keep_zero_uncertainty() {
        for angle in [0, 5400000, 10800000, 16200000] {
            let v = a(angle * 7, 7).cos_sin(&|| false).unwrap();
            let expected = Angle::integer(angle).cos_sin(&|| false).unwrap();
            for i in 0..2 {
                assert_eq!(v[i].lo, expected[i].lo);
                assert_eq!(v[i].hi, expected[i].hi);
                assert_eq!(v[i].q32().unwrap().1, mo_geometry::Fixed::ZERO);
            }
        }
    }
    #[test]
    fn forged_and_over_budget_ratios_do_not_allocate_unbounded_arithmetic() {
        for (n, d) in [
            ("1".into(), "0".into()),
            ("1".repeat(2471), "1".into()),
            ("not-angle".into(), "1".into()),
        ] {
            assert!(
                Angle::exact(&ExactValue {
                    numerator: n,
                    denominator: d
                })
                .is_err()
            );
        }
        let d: BigInt = (BigInt::from(1) << 8100usize) + 3;
        let p = Angle::ratio(1.into(), d.clone()).unwrap();
        assert!(p.compose(&p, false).is_ok());
        let q = Angle::ratio(1.into(), d + 2).unwrap();
        assert!(matches!(p.compose(&q, false), Err(CompileError::Limit(_))));
    }
}
