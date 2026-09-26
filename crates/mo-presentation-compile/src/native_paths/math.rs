//! Certified polar-to-parametric conversion and arbitrary-angle trigonometry.
//! The angle input is in native 1/60000-degree units, increasing clockwise.
use super::{NativePathError, Work};
use crate::{interval::Interval as I, interval_extended::floor_ratio, trig};
use num_bigint::BigInt;

pub(super) fn pi() -> I {
    I::raw(BigInt::from(trig::PI_LO), BigInt::from(trig::PI_LO + 1))
}
fn quarter_units() -> BigInt {
    BigInt::from(trig::TURN / 4) << 96usize
}
pub(super) fn normalize_start(angle: I) -> I {
    let turn = BigInt::from(trig::TURN) << 96usize;
    let offset = floor_ratio(&angle.lo, &turn) * turn;
    I::raw(angle.lo - &offset, angle.hi - offset)
}
/// All callers keep radians bounded by the explicit arc-turn budget.
pub(super) fn cos_sin(x: &I, work: &mut Work<'_>) -> Result<[I; 2], NativePathError> {
    work.step()?;
    // Choose the nearest quarter turn from the interval midpoint. Reduction
    // remains interval arithmetic, including the uncertainty of pi itself.
    let midpoint = (&x.lo + &x.hi) / 2;
    let half_pi = BigInt::from(trig::PI_LO / 2);
    let q = floor_ratio(&(midpoint + &half_pi / 2), &half_pi);
    let q = i64::try_from(q).map_err(|_| work.numeric())?;
    let reduced = x.sub(&pi().mul(&I::integer(q)).divide(2));
    if reduced.abs_upper().hi >= I::integer(1).lo {
        return Err(work.precision());
    }
    if reduced.lo == BigInt::from(0) && reduced.hi == BigInt::from(0) {
        return Ok(rotate([I::integer(1), I::integer(0)], q));
    }
    let square = reduced.mul(&reduced).neg();
    let mut ct = I::integer(1);
    let mut st = reduced;
    let mut c = ct.clone();
    let mut s = st.clone();
    for k in 1..24 {
        work.step()?;
        ct = ct.mul(&square).divide((2 * k - 1) * (2 * k));
        st = st.mul(&square).divide((2 * k) * (2 * k + 1));
        c = c.add(&ct);
        s = s.add(&st);
    }
    // |reduced|<1: the first omitted terms are <1/48! and <1/49! <2^-96.
    c.lo -= 1;
    c.hi += 1;
    s.lo -= 1;
    s.hi += 1;
    Ok(rotate([c, s], q))
}
fn rotate([c, s]: [I; 2], q: i64) -> [I; 2] {
    match q.rem_euclid(4) {
        0 => [c, s],
        1 => [s.neg(), c],
        2 => [c.neg(), s.neg()],
        _ => [s, c.neg()],
    }
}
fn atan_ratio(y: &BigInt, x: &BigInt, work: &mut Work<'_>) -> Result<I, NativePathError> {
    work.step()?;
    if y == &BigInt::from(0) {
        return Ok(I::integer(0));
    }
    if x == &BigInt::from(0) {
        return Ok(pi().divide(2));
    }
    if y > x {
        return Ok(pi().divide(2).sub(&atan_ratio(x, y, work)?));
    }
    let mut t = I::raw(y.clone(), y.clone())
        .div_positive(&I::raw(x.clone(), x.clone()))
        .map_err(|_| work.numeric())?;
    for _ in 0..3 {
        work.step()?;
        let root = I::integer(1)
            .add(&t.mul(&t))
            .sqrt_nonnegative()
            .map_err(|_| work.numeric())?;
        t = t
            .div_positive(&I::integer(1).add(&root))
            .map_err(|_| work.numeric())?;
    }
    if t.hi > I::ratio(1, 8).hi {
        return Err(work.precision());
    }
    let square = t.mul(&t).neg();
    let mut term = t.clone();
    let mut sum = t;
    for k in 1..32 {
        work.step()?;
        term = term.mul(&square);
        sum = sum.add(&term.divide(2 * k + 1));
    }
    // Alternating remainder <= (1/8)^65/65 < one Q96 unit.
    sum.lo -= 1;
    sum.hi += 1;
    Ok(sum.mul(&I::integer(8)))
}
fn angle_at(raw: &BigInt, radii: &[I; 2], work: &mut Work<'_>) -> Result<I, NativePathError> {
    let quarter = quarter_units();
    let q = floor_ratio(raw, &quarter);
    let remainder = raw - &q * quarter;
    let q = i64::try_from(q).map_err(|_| work.numeric())?;
    let base = pi().mul(&I::integer(q)).divide(2);
    if remainder == BigInt::from(0) {
        return Ok(base);
    }
    let radians = I::raw(remainder.clone(), remainder)
        .mul(&pi())
        .divide(trig::TURN / 2);
    let [c, s] = cos_sin(&radians, work)?;
    let (rx, ry) = if q.rem_euclid(2) == 0 {
        (&radii[0], &radii[1])
    } else {
        (&radii[1], &radii[0])
    };
    let mut x = ry.mul(&c);
    let mut y = rx.mul(&s);
    // The remainder is strictly in (0,pi/2); outward rounding may cross zero.
    x.lo = x.lo.max(BigInt::from(0));
    y.lo = y.lo.max(BigInt::from(0));
    if x.hi <= BigInt::from(0) || y.hi <= BigInt::from(0) {
        return Err(work.precision());
    }
    let low = atan_ratio(&y.lo, &x.hi, work)?;
    let high = atan_ratio(&y.hi, &x.lo, work)?;
    Ok(base.add(&I::raw(low.lo, high.hi)))
}
pub(super) fn parametric(
    angle: &I,
    radii: &[I; 2],
    work: &mut Work<'_>,
) -> Result<I, NativePathError> {
    // The polar-to-parametric map is globally increasing for positive radii.
    let low = angle_at(&angle.lo, radii, work)?;
    if angle.lo == angle.hi {
        return Ok(low);
    }
    let high = angle_at(&angle.hi, radii, work)?;
    Ok(I::raw(low.lo, high.hi))
}
