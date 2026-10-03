//! One paint-order engine for plain, text, and image-resource source pages.
use super::*;
use super::{paint::FillPaint, prepared::filled};
use crate::path_scene::ScenePaint;
use crate::source_image_paint::NativeImagePaint;
use mo_raster::StrokeStyle;
use std::collections::BTreeMap;
pub(crate) type ImagePaints = BTreeMap<FillOwner, NativeImagePaint>;
struct Emitter<'a> {
    builder: SceneBuilder<SourcePagePaintSource>,
    images: &'a ImagePaints,
    clips: BTreeMap<FillOwner, u32>,
    clip_error: Fixed,
    viewport: &'a mo_raster::RasterViewport,
    background_prefix: u32,
    opaque_background: Option<[u8; 4]>,
}
struct PaintPath<'a> {
    commands: &'a [C],
    affine: Affine,
    origin: Option<mo_presentation_source::source::geometry::evaluate::GeometryOrigin>,
}
impl<'a> PaintPath<'a> {
    fn native(path: &'a CompiledNativePath, affine: Affine) -> Self {
        Self {
            commands: &path.commands,
            affine,
            origin: Some(path.origin),
        }
    }
}
impl Emitter<'_> {
    fn fill(
        &mut self,
        path: PaintPath<'_>,
        binding: u32,
        b: &SourcePagePaintBinding,
        paint: &FillPaint,
        picture: bool,
    ) -> Result<(), SourcePageError> {
        // Preflight has already admitted the geometry, fill and resource
        // bindings. A proven zero-area fill needs no inverse paint matrix.
        // Keep the object/resource binding and text traversal for this frame.
        if b.placement
            .as_ref()
            .is_some_and(|p| crate::placement_core::empty_fill(&p.affine, &p.uncertainty))
        {
            return Ok(());
        }
        let target = if picture {
            b.picture_fill
                .as_ref()
                .expect("picture fill")
                .target
                .clone()
        } else {
            b.fill.target.clone()
        };
        let mut clip = None;
        let mut blend = mo_raster::BlendMode::SourceOver;
        let brush = match paint {
            FillPaint::Solid(rgba) => Brush::Solid { rgba: *rgba },
            FillPaint::Gradient(gradient) => Brush::Gradient {
                gradient: gradient.as_ref().clone(),
            },
            FillPaint::Background => match self.opaque_background {
                Some(rgba) => Brush::Solid { rgba },
                None => {
                    blend = mo_raster::BlendMode::Source;
                    Brush::Snapshot {
                        after_draws: self.background_prefix,
                        scope: mo_raster::SnapshotScope::Output,
                    }
                }
            },
            FillPaint::Image => {
                let owner = FillOwner {
                    part: b.location.part.clone(),
                    target: target.clone(),
                };
                let image = self
                    .images
                    .get(&owner)
                    .ok_or(SourcePageError::Invalid("unbound page image"))?;
                if let Some(c) = &image.fill_clip {
                    let error = c.upstream_error.x.max(c.upstream_error.y).ratio_up(
                        self.viewport.scale.numerator,
                        self.viewport.scale.denominator,
                    )?;
                    self.clip_error = self.clip_error.max(error);
                    clip = Some(if let Some(id) = self.clips.get(&owner) {
                        *id
                    } else {
                        let id = self.builder.clip(&c.path, c.affine)?;
                        self.clips.insert(owner, id);
                        id
                    });
                }
                Brush::Image {
                    image: image.brush.clone(),
                }
            }
        };
        self.builder.paint(
            path.commands,
            path.affine,
            ScenePaint {
                brush,
                stroke: None,
                clip,
                blend,
            },
            |instance| SourcePagePaintSource {
                chart: None,
                instance,
                binding,
                path: path.origin,
                paint: crate::PagePaintKind::Fill,
                fill_target: (picture || b.region.is_some()).then_some(target),
            },
        )?;
        Ok(())
    }
    fn stroke(
        &mut self,
        path: &CompiledNativePath,
        affine: Affine,
        binding: u32,
        target: &SourcePagePaintBinding,
        line: Option<([u8; 4], StrokeStyle)>,
    ) -> Result<(), SourcePageError> {
        if let Some((rgba, stroke)) = line.filter(|_| path.stroke != Some(false)) {
            self.builder.add(
                &path.commands,
                affine,
                Brush::Solid { rgba },
                Some(stroke),
                |instance| SourcePagePaintSource {
                    chart: None,
                    instance,
                    binding,
                    path: Some(path.origin),
                    paint: crate::PagePaintKind::Stroke,
                    fill_target: target
                        .table_stroke
                        .as_ref()
                        .map(|_| target.fill.target.clone()),
                },
            )?;
        }
        Ok(())
    }
}
pub(crate) fn build(
    mut page: PreparedPage<'_>,
    mut text: Option<&mut dyn crate::source_text_page::Painter>,
    images: &ImagePaints,
    check: &dyn Fn() -> bool,
) -> Result<BuiltPage, SourcePageError> {
    let mut emit = Emitter {
        builder: SceneBuilder::new(),
        images,
        clips: BTreeMap::new(),
        clip_error: Fixed::ZERO,
        viewport: &page.viewport,
        background_prefix: 0,
        // Exact opaque constants need no capture. Every other background is
        // replayed from its actual pixels, with one source replacement and one
        // shape mask, including alpha and the background's image-fill clip.
        opaque_background: match &page.background_paint {
            Some(FillPaint::Solid(rgba)) if rgba[3] == 255 => Some(*rgba),
            None if page.viewport.background[3] == 255 => Some(page.viewport.background),
            _ => None,
        },
    };
    if page.clip_page {
        emit.builder.set_page_clip(&page.background_path)?;
    }
    if let Some(paint) = &page.background_paint {
        emit.fill(
            PaintPath {
                commands: &page.background_path,
                affine: Affine::IDENTITY,
                origin: None,
            },
            0,
            &page.background,
            paint,
            false,
        )?;
    }
    emit.background_prefix = emit.builder.scene.instances.len() as u32;
    let mut bindings = vec![page.background];
    let mut scopes = crate::opacity_scopes::OpacityScopes::new();
    for object in page.objects {
        cancel(check)?;
        scopes.enter(&object.opacity, emit.builder.scene.instances.len() as u32)?;
        let binding = bindings.len() as u32;
        bindings.push(object.binding);
        let b = &bindings[binding as usize];
        if let Some(chart) = page.charts.get(&(
            b.location.part.clone(),
            b.location.object.expect("object id"),
        )) {
            let (position, geometry) =
                chart.emit(binding, b, &mut emit.builder, &page.viewport, check)?;
            page.info.placement_coordinate_error_bound =
                page.info.placement_coordinate_error_bound.max(position);
            page.info.path_coordinate_error_bound =
                page.info.path_coordinate_error_bound.max(geometry);
        }
        for receiver in object.paints {
            let binding = if let Some(owner) = receiver.binding {
                let id = bindings.len() as u32;
                bindings.push(owner);
                id
            } else {
                binding
            };
            let b = &bindings[binding as usize];
            let affine = b.placement.as_ref().expect("object placement").affine;
            // ECMA-376-1 19.3.1.4 shows the spPr fill through transparent pixels of
            // p:blipFill. Preserve both; do not replace either declaration. Picture
            // fill is above the shape fill and below its outline.
            if b.picture_fill.is_some() {
                for (paint, picture) in [(&receiver.fill, false), (&receiver.picture_fill, true)] {
                    if let Some(paint) = paint {
                        for path in receiver.paths.iter().filter(|p| filled(p)) {
                            cancel(check)?;
                            emit.fill(PaintPath::native(path, affine), binding, b, paint, picture)?;
                        }
                    }
                }
                for path in &receiver.paths {
                    emit.stroke(path, affine, binding, b, receiver.line)?;
                }
            } else {
                // Retain native path-local fill/outline order for ordinary shapes.
                for path in &receiver.paths {
                    cancel(check)?;
                    if let Some(paint) = receiver.fill.as_ref().filter(|_| filled(path)) {
                        emit.fill(PaintPath::native(path, affine), binding, b, paint, false)?;
                    }
                    emit.stroke(path, affine, binding, b, receiver.line)?;
                }
            }
        }
        let b = &bindings[binding as usize];
        if let Some(text) = text.as_deref_mut() {
            let (position, geometry) = text
                .append(binding, b, &mut emit.builder, &page.viewport, check)
                .map_err(|e| e.at(&b.location))?;
            page.info.placement_coordinate_error_bound =
                page.info.placement_coordinate_error_bound.max(position);
            page.info.path_coordinate_error_bound =
                page.info.path_coordinate_error_bound.max(geometry);
        }
    }
    emit.builder.scene.opacity_groups = scopes.finish(emit.builder.scene.instances.len() as u32);
    let remaining = page
        .viewport
        .coordinate_tolerance
        .raw()
        .checked_sub(page.info.placement_coordinate_error_bound.raw())
        .and_then(|n| n.checked_sub(page.info.path_coordinate_error_bound.raw()))
        .and_then(|n| n.checked_sub(emit.clip_error.raw()))
        .filter(|n| *n >= 256)
        .ok_or(RasterError::Precision)?;
    page.info.image_clip_coordinate_error_bound = emit.clip_error;
    page.info.generated_commands = emit.builder.generated_commands;
    let mut viewport = page.viewport.clone();
    viewport.coordinate_tolerance = Fixed::from_raw(remaining);
    cancel(check)?;
    Ok(BuiltPage {
        info: page.info,
        raster: SceneRasterRequest {
            viewport,
            scene: emit.builder.scene,
        },
        bindings,
        paint_sources: emit.builder.sources,
        requested_tolerance: page.viewport.coordinate_tolerance,
    })
}
