//! Common local-text placement and per-frame precision certification.
use super::*;
use mo_geometry::{Affine, PathCommand};
use mo_raster::Brush;

pub(crate) trait Painter {
    fn append(
        &mut self,
        binding: u32,
        object: &SourcePagePaintBinding,
        builder: &mut SceneBuilder<SourcePagePaintSource>,
        viewport: &RasterViewport,
        check: &dyn Fn() -> bool,
    ) -> Result<(Fixed, Fixed), SourcePageError>;
}
impl Painter for Compiler<'_, '_, '_> {
    fn append(
        &mut self,
        binding: u32,
        object: &SourcePagePaintBinding,
        builder: &mut SceneBuilder<SourcePagePaintSource>,
        viewport: &RasterViewport,
        check: &dyn Fn() -> bool,
    ) -> Result<(Fixed, Fixed), SourcePageError> {
        Compiler::append(self, binding, object, builder, viewport, check)
    }
}
pub(super) struct LocalPath<'a> {
    pub commands: &'a [PathCommand],
    pub origin: Point,
    pub rgba: [u8; 4],
    pub uncertainty: Fixed,
    pub clip: Option<u32>,
}
pub(super) fn paint(
    binding: u32,
    object: &SourcePagePaintBinding,
    builder: &mut SceneBuilder<SourcePagePaintSource>,
    viewport: &RasterViewport,
    local: LocalPath<'_>,
    check: &dyn Fn() -> bool,
) -> Result<(u32, Fixed, Fixed), SourcePageError> {
    let LocalPath {
        commands,
        origin,
        rgba,
        uncertainty,
        clip,
    } = local;
    let (affine, position, geometry) =
        transform(commands, origin, uncertainty, object, viewport, check)?;
    let instance = builder.scene.instances.len() as u32;
    builder.add_clipped(
        commands,
        affine,
        Brush::Solid { rgba },
        None,
        clip,
        |instance| SourcePagePaintSource {
            instance,
            binding,
            path: None,
            paint: crate::PagePaintKind::Fill,
            fill_target: None,
        },
    )?;
    Ok((instance, position, geometry))
}

pub(crate) fn transform(
    commands: &[PathCommand],
    origin: Point,
    uncertainty: Fixed,
    object: &SourcePagePaintBinding,
    viewport: &RasterViewport,
    check: &dyn Fn() -> bool,
) -> Result<(Affine, Fixed, Fixed), SourcePageError> {
    let placement = object.placement.as_ref().expect("object placement");
    // Shape-local paths stay untransformed. Rebase and certify the sampled
    // world affine, including group sectors, on every frame.
    let origin = Point {
        x: origin.x.checked_sub(placement.anchor.x)?,
        y: origin.y.checked_sub(placement.anchor.y)?,
    };
    let translated = placement.affine.map(origin)?;
    let affine = Affine {
        linear: placement.affine.linear,
        translation: translated.point,
    };
    let position = crate::coordinate_budget::matrix_budget_at(
        commands,
        origin,
        &placement.uncertainty,
        viewport,
        check,
    )?;
    let bound = crate::coordinate_budget::geometry_budget(
        [0; 2].map(|_| crate::interval::Interval::fixed(uncertainty)),
        &placement.affine,
        &placement.uncertainty,
        viewport,
    )?;
    let translation_error = translated
        .error
        .x
        .max(translated.error.y)
        .ratio_up(viewport.scale.numerator, viewport.scale.denominator)?;
    let geometry = bound.checked_add(translation_error)?;
    Ok((affine, position, geometry))
}
