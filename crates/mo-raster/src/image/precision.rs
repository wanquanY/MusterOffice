//! Image-specific input uncertainty validation; shared paint bounds are separate.
use super::ImageBrush;
use crate::{RasterError as E, RasterViewport};
use mo_geometry::Fixed;

pub(super) fn input_errors(
    brush: &ImageBrush,
    view: &RasterViewport,
) -> Result<([i128; 6], [i128; 4]), E> {
    let Some(e) = brush.uncertainty.as_deref() else {
        return Ok(([0; 6], [0; 4]));
    };
    let values = [
        e.x_step.x, e.y_step.x, e.origin.x, e.x_step.y, e.y_step.y, e.origin.y,
    ];
    if values.iter().chain(&e.source_domain).any(|v| v.raw() < 0) {
        return Err(E::Invalid("negative image input uncertainty"));
    }
    if brush.source_domain.is_none() && e.source_domain.iter().any(|v| v.raw() != 0) {
        return Err(E::Invalid(
            "image domain uncertainty requires source domain",
        ));
    }
    let mut matrix = [0; 6];
    for (out, value) in matrix.iter_mut().zip(values) {
        *out = value
            .ratio_up(view.scale.numerator, view.scale.denominator)
            .map_err(|_| E::Range)?
            .raw();
    }
    Ok((matrix, e.source_domain.map(Fixed::raw)))
}
