use crate::coordinate_budget::{axes, geometry_budget, matrix_budget};
use crate::path_scene::SceneBuilder;
use crate::{
    PAGE_PROFILE, PageError, PageFeature, PageImage, PagePaintKind, PagePaintSource, PagePlan,
    PagePlanInfo, PageRasterInfo, PageRenderRequest,
    interval::Interval as I,
    page_paint::{Paint, PaintContext},
    page_placements, shape_paths,
};
use mo_common::ObjectId;
use mo_geometry::{Affine, Fixed, PathCommand as C, Point};
use mo_presentation_model::{
    self as model, Color, ContainerId, Fill, Inherited, ObjectContent, Stroke,
};
use mo_raster::{PathRasterRequest, RasterBackend, RasterError};
use mo_render::SceneRasterRequest;
const ZERO: Point = Point {
    x: Fixed::ZERO,
    y: Fixed::ZERO,
};
fn cancel(check: &dyn Fn() -> bool) -> Result<(), PageError> {
    if check() {
        Err(RasterError::Cancelled.into())
    } else {
        Ok(())
    }
}
fn unsupported(object: Option<&ObjectId>, feature: PageFeature) -> PageError {
    PageError::Unsupported {
        object: object.cloned(),
        feature,
    }
}
fn add_author_paint(
    builder: &mut SceneBuilder<PagePaintSource>,
    commands: &[C],
    affine: Affine,
    paint: Paint,
    object: Option<ObjectId>,
    container: ContainerId,
) -> Result<(), PageError> {
    builder.add(
        commands,
        affine,
        mo_raster::Brush::Solid { rgba: paint.color },
        paint.stroke,
        |instance| PagePaintSource {
            object,
            container,
            instance,
            paint: if paint.stroke.is_some() {
                PagePaintKind::Stroke
            } else {
                PagePaintKind::Fill
            },
        },
    )?;
    Ok(())
}
fn build_page(
    request: &PageRenderRequest,
    placement: Option<&crate::PagePlacements>,
    check: &dyn Fn() -> bool,
) -> Result<(PagePlanInfo, SceneRasterRequest, Vec<PagePaintSource>), PageError> {
    cancel(check)?;
    let v = &request.viewport;
    mo_raster::compile(
        &PathRasterRequest {
            clips: vec![],
            viewport: v.clone(),
            paths: vec![],
            draws: vec![],
        },
        check,
    )?;
    let calculated;
    let placement = match placement {
        Some(p) => p,
        None => {
            calculated = page_placements(&request.page, check)?;
            &calculated
        }
    };
    let size = placement.page_size;
    if v.origin != ZERO
        || i128::from(size.width.get()) * i128::from(v.scale.numerator)
            != i128::from(v.width) * i128::from(v.scale.denominator)
        || i128::from(size.height.get()) * i128::from(v.scale.numerator)
            != i128::from(v.height) * i128::from(v.scale.denominator)
    {
        return Err(RasterError::Invalid(
            "page viewport must exactly cover the page at uniform scale",
        )
        .into());
    }
    let d = &request.page.document;
    let slide = &d.slides[&request.page.slide];
    let layout = slide.layout.as_ref().map(|id| &d.layouts[id]);
    let master = layout.map(|l| &d.masters[&l.master]);
    let context = PaintContext {
        defaults: &request.defaults,
        theme: master.map(|m| &d.themes[&m.theme]),
    };
    let background = std::iter::once(&slide.background)
        .chain(layout.map(|l| &l.background))
        .chain(master.map(|m| &m.background))
        .find_map(|f| match f {
            Inherited::Value(fill) => Some(fill),
            Inherited::Inherit => None,
        });
    let default_background = Fill::Solid {
        color: Color::Srgb {
            rgba: request.defaults.page_background,
        },
    };
    let mut builder = SceneBuilder::new();
    let mut curve_segments = 0;
    if let Some(paint) = context.fill(background.unwrap_or(&default_background), None)? {
        let w = Fixed::emu(size.width);
        let h = Fixed::emu(size.height);
        add_author_paint(
            &mut builder,
            &[
                C::Move { to: ZERO },
                C::Line {
                    to: Point {
                        x: w,
                        y: Fixed::ZERO,
                    },
                },
                C::Line {
                    to: Point { x: w, y: h },
                },
                C::Line {
                    to: Point {
                        x: Fixed::ZERO,
                        y: h,
                    },
                },
                C::Close,
            ],
            Affine::IDENTITY,
            paint,
            None,
            ContainerId::Slide(slide.id.clone()),
        )?;
    }
    let mut table = shape_paths::ArcTable::default();
    let mut author_error = Fixed::ZERO;
    let mut geometry_error = Fixed::ZERO;
    let mut miter_error = Fixed::ZERO;
    let mut objects = 0;
    for surface in &placement.surfaces {
        for p in &surface.objects {
            cancel(check)?;
            objects += 1;
            let o = &d.objects[&p.object];
            let geometry = match &o.content {
                ObjectContent::Group { .. } => {
                    if matches!(o.appearance.stroke, Inherited::Value(Stroke::Solid { .. })) {
                        return Err(unsupported(Some(&o.id), PageFeature::GroupStroke));
                    }
                    continue;
                }
                ObjectContent::Shape { geometry, text } => {
                    if text.is_some() {
                        return Err(unsupported(Some(&o.id), PageFeature::ShapeText));
                    }
                    geometry
                }
                ObjectContent::Picture { .. } => {
                    return Err(unsupported(Some(&o.id), PageFeature::Picture));
                }
                ObjectContent::Connector { .. } => {
                    return Err(unsupported(Some(&o.id), PageFeature::Connector));
                }
            };
            let stroke = match &o.appearance.stroke {
                Inherited::Value(stroke) => context.stroke(stroke, &o.id)?,
                Inherited::Inherit => {
                    return Err(unsupported(Some(&o.id), PageFeature::InheritedStroke));
                }
            };
            let fill = match &o.appearance.fill {
                Inherited::Value(fill) => fill,
                Inherited::Inherit => {
                    return Err(unsupported(Some(&o.id), PageFeature::InheritedFill));
                }
            };
            let fill = context.fill(fill, Some(&o.id))?;
            if fill.is_none() && stroke.is_none() {
                continue;
            }
            if let model::Geometry::RoundRectangle { radius } = geometry
                && i128::from(radius.get()) * 2
                    > i128::from(p.source_size.width.get().min(p.source_size.height.get()))
            {
                return Err(RasterError::Invalid(
                    "round rectangle radius exceeds half the short side",
                )
                .into());
            }
            let mut segments = 1;
            if let Some(radii) = shape_paths::radii(geometry, p.source_size) {
                loop {
                    let bound = geometry_budget(
                        shape_paths::curve_remainder(&radii, segments),
                        &p.affine,
                        &p.uncertainty,
                        v,
                    )?;
                    if bound.raw() <= v.coordinate_tolerance.raw() / 4 {
                        break;
                    }
                    if segments == 64 {
                        return Err(RasterError::Precision.into());
                    }
                    segments *= 2;
                }
            }
            let outline = shape_paths::outline(
                geometry,
                p.source_size,
                p.anchor,
                segments,
                &mut table,
                check,
            )?;
            author_error =
                author_error.max(matrix_budget(&outline.commands, &p.uncertainty, v, check)?);
            geometry_error = geometry_error.max(geometry_budget(
                axes(outline.error).map(I::fixed),
                &p.affine,
                &p.uncertainty,
                v,
            )?);
            curve_segments += outline.curve_segments;
            for paint in fill.into_iter().chain(stroke) {
                miter_error = miter_error.max(paint.miter_error);
                add_author_paint(
                    &mut builder,
                    &outline.commands,
                    p.affine,
                    paint,
                    Some(o.id.clone()),
                    surface.container.clone(),
                )?;
            }
        }
    }
    let remaining = v
        .coordinate_tolerance
        .raw()
        .checked_sub(author_error.raw())
        .and_then(|v| v.checked_sub(geometry_error.raw()))
        .filter(|v| *v >= 256)
        .ok_or(RasterError::Precision)?;
    let mut viewport = v.clone();
    viewport.coordinate_tolerance = Fixed::from_raw(remaining);
    let info = PagePlanInfo {
        profile: PAGE_PROFILE.into(),
        placement_profile: placement.profile.clone(),
        document_sha256: placement.document_sha256.clone(),
        slide: slide.id.clone(),
        hidden: slide.hidden,
        source_objects: objects,
        generated_commands: builder.generated_commands,
        curve_segments,
        author_coordinate_error_bound: author_error,
        geometry_coordinate_error_bound: geometry_error,
        author_miter_limit_error_bound: miter_error,
    };
    cancel(check)?;
    Ok((
        info,
        SceneRasterRequest {
            viewport,
            scene: builder.scene,
        },
        builder.sources,
    ))
}
pub(crate) fn prepare_page(
    request: &PageRenderRequest,
    placements: Option<&crate::PagePlacements>,
    check: &dyn Fn() -> bool,
) -> Result<(PagePlan, mo_render::CompiledScene), PageError> {
    let (info, raster, paint_sources) = build_page(request, placements, check)?;
    let compiled = mo_render::compile(&raster, check)?;
    let total = info
        .author_coordinate_error_bound
        .raw()
        .checked_add(info.geometry_coordinate_error_bound.raw())
        .and_then(|v| v.checked_add(compiled.work().combined_coordinate_error_bound.raw()))
        .ok_or(RasterError::Range)?;
    if total > request.viewport.coordinate_tolerance.raw() {
        return Err(RasterError::Precision.into());
    }
    let plan = PagePlan {
        info,
        raster,
        paint_sources,
        device_work: compiled.work().clone(),
        combined_coordinate_error_bound: Fixed::from_raw(total),
    };
    Ok((plan, compiled))
}
/// Compile and certify the current author subset through device lowering.
/// No backend is called and unsupported content never produces a partial page.
pub fn compile_page(
    request: &PageRenderRequest,
    check: &dyn Fn() -> bool,
) -> Result<PagePlan, PageError> {
    prepare_page(request, None, check).map(|(plan, _)| plan)
}
pub fn render_page(
    request: &PageRenderRequest,
    backend: &mut dyn RasterBackend,
    check: &dyn Fn() -> bool,
) -> Result<PageImage, PageError> {
    let (plan, compiled) = prepare_page(request, None, check)?;
    let image = mo_render::render_compiled(compiled, backend, check)?;
    Ok(PageImage {
        info: PageRasterInfo {
            page: plan.info,
            scene: image.info,
            combined_coordinate_error_bound: plan.combined_coordinate_error_bound,
        },
        pixels: image.pixels,
    })
}
