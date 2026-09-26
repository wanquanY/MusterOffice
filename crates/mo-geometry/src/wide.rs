//! Bounded exact intermediates. Ordinary page arithmetic stays in i128; the
//! fallback handles cancellation of at most three <=256-bit products. Inputs
//! are typed i128 values, never unbounded decimal expressions from a caller.
use crate::GeometryError;
use num_bigint::{BigInt, Sign};
const U: i128 = 1 << 32;
fn narrow(v: BigInt) -> Result<i128, GeometryError> {
    i128::try_from(v).map_err(|_| GeometryError::Numeric)
}
fn rounded(n: i128) -> Result<(i128, bool), GeometryError> {
    let q = n.div_euclid(U);
    let r = n.rem_euclid(U);
    let up = r > U / 2 || (r == U / 2 && n >= 0);
    Ok((
        q.checked_add(i128::from(up))
            .ok_or(GeometryError::Numeric)?,
        r != 0,
    ))
}
fn rounded_wide(n: BigInt) -> Result<(i128, bool), GeometryError> {
    let mut q = &n >> 32usize;
    let r = narrow(&n - (&q << 32usize))?;
    if r > U / 2 || (r == U / 2 && n.sign() != Sign::Minus) {
        q += 1;
    }
    Ok((narrow(q)?, r != 0))
}
pub(crate) fn dot(
    a: i128,
    x: i128,
    b: i128,
    y: i128,
    t: i128,
) -> Result<(i128, bool), GeometryError> {
    if let Some(n) = a
        .checked_mul(x)
        .and_then(|v| b.checked_mul(y).and_then(|w| v.checked_add(w)))
        .and_then(|v| t.checked_mul(U).and_then(|w| v.checked_add(w)))
    {
        return rounded(n);
    }
    rounded_wide(BigInt::from(a) * x + BigInt::from(b) * y + (BigInt::from(t) << 32usize))
}
pub(crate) fn dot_delta(
    a: i128,
    x: i128,
    ox: i128,
    b: i128,
    y: i128,
    oy: i128,
) -> Result<(i128, bool), GeometryError> {
    if let (Some(x), Some(y)) = (x.checked_sub(ox), y.checked_sub(oy)) {
        return dot(a, x, b, y, 0);
    }
    rounded_wide(
        BigInt::from(a) * (BigInt::from(x) - ox) + BigInt::from(b) * (BigInt::from(y) - oy),
    )
}
pub(crate) fn dot_in_view(
    a: i128,
    x: i128,
    b: i128,
    y: i128,
    t: i128,
    origin: i128,
) -> Result<(i128, bool), GeometryError> {
    if let Some(t) = t.checked_sub(origin) {
        return dot(a, x, b, y, t);
    }
    rounded_wide(
        BigInt::from(a) * x + BigInt::from(b) * y + ((BigInt::from(t) - origin) << 32usize),
    )
}
pub(crate) fn error_up(a: i128, x: i128, b: i128, y: i128) -> Result<i128, GeometryError> {
    if x < 0 || y < 0 {
        return Err(GeometryError::Invalid("negative coordinate error"));
    }
    if let Some(n) = a
        .checked_abs()
        .and_then(|a| a.checked_mul(x))
        .and_then(|v| {
            b.checked_abs()
                .and_then(|b| b.checked_mul(y))
                .and_then(|w| v.checked_add(w))
        })
    {
        return Ok(n / U + i128::from(n % U != 0));
    }
    let n = BigInt::from(a.unsigned_abs()) * x + BigInt::from(b.unsigned_abs()) * y;
    narrow((n + (U - 1)) >> 32usize)
}
pub(crate) fn ratio_up(v: i128, n: u32, d: u32) -> Result<i128, GeometryError> {
    if v < 0 || d == 0 {
        return Err(GeometryError::Invalid("nonnegative ratio"));
    }
    if let Some(v) = v.checked_mul(i128::from(n)) {
        return Ok(v / i128::from(d) + i128::from(v % i128::from(d) != 0));
    }
    narrow((BigInt::from(v) * n + (d - 1)) / d)
}
pub(crate) fn deviation(a: i128, b: i128, c: i128) -> Result<i128, GeometryError> {
    if let Some(v) = a
        .checked_add(b)
        .and_then(|v| v.checked_sub(c))
        .and_then(i128::checked_abs)
    {
        return Ok(v);
    }
    let n = BigInt::from(a) + b - c;
    narrow(if n.sign() == Sign::Minus { -n } else { n })
}
