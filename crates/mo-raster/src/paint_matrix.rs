//! Shared forward paint-basis quantization. No resource or color semantics.
use crate::{RasterError, RasterViewport, number::Scale};
use mo_geometry::{Fixed, Point};
pub(crate) fn compile(
    origin: Point,
    x_step: Point,
    y_step: Point,
    mut errors: [i128; 6],
    view: &RasterViewport,
) -> Result<([f32; 6], [i128; 6]), RasterError> {
    let scale = Scale::new(view.scale)?;
    let ox = origin
        .x
        .checked_sub(view.origin.x)
        .map_err(|_| RasterError::Range)?;
    let oy = origin
        .y
        .checked_sub(view.origin.y)
        .map_err(|_| RasterError::Range)?;
    let raw: [Fixed; 6] = [x_step.x, y_step.x, ox, x_step.y, y_step.y, oy];
    let mut matrix = [0.0; 6];
    for (i, value) in raw.iter().enumerate() {
        matrix[i] = scale.value(value.raw())?;
        errors[i] = errors[i]
            .checked_add(scale.error(value.raw(), matrix[i])?)
            .ok_or(RasterError::Range)?;
    }
    let m = matrix.map(f64::from);
    let det = m[0] * m[4] - m[1] * m[3];
    let trace = m[0] * m[0] + m[1] * m[1] + m[3] * m[3] + m[4] * m[4];
    if trace == 0.0 {
        return Err(RasterError::Precision);
    }
    let largest = ((trace + (trace * trace - 4.0 * det * det).max(0.0).sqrt()) / 2.0).sqrt();
    if det.abs() / largest < 1.0 / 16384.0 {
        return Err(RasterError::Precision);
    }
    Ok((matrix, errors))
}
pub(crate) fn bounds(matrix: [f32; 6], domain: [f64; 4]) -> Result<(), RasterError> {
    let m = matrix.map(f64::from);
    for x in [domain[0], domain[2]] {
        for y in [domain[1], domain[3]] {
            if ((m[0] * x + m[1] * y) + m[2]).abs() > 32768.0
                || ((m[3] * x + m[4] * y) + m[5]).abs() > 32768.0
            {
                return Err(RasterError::Range);
            }
        }
    }
    Ok(())
}
