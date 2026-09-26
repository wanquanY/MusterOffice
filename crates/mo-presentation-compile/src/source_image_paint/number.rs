use super::{ImagePaintError as E, *};
use num_bigint::BigInt;

pub(super) fn enclose(v: Fixed, error: Fixed) -> Result<I, E> {
    if error.raw() < 0 {
        return Err(E::Invalid("negative input uncertainty"));
    }
    let center = I::fixed(v);
    let e = I::fixed(error);
    Ok(I::raw(center.lo - e.lo, center.hi + e.hi))
}
pub(super) fn rect(r: ImageLayoutRectangle, errors: [Fixed; 4]) -> Result<[I; 4], E> {
    Ok([
        enclose(r.left, errors[0])?,
        enclose(r.top, errors[1])?,
        enclose(r.right, errors[2])?,
        enclose(r.bottom, errors[3])?,
    ])
}
pub(super) fn positive(v: &I) -> Result<(), E> {
    if v.lo <= BigInt::from(0) {
        Err(E::Precision)
    } else {
        Ok(())
    }
}
pub(super) fn point(values: &[I; 2]) -> Result<(Point, Point), E> {
    let (x, ex) = values[0].q32().map_err(|_| E::Range)?;
    let (y, ey) = values[1].q32().map_err(|_| E::Range)?;
    Ok((Point { x, y }, Point { x: ex, y: ey }))
}
pub(super) fn map(m: &[I], t: &[I; 2], p: &[I; 2]) -> [I; 2] {
    std::array::from_fn(|row| {
        m[row * 2]
            .mul(&p[0])
            .add(&m[row * 2 + 1].mul(&p[1]))
            .add(&t[row])
    })
}
pub(super) fn deviation(actual: &I, nominal: &I) -> Result<Fixed, E> {
    let delta = actual.sub(nominal);
    I::raw(BigInt::from(0), (-delta.lo).max(delta.hi))
        .upper_q32()
        .map_err(|_| E::Range)
}
