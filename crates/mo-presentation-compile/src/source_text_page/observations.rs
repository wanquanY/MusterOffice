//! Page-space envelopes of actual glyph paint, before clip/compositing. A box
//! intersection is a review candidate, never proof that visible pixels collide.
use super::*;
use mo_geometry::Rect;

pub(super) fn add_ink(
    accumulated: &mut Option<Rect>,
    bounds: Option<Rect>,
    origin: Point,
    rgba: [u8; 4],
) -> Result<(), SourcePageError> {
    if rgba[3] == 0 {
        return Ok(());
    }
    if let Some(bounds) = bounds.filter(|b| b.min.x < b.max.x && b.min.y < b.max.y) {
        let placed = bounds.translate(origin)?;
        *accumulated = Some(accumulated.map_or(placed, |b| b.union(placed)));
    }
    Ok(())
}

pub(super) fn place(
    local: Option<Rect>,
    uncertainty: Fixed,
    cell: Option<mo_presentation_source::source::table::SourceCellAddress>,
    clipping_applied: bool,
    object: &SourcePagePaintBinding,
    viewport: &RasterViewport,
    check: &dyn Fn() -> bool,
) -> Result<source_frame::capacity::PageTextInk, SourcePageError> {
    cancel(check)?;
    let mut result = source_frame::capacity::PageTextInk {
        object: SourceObjectRef {
            part: object.location.part.clone(),
            native_id: object
                .location
                .object
                .ok_or(SourcePageError::Invalid("text ink object"))?,
        },
        cell,
        bounds: None,
        clipping_applied,
        coordinate_error_bound: Fixed::ZERO,
    };
    let Some(local) = local else {
        return Ok(result);
    };
    // Reuse the same anchor, sampled world affine and precision computation as
    // glyph placement. Do not reconstruct geometry from authored object frames.
    let (affine, position, geometry) = placement::transform(
        &decorations::commands(local),
        Point {
            x: Fixed::ZERO,
            y: Fixed::ZERO,
        },
        uncertainty,
        object,
        viewport,
        check,
    )?;
    let pad = position
        .checked_add(geometry)?
        .ratio_up(viewport.scale.denominator, viewport.scale.numerator)?;
    let mut rounding = Fixed::ZERO;
    for point in [
        local.min,
        Point {
            x: local.max.x,
            y: local.min.y,
        },
        local.max,
        Point {
            x: local.min.x,
            y: local.max.y,
        },
    ] {
        cancel(check)?;
        let mapped = affine.map(point)?;
        rounding = rounding.max(mapped.error.x).max(mapped.error.y);
        let bounds = Rect {
            min: mapped.point,
            max: mapped.point,
        };
        result.bounds = Some(result.bounds.map_or(bounds, |old| old.union(bounds)));
    }
    let pad = pad.checked_add(rounding)?;
    let bounds = result
        .bounds
        .as_mut()
        .expect("four mapped envelope corners");
    bounds.min.x = bounds.min.x.checked_sub(pad)?;
    bounds.min.y = bounds.min.y.checked_sub(pad)?;
    bounds.max.x = bounds.max.x.checked_add(pad)?;
    bounds.max.y = bounds.max.y.checked_add(pad)?;
    result.coordinate_error_bound = pad;
    Ok(result)
}
