//! Native page computation. Never creates a lossy author document or reads a
//! host resource. Unsupported visible content aborts before any backend call.
mod emit;
mod gradient;
mod gradient_circle;
mod gradient_rect;
mod layers;
mod objects;
mod opacity;
mod paint;
mod prepared;
mod table;
mod types;
use crate::{
    coordinate_budget::{axes, geometry_budget, matrix_budget},
    interval::Interval as I,
    native_paths::*,
    path_scene::SceneBuilder,
    source_placement::*,
};
use mo_geometry::{Affine, Fixed, PathCommand as C, Point};
use mo_presentation_source::source::{
    SourceIndex,
    color::ColorProfile,
    fill::{colors::*, resolve::*},
    geometry::{NativePathFill, evaluate::*},
    line::{colors::*, resolve::*},
};
use mo_raster::{Brush, RasterBackend, RasterError};
use mo_render::SceneRasterRequest;
use objects::Object;
pub(crate) use prepared::{BuiltPage, PreparedPage};
pub use types::*;
const ZERO: Point = Point {
    x: Fixed::ZERO,
    y: Fixed::ZERO,
};
fn cancel(check: &dyn Fn() -> bool) -> Result<(), SourcePageError> {
    if check() {
        Err(RasterError::Cancelled.into())
    } else {
        Ok(())
    }
}
fn mapping(location: &SourcePageLocation, issue: SourcePageIssue) -> SourcePageError {
    SourcePageError::Mapping {
        location: location.clone(),
        issue: Box::new(issue),
    }
}
fn local_tolerance(
    objects: &[Object],
    q: &SourcePageRequest,
    check: &dyn Fn() -> bool,
) -> Result<Fixed, SourcePageError> {
    let placements = objects.iter().map(|object| {
        let p = object.binding.placement.as_ref().expect("object placement");
        (&p.affine, &p.uncertainty)
    });
    Ok(crate::coordinate_budget::local_tolerance(
        placements,
        &q.viewport,
        check,
    )?)
}
fn shifted(
    commands: &mut [C],
    anchor: Point,
    check: &dyn Fn() -> bool,
) -> Result<(), SourcePageError> {
    let shift = |p: &mut Point| -> Result<(), SourcePageError> {
        p.x = Fixed::from_raw(
            p.x.raw()
                .checked_sub(anchor.x.raw())
                .ok_or(RasterError::Range)?,
        );
        p.y = Fixed::from_raw(
            p.y.raw()
                .checked_sub(anchor.y.raw())
                .ok_or(RasterError::Range)?,
        );
        Ok(())
    };
    for c in commands {
        cancel(check)?;
        match c {
            C::Move { to } | C::Line { to } => shift(to)?,
            C::Quadratic { control, to } => {
                shift(control)?;
                shift(to)?;
            }
            C::Cubic {
                control1,
                control2,
                to,
            } => {
                shift(control1)?;
                shift(control2)?;
                shift(to)?;
            }
            C::Close => (),
        }
    }
    Ok(())
}
pub(crate) fn prepare(
    index: &SourceIndex,
    q: &SourcePageRequest,
    mut text: Option<&mut crate::source_text_page::Compiler<'_, '_, '_>>,
    check: &dyn Fn() -> bool,
) -> Result<(SourcePagePlan, mo_render::CompiledScene), SourcePageError> {
    let mut prepared = prepared::preflight(index, q, text.is_some(), false, check)?;
    if let Some(text) = text.as_deref_mut() {
        text.preflight_shared(
            index,
            q,
            prepared.objects.iter().map(|o| &o.binding),
            &prepared.tables,
            check,
        )?;
    }
    prepared.tables.clear();
    prepared.source = None;
    let built = emit::build(
        prepared,
        text.map(|t| t as &mut dyn crate::source_text_page::Painter),
        &std::collections::BTreeMap::new(),
        check,
    )?;
    let compiled = mo_render::compile(&built.raster, check)?;
    let plan = built.finish(compiled.work().clone(), Fixed::ZERO)?;
    cancel(check)?;
    Ok((plan, compiled))
}
pub(crate) use emit::build;
pub(crate) use prepared::{preflight_retained, preflight_sampled};
pub fn compile(
    index: &SourceIndex,
    request: &SourcePageRequest,
    check: &dyn Fn() -> bool,
) -> Result<SourcePagePlan, SourcePageError> {
    prepare(index, request, None, check).map(|(plan, _)| plan)
}
pub fn render(
    index: &SourceIndex,
    request: &SourcePageRequest,
    backend: &mut dyn RasterBackend,
    check: &dyn Fn() -> bool,
) -> Result<SourcePageImage, SourcePageError> {
    let (plan, compiled) = prepare(index, request, None, check)?;
    let image = mo_render::render_compiled(compiled, backend, check)?;
    Ok(SourcePageImage {
        info: SourcePageRasterInfo {
            page: plan.info,
            scene: image.info,
            downstream_coordinate_error_bound: plan.downstream_coordinate_error_bound,
        },
        pixels: image.pixels,
    })
}
