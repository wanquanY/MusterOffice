//! Circle layout consumes original local paths exactly once, before rebasing.
use super::*;
use crate::radial_layout::{self as radial, CircleFocusBasis, NativeRadialLayout};
use mo_geometry::BoundsBudget;
use mo_raster::GradientField;

pub(super) fn required(result: &SourceFillColorResult, at: &SourcePageLocation) -> bool {
    !paint::background_redirect(result, at)
        && matches!(&result.colors, FillPaintColors::Gradient { .. })
        && matches!(&result.style, FillOutcome::Resolved { fill, .. }
            if matches!(fill.as_ref(), EffectiveFill::Gradient { gradient, .. }
                if matches!(gradient.shade, EffectiveGradientShade::Path { ref path, .. }
                    if path.value == mo_presentation_source::source::fill::NativePathShade::Circle)))
}

pub(super) struct LayoutContext<'a> {
    pub page_size: mo_presentation_model::Size,
    pub paths: Option<&'a [CompiledNativePath]>,
    pub tolerance: Fixed,
    pub budget: &'a mut BoundsBudget,
}
impl LayoutContext<'_> {
    pub fn layout(
        &mut self,
        result: &SourceFillColorResult,
        at: &SourcePageLocation,
        check: &dyn Fn() -> bool,
    ) -> Result<Option<NativeRadialLayout>, SourcePageError> {
        if !required(result, at) {
            return Ok(None);
        }
        let FillOutcome::Resolved { fill, .. } = &result.style else {
            unreachable!()
        };
        let EffectiveFill::Gradient { gradient, .. } = fill.as_ref() else {
            unreachable!()
        };
        if at.object.is_some() && !gradient.rotate_with_shape.value {
            return Err(SourcePageError::Invalid(
                "native stationary gradient orientation required",
            )
            .at(at));
        }
        let result = if at.object.is_some() {
            radial::layout_paths_with_basis(
                gradient,
                self.paths
                    .ok_or(SourcePageError::Invalid("circle gradient paths missing"))?,
                radial::RadialLayoutOptions {
                    coordinate_tolerance: self.tolerance,
                },
                self.budget,
                CircleFocusBasis::AnchorRectangle,
                check,
            )
        } else {
            radial::layout_background_with_basis(
                gradient,
                self.page_size,
                CircleFocusBasis::AnchorRectangle,
                check,
            )
        };
        result
            .map(Some)
            .map_err(|e| SourcePageError::RadialLayout(e).at(at))
    }
}

pub(super) fn geometry(
    layout: &NativeRadialLayout,
) -> Result<([I; 2], I, I, GradientField), SourcePageError> {
    let enclosure = |v: Fixed, e: Fixed| super::gradient::enclose(v, e);
    let r: [I; 4] = std::array::from_fn(|i| {
        enclosure(
            layout.tile_rectangle.values[i],
            layout.tile_rectangle.errors[i],
        )
    });
    let radius = enclosure(layout.outer_radius.values[0], layout.outer_radius.errors[0]);
    let w = r[2].sub(&r[0]);
    let h = r[3].sub(&r[1]);
    let diameter = radius.mul(&I::integer(2));
    let normalized = |v: &I, d: &I| v.div_positive(d).map_err(super::gradient::numeric);
    let mut p = [Fixed::ZERO; 6];
    let mut e = p;
    for (i, v) in [normalized(&w, &diameter)?, normalized(&h, &diameter)?]
        .iter()
        .enumerate()
    {
        (p[i], e[i]) = v.q32().map_err(super::gradient::numeric)?;
    }
    for i in 0..2 {
        let center = enclosure(layout.inner_center.values[i], layout.inner_center.errors[i]);
        let outer = enclosure(layout.outer_center.values[i], layout.outer_center.errors[i]);
        (p[i + 2], e[i + 2]) = normalized(&center.sub(&outer), &radius)?
            .q32()
            .map_err(super::gradient::numeric)?;
        // The layout already carries exact-ratio scale. Re-dividing radius*S
        // by its independently enclosed radius would lose their correlation.
        p[i + 4] = layout.focus_scale.values[i];
        e[i + 4] = layout.focus_scale.errors[i];
    }
    Ok((
        [r[0].clone(), r[1].clone()],
        w,
        h,
        GradientField::Elliptic {
            tile_scale: [p[0], p[1]],
            inner_center: [p[2], p[3]],
            inner_radii: [p[4], p[5]],
            uncertainty: Some(Box::new(e)),
        },
    ))
}
