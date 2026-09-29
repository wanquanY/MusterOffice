//! Native overflow masks. Inactive axes enclose all painted ink plus a device-AA
//! guard derived from the inverse sampled affine, never a guessed huge rectangle.
use super::*;
use crate::interval::Interval as I;
use mo_geometry::{Affine, Rect};
use mo_raster::{FillPath, FillRule};
use num_bigint::BigInt;

#[derive(Clone, Copy)]
pub(super) struct LocalClip {
    constraint: FrameClip,
    ink: Rect,
    ink_uncertainty: Fixed,
}
impl LocalClip {
    pub fn for_frame(
        frame: &SourceFramePlan,
        decorations: &[TextDecoration],
        uncertainty: Fixed,
    ) -> Result<Option<Self>, SourcePageError> {
        let Some(constraint) = frame.clip else {
            return Ok(None);
        };
        let mut ink = frame
            .bounds
            .map_or(constraint.bounds, |b| constraint.bounds.union(b));
        for decoration in decorations {
            ink = ink.union(decoration.rect);
        }
        Ok(Some(Self {
            constraint,
            ink,
            ink_uncertainty: uncertainty.checked_add(Fixed::from_raw(8))?,
        }))
    }
    fn rectangle(
        &self,
        affine: &Affine,
        viewport: &RasterViewport,
    ) -> Result<Rect, SourcePageError> {
        let mut result = self.constraint.bounds;
        if self.constraint.horizontal && self.constraint.vertical {
            return Ok(result);
        }
        let [a, b, c, d] = affine.linear.map(I::fixed);
        let determinant = a.mul(&d).sub(&b.mul(&c));
        if determinant.lo == BigInt::from(0) && determinant.hi == BigInt::from(0) {
            // An exact rank-deficient affine maps all filled glyph/decoration
            // geometry to zero area. Its finite native mask collapses with it;
            // no inverse-AA guard is needed or defined at a zero-scale keyframe.
            return Ok(result);
        }
        let determinant = if determinant.lo > BigInt::from(0) {
            determinant
        } else if determinant.hi < BigInt::from(0) {
            determinant.neg()
        } else {
            return Err(RasterError::Precision.into());
        };
        // An antialiased fill has at most one device-pixel support outside ink.
        // Two pixels also cover the certified (<1/256 px) placement uncertainty
        // on both the outline and the inactive clip edge. The inverse infinity
        // norm bounds that device neighbourhood in either local axis.
        let pixel = I::ratio(
            i64::from(viewport.scale.denominator) * 2,
            i64::from(viewport.scale.numerator),
        );
        let pad = |v: &I, w: &I| -> Result<Fixed, SourcePageError> {
            Ok(v.abs_upper()
                .add(&w.abs_upper())
                .mul(&pixel)
                .div_positive(&determinant)
                .map_err(|_| RasterError::Precision)?
                .upper_q32()
                .map_err(|_| RasterError::Precision)?
                .checked_add(self.ink_uncertainty)?)
        };
        if !self.constraint.horizontal {
            let x = pad(&d, &b)?;
            result.min.x = self.ink.min.x.checked_sub(x)?;
            result.max.x = self.ink.max.x.checked_add(x)?;
        }
        if !self.constraint.vertical {
            let y = pad(&c, &a)?;
            result.min.y = self.ink.min.y.checked_sub(y)?;
            result.max.y = self.ink.max.y.checked_add(y)?;
        }
        Ok(result)
    }
}
pub(super) fn append(
    local: Option<LocalClip>,
    object: &SourcePagePaintBinding,
    builder: &mut SceneBuilder<SourcePagePaintSource>,
    viewport: &RasterViewport,
    check: &dyn Fn() -> bool,
) -> Result<(Option<u32>, Fixed, Fixed), SourcePageError> {
    cancel(check)?;
    let Some(local) = local else {
        return Ok((None, Fixed::ZERO, Fixed::ZERO));
    };
    let placement = object
        .placement
        .as_ref()
        .ok_or(SourcePageError::Invalid("text clip placement"))?;
    let rect = local.rectangle(&placement.affine, viewport)?;
    let commands = decorations::commands(rect);
    let (affine, position, geometry) = placement::transform(
        &commands,
        Point {
            x: Fixed::ZERO,
            y: Fixed::ZERO,
        },
        local.constraint.conversion_error_bound,
        object,
        viewport,
        check,
    )?;
    let clip = builder.clip(
        &FillPath {
            commands: commands.to_vec(),
            fill_rule: FillRule::Nonzero,
        },
        affine,
    )?;
    Ok((Some(clip), position, geometry))
}
