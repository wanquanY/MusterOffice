use crate::{CompileError, cancel, interval::Interval};
use num_bigint::BigInt;
pub(crate) const TURN: i64 = mo_presentation_model::ROTATION_UNITS_PER_TURN as i64;
const QUARTER: i64 = TURN / 4;
// floor(pi * 2^96), independently enclosed by the Machin identity in tests.
pub(crate) const PI_LO: u128 = 248902613312231085230521944622;
pub(crate) fn radians(angle: i64) -> Interval {
    Interval::raw(BigInt::from(PI_LO), BigInt::from(PI_LO + 1))
        .mul(&Interval::integer(angle))
        .divide(TURN / 2)
}

/// (cos, sin), clockwise in a y-down canvas. Exact quadrant reduction preserves
/// all cardinal angles; an alternating Taylor series encloses the remainder.
pub(crate) fn cos_sin(angle: i64, check: &dyn Fn() -> bool) -> Result<[Interval; 2], CompileError> {
    cancel(check)?;
    let angle = angle.rem_euclid(TURN);
    let quadrant = angle / QUARTER;
    let offset = angle % QUARTER;
    let swap = offset > QUARTER / 2;
    let reduced = if swap { QUARTER - offset } else { offset };
    reduced_cos_sin(quadrant, swap, Interval::integer(reduced), check)
}
/// `reduced` encloses a nonnegative exact angle at most 45 degrees. Rational
/// reduction rounds outward before Taylor evaluation; no angle is rounded to
/// the source format's integral angle grid.
pub(crate) fn reduced_cos_sin(
    quadrant: i64,
    swap: bool,
    reduced: Interval,
    check: &dyn Fn() -> bool,
) -> Result<[Interval; 2], CompileError> {
    let (mut c, mut s) = if reduced.hi == BigInt::from(0) {
        (Interval::integer(1), Interval::integer(0))
    } else {
        let pi = Interval::raw(BigInt::from(PI_LO), BigInt::from(PI_LO + 1));
        let x = pi.mul(&reduced).divide(TURN / 2);
        let xx = x.mul(&x).neg();
        let mut ct = Interval::integer(1);
        let mut st = x;
        let mut c = ct.clone();
        let mut s = st.clone();
        for k in 1..24 {
            cancel(check)?;
            ct = ct.mul(&xx).divide((2 * k - 1) * (2 * k));
            st = st.mul(&xx).divide((2 * k) * (2 * k + 1));
            c = c.add(&ct);
            s = s.add(&st);
        }
        // |x| <= pi/4 < 1: omitted remainders < 1/48! and 1/49!,
        // both strictly below one Q96 unit. Interval operations round out.
        c.lo -= 1;
        c.hi += 1;
        s.lo -= 1;
        s.hi += 1;
        (c, s)
    };
    if swap {
        std::mem::swap(&mut c, &mut s);
    }
    Ok(match quadrant {
        0 => [c, s],
        1 => [s.neg(), c],
        2 => [c.neg(), s.neg()],
        _ => [s, c.neg()],
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn pi_constant_has_an_independent_machin_enclosure() {
        // Exact rational term divisions at Q256; the first omitted alternating
        // term bounds the series. 128 terms at 1/5 have error < 2^-590.
        fn atan(d: i64) -> (BigInt, BigInt) {
            let scale = BigInt::from(1) << 256usize;
            let mut power = BigInt::from(d);
            let mut lo = BigInt::from(0);
            let mut hi = lo.clone();
            for k in 0..128 {
                let denominator = &power * (2 * k + 1);
                let v = &scale / denominator;
                if k % 2 == 0 {
                    lo += &v;
                    hi += &v + 1;
                } else {
                    lo -= &v + 1;
                    hi -= &v;
                }
                power *= d * d;
            }
            // The next term is positive and less than one Q256 unit.
            hi += 1;
            (lo, hi)
        }
        let (al, ah) = atan(5);
        let (bl, bh) = atan(239);
        assert_eq!((al * 16 - bh * 4) >> 160usize, BigInt::from(PI_LO));
        assert_eq!((ah * 16 - bl * 4) >> 160usize, BigInt::from(PI_LO));
    }
}
