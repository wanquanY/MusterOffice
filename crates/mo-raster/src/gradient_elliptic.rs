//! Ellipse parameter conversion with a geometry displacement budget. First-entry
//! roots may change branches under perturbation: never call this a scalar bound.
use crate::{
    EllipticGradientWork, PixelScale, RasterError as E, RasterViewport,
    number::Scale,
    paint_precision::{I, ceil_integer},
};
use mo_geometry::Fixed;

pub(crate) fn compile(
    values: [Fixed; 6],
    uncertainty: [Fixed; 6],
    matrix: [f32; 6],
    matrix_errors: [i128; 6],
    phase_error: Fixed,
    view: &RasterViewport,
) -> Result<([f32; 6], EllipticGradientWork), E> {
    if uncertainty.iter().any(|e| e.raw() < 0) {
        return Err(E::Invalid("negative elliptic gradient uncertainty"));
    }
    if values[..2]
        .iter()
        .any(|v| !(1..=1i128 << 32).contains(&v.raw()))
        || values[4..].iter().any(|v| v.raw() < 0)
    {
        return Err(E::Invalid("elliptic gradient scale or radius"));
    }
    let unit = Scale::new(PixelScale {
        numerator: 1,
        denominator: 1,
    })?;
    let mut f = [0f32; 6];
    let mut error = [Fixed::ZERO; 6];
    for i in 0..6 {
        f[i] = unit.value(values[i].raw())?;
        error[i] = Fixed::from_raw(
            unit.error(values[i].raw(), f[i])?
                .checked_add(uncertainty[i].raw())
                .ok_or(E::Range)?,
        );
    }
    let mut coordinate_error = phase_error;
    if error.iter().any(|e| e.raw() != 0) || matrix_errors.iter().any(|e| *e != 0) {
        let a: [I; 6] = std::array::from_fn(|i| I::around(f64::from(f[i]), error[i].raw()));
        let b = f.map(|v| I::exact(f64::from(v)));
        let half = I::exact(0.5);
        let mut delta = [I::exact(0.0); 2];
        let mut extent = [I::exact(0.0); 2];
        for i in 0..2 {
            if a[i].lo <= 0.0 {
                return Err(E::Precision);
            }
            // In unit-tile coordinates C=(1+c/scale)/2, R=r/(2*scale).
            // Both vary affinely in t. Corresponding points at any t,angle
            // have displacement bounded by max of the two endpoint budgets.
            let center = half.add(half.mul(a[i + 2]).div(a[i])?);
            let encoded_center = half.add(half.mul(b[i + 2]).div(b[i])?);
            let radius = half.mul(a[i + 4]).div(a[i])?;
            let encoded_radius = half.mul(b[i + 4]).div(b[i])?;
            let outer = half.div(a[i])?;
            let encoded_outer = half.div(b[i])?;
            let inner_error = I::exact(center.sub(encoded_center).magnitude())
                .add(I::exact(radius.sub(encoded_radius).magnitude()));
            delta[i] = I::exact(inner_error.hi.max(outer.sub(encoded_outer).magnitude()));
            extent[i] = I::exact(
                I::exact(center.magnitude())
                    .add(I::exact(radius.magnitude()))
                    .hi
                    .max(half.add(I::exact(outer.magnitude())).hi),
            );
        }
        let mut raw = 0;
        for row in [0, 3] {
            // Encoded matrix maps parameter displacement. Source matrix error
            // multiplies the uncertain full extent, including the cross term.
            let displacement = I::exact(f64::from(matrix[row]).abs())
                .mul(delta[0])
                .add(I::exact(f64::from(matrix[row + 1]).abs()).mul(delta[1]));
            let affine_raw = I::integer(matrix_errors[row])
                .mul(extent[0])
                .add(I::integer(matrix_errors[row + 1]).mul(extent[1]))
                .add(I::integer(matrix_errors[row + 2]));
            let bound = displacement.mul(I::exact(4294967296.0)).add(affine_raw);
            raw = raw.max(ceil_integer(bound.hi)?);
        }
        // The phase term covers translated/reflected copies over the viewport.
        // Adding it again is conservative even where the matrix terms overlap.
        coordinate_error = Fixed::from_raw(raw.checked_add(phase_error.raw()).ok_or(E::Range)?);
    }
    if coordinate_error > view.coordinate_tolerance {
        return Err(E::Precision);
    }
    Ok((
        f,
        EllipticGradientWork {
            parameter_error_bounds: error,
            coordinate_error_bound: coordinate_error,
            encoded_root_interval_bound: Fixed::from_raw(1 << 9),
        },
    ))
}
