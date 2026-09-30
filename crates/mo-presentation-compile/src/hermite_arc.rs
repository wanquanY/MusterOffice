//! Shared cubic Hermite interpolation of a parametric ellipse. The per-axis
//! remainder is r |h|^4 / 384 on each segment; numeric interval error is carried
//! separately by the caller. No circular-arc fitting constants or host libm.
use crate::{CompileError, interval::Interval as I};
use mo_geometry::Fixed;

pub(crate) fn error(radii: &[I; 2], h: &I) -> Result<[Fixed; 2], CompileError> {
    let square = h.mul(h);
    let fourth = square.mul(&square).abs_upper().divide(384);
    Ok([
        radii[0].mul(&fourth).upper_q32()?,
        radii[1].mul(&fourth).upper_q32()?,
    ])
}

/// Controls and endpoint for endpoint values (cos, sin), including derivatives
/// with respect to the signed segment angle h. Reverse arcs need no special case.
pub(crate) fn cubic(
    center: &[I; 2],
    radii: &[I; 2],
    start: &[I; 2],
    end: &[I; 2],
    h: &I,
) -> [[I; 2]; 3] {
    let tangent = h.divide(3);
    [
        [
            center[0].add(&radii[0].mul(&start[0].sub(&tangent.mul(&start[1])))),
            center[1].add(&radii[1].mul(&start[1].add(&tangent.mul(&start[0])))),
        ],
        [
            center[0].add(&radii[0].mul(&end[0].add(&tangent.mul(&end[1])))),
            center[1].add(&radii[1].mul(&end[1].sub(&tangent.mul(&end[0])))),
        ],
        [
            center[0].add(&radii[0].mul(&end[0])),
            center[1].add(&radii[1].mul(&end[1])),
        ],
    ]
}
