//! Bind native image-local geometry to world paint and a separate fill clip.
//! Native placement is already flattened; neither parent transforms nor source
//! origin/anchor may be applied again by a scene consumer.
mod number;
mod types;
use crate::{interval::Interval as I, source_image_layout::*};
use mo_geometry::{Affine, Fixed, PathCommand as C, Point};
use mo_pptx::source::fill::resolve::FillTarget;
use mo_raster::{
    FillPath, FillRule, ImageBrush, ImageBrushUncertainty, ImageSampling, ImageSourceDomain,
};
use number::*;
pub use types::*;

pub const PROFILE: &str = "drawingml-image-world-paint-q96-v1-draft";
const ZERO: Point = Point {
    x: Fixed::ZERO,
    y: Fixed::ZERO,
};
fn cancel(check: &dyn Fn() -> bool) -> Result<(), ImagePaintError> {
    if check() {
        Err(ImagePaintError::Cancelled)
    } else {
        Ok(())
    }
}

/// Compute paint for a source-bound local plan. Backgrounds use the page basis;
/// object fills that follow their shape use its resolved native affine. The
/// stationary-image orientation policy is not guessed from a browser or host
/// matrix: it remains an explicit prerequisite until its native rules resolve.
pub fn compile(
    plan: &ImageSourceLayoutPlan,
    sampling: ImageSampling,
    check: &dyn Fn() -> bool,
) -> Result<NativeImagePaint, ImagePaintError> {
    use ImagePaintError as E;
    cancel(check)?;
    let l = &plan.layout;
    if l.profile != crate::source_image_layout::PROFILE {
        return Err(E::Invalid("local layout profile"));
    }
    let zero = crate::AffineUncertainty {
        linear: [Fixed::ZERO; 4],
        translation: ZERO,
    };
    let (affine, uncertainty, anchor) = match (&plan.target, &plan.placement) {
        (FillTarget::Background {}, None) => (Affine::IDENTITY, &zero, ZERO),
        (
            FillTarget::Object { .. } | FillTarget::Picture { .. } | FillTarget::Line { .. },
            Some(p),
        ) => {
            if !l.rotate_with_shape {
                return Err(E::OrientationRequired);
            }
            // Image layout starts at the top-left of the effective source box.
            // A group viewport may start at chOff, unlike an ordinary shape.
            let anchor = Point {
                x: p.anchor
                    .x
                    .checked_sub(Fixed::emu(p.source_origin.x))
                    .map_err(|_| E::Range)?,
                y: p.anchor
                    .y
                    .checked_sub(Fixed::emu(p.source_origin.y))
                    .map_err(|_| E::Range)?,
            };
            (p.affine, &p.uncertainty, anchor)
        }
        _ => return Err(E::Invalid("target and placement identity")),
    };
    let source = rect(l.source_rectangle, l.uncertainty.source_rectangle)?;
    let fill = rect(l.fill_rectangle, l.uncertainty.fill_rectangle)?;
    positive(&source[2].sub(&source[0]))?;
    positive(&source[3].sub(&source[1]))?;
    positive(&fill[2].sub(&fill[0]))?;
    positive(&fill[3].sub(&fill[1]))?;
    let step = [
        enclose(l.pixel_step.x, l.uncertainty.pixel_step.x)?,
        enclose(l.pixel_step.y, l.uncertainty.pixel_step.y)?,
    ];
    for s in &step {
        positive(s)?;
    }
    let m = [
        enclose(affine.linear[0], uncertainty.linear[0])?,
        enclose(affine.linear[1], uncertainty.linear[1])?,
        enclose(affine.linear[2], uncertainty.linear[2])?,
        enclose(affine.linear[3], uncertainty.linear[3])?,
    ];
    let translation = [
        enclose(affine.translation.x, uncertainty.translation.x)?,
        enclose(affine.translation.y, uncertainty.translation.y)?,
    ];
    let origin = [
        enclose(l.origin.x, l.uncertainty.origin.x)?.sub(&I::fixed(anchor.x)),
        enclose(l.origin.y, l.uncertainty.origin.y)?.sub(&I::fixed(anchor.y)),
    ];
    let mapped = map(&m, &translation, &origin);
    cancel(check)?;
    let (origin, origin_error) = point(&mapped)?;
    let (x_step, x_error) = point(&[m[0].mul(&step[0]), m[2].mul(&step[0])])?;
    let (y_step, y_error) = point(&[m[1].mul(&step[1]), m[3].mul(&step[1])])?;
    let error = ImageBrushUncertainty {
        origin: origin_error,
        x_step: x_error,
        y_step: y_error,
        source_domain: l.uncertainty.source_rectangle,
    };
    let exact = error.origin == ZERO
        && error.x_step == ZERO
        && error.y_step == ZERO
        && error.source_domain.iter().all(|v| *v == Fixed::ZERO);
    let r = l.source_rectangle;
    let brush = ImageBrush {
        resource: plan.resource,
        origin,
        x_step,
        y_step,
        tile_x: l.tile_x,
        tile_y: l.tile_y,
        sampling,
        source_domain: Some(ImageSourceDomain {
            left: r.left,
            top: r.top,
            right: r.right,
            bottom: r.bottom,
        }),
        uncertainty: if exact { None } else { Some(Box::new(error)) },
    };
    let fill_clip = if l.clip_to_fill_rectangle {
        let r = l.fill_rectangle;
        let edges = [r.left, r.top, r.right, r.bottom];
        let nominal_m = affine.linear.map(I::fixed);
        let nominal_t = [
            I::fixed(affine.translation.x),
            I::fixed(affine.translation.y),
        ];
        let mut commands = Vec::with_capacity(5);
        let mut error = ZERO;
        for (i, [x, y]) in [[0, 1], [2, 1], [2, 3], [0, 3]].into_iter().enumerate() {
            cancel(check)?;
            let p = Point {
                x: edges[x].checked_sub(anchor.x).map_err(|_| E::Range)?,
                y: edges[y].checked_sub(anchor.y).map_err(|_| E::Range)?,
            };
            // Bound only upstream layout/native-affine uncertainty here. The
            // scene compiler owns its own Q32 transform and float conversion.
            let exact = map(
                &m,
                &translation,
                &[
                    fill[x].sub(&I::fixed(anchor.x)),
                    fill[y].sub(&I::fixed(anchor.y)),
                ],
            );
            let nominal = map(&nominal_m, &nominal_t, &[I::fixed(p.x), I::fixed(p.y)]);
            error.x = error.x.max(deviation(&exact[0], &nominal[0])?);
            error.y = error.y.max(deviation(&exact[1], &nominal[1])?);
            commands.push(if i == 0 {
                C::Move { to: p }
            } else {
                C::Line { to: p }
            });
        }
        commands.push(C::Close);
        Some(ImageFillClip {
            path: FillPath {
                fill_rule: FillRule::Nonzero,
                commands,
            },
            affine,
            upstream_error: error,
        })
    } else {
        None
    };
    cancel(check)?;
    Ok(NativeImagePaint {
        profile: PROFILE.into(),
        target: plan.target.clone(),
        brush,
        fill_clip,
    })
}

#[cfg(test)]
mod tests;
