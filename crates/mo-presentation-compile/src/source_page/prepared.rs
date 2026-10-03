//! Pure source preflight. All visible declarations and native geometry are
//! checked before image decoding or text shaping can call a host component.
use super::paint::FillPaint;
use super::*;
use mo_raster::StrokeStyle;
use std::collections::BTreeMap;

pub(crate) struct PreparedObject {
    pub opacity: super::opacity::Chain,
    pub binding: SourcePagePaintBinding,
    pub paints: Vec<PreparedPaint>,
}
/// Ordered paint receivers belonging to one real source object. None uses the
/// object's base binding; a cell/edge has its own native fill target binding.
pub(crate) struct PreparedPaint {
    pub binding: Option<SourcePagePaintBinding>,
    pub paths: Vec<CompiledNativePath>,
    pub fill: Option<FillPaint>,
    pub picture_fill: Option<FillPaint>,
    pub line: Option<([u8; 4], StrokeStyle)>,
}
pub(crate) struct PreparedPage<'a> {
    pub source: Option<mo_presentation_source::source::prepared::SourcePreparation<'a>>,
    pub tables: crate::source_table::SharedTables<'a>,
    pub info: SourcePageInfo,
    pub viewport: mo_raster::RasterViewport,
    pub background: SourcePagePaintBinding,
    pub background_paint: Option<FillPaint>,
    pub background_path: Vec<C>,
    pub clip_page: bool,
    pub objects: Vec<PreparedObject>,
}
pub(crate) struct BuiltPage {
    pub info: SourcePageInfo,
    pub raster: SceneRasterRequest,
    pub bindings: Vec<SourcePagePaintBinding>,
    pub paint_sources: Vec<SourcePagePaintSource>,
    pub requested_tolerance: Fixed,
}
impl BuiltPage {
    pub fn finish(
        self,
        work: mo_render::SceneWork,
        paint_error: Fixed,
    ) -> Result<SourcePagePlan, SourcePageError> {
        let total = self
            .info
            .placement_coordinate_error_bound
            .raw()
            .checked_add(self.info.path_coordinate_error_bound.raw())
            .and_then(|n| n.checked_add(self.info.image_clip_coordinate_error_bound.raw()))
            .and_then(|n| {
                n.checked_add(work.combined_coordinate_error_bound.max(paint_error).raw())
            })
            .ok_or(RasterError::Range)?;
        if total > self.requested_tolerance.raw() {
            return Err(RasterError::Precision.into());
        }
        Ok(SourcePagePlan {
            info: self.info,
            raster: self.raster,
            bindings: self.bindings,
            paint_sources: self.paint_sources,
            device_work: work,
            downstream_coordinate_error_bound: Fixed::from_raw(total),
        })
    }
}
/// Whether a compiled geometry path paints the ordinary fill and picture fill.
pub(super) fn filled(path: &CompiledNativePath) -> bool {
    !matches!(path.fill, Some(NativePathFill::None))
}
pub(crate) fn preflight<'a>(
    index: &'a SourceIndex,
    q: &SourcePageRequest,
    text_enabled: bool,
    images_enabled: bool,
    check: &dyn Fn() -> bool,
) -> Result<PreparedPage<'a>, SourcePageError> {
    preflight_sampled(index, q, text_enabled, images_enabled, None, check)
}
pub(crate) fn preflight_sampled<'a>(
    index: &'a SourceIndex,
    q: &SourcePageRequest,
    text_enabled: bool,
    images_enabled: bool,
    transforms: Option<&crate::source_placement::SourceProperties>,
    check: &dyn Fn() -> bool,
) -> Result<PreparedPage<'a>, SourcePageError> {
    preflight_retained(
        index,
        q,
        text_enabled,
        images_enabled,
        transforms,
        None,
        check,
    )
}
pub(crate) fn preflight_retained<'a>(
    index: &'a SourceIndex,
    q: &SourcePageRequest,
    text_enabled: bool,
    images_enabled: bool,
    transforms: Option<&crate::source_placement::SourceProperties>,
    retained: Option<&'a crate::source_table::RetainedTables>,
    check: &dyn Fn() -> bool,
) -> Result<PreparedPage<'a>, SourcePageError> {
    cancel(check)?;
    if retained.is_some_and(|r| !std::ptr::eq(r.source.index(), index)) {
        return Err(SourcePageError::SourceConflict);
    }
    if index.source_sha256 != q.expected_source_sha256 {
        return Err(SourcePageError::SourceConflict);
    }
    let v = &q.viewport;
    v.validate()?;
    let size = index
        .page_size
        .ok_or(SourcePageError::Invalid("missing page size"))?;
    let clip_page = crate::page_boundary::validate(size, v)?;
    let layers = layers::select(index, q, text_enabled, images_enabled, transforms, check)?;
    let mut tables = super::table::admit(index, &layers, retained, check)?;
    let mut source = if tables.is_empty() {
        None
    } else if let Some(retained) = retained {
        Some(retained.source.session(check)?)
    } else {
        Some(
            mo_presentation_source::source::prepared::SourcePreparation::new(
                index,
                &q.expected_source_sha256,
                Default::default(),
                check,
            )?,
        )
    };
    let objects = objects::objects(index, q, &layers, transforms, source.as_mut(), check)?;
    let mut shared_tables = BTreeMap::new();
    let mut opacities = super::opacity::chains(index, &layers, transforms, check)?;
    let tolerance = local_tolerance(&objects, q, check)?;
    // Bounds affect both tile placement and normalized focus geometry. Reserve
    // finer local geometry for circle receivers; the final device verifier
    // still enforces the original viewport budget without relaxation.
    let radial_tolerance = Fixed::from_raw(tolerance.raw() / 16);
    let mut bounds_budget = mo_geometry::BoundsBudget::new(4_000_000);
    let mut compiler = NativePathCompiler::new(
        NativePathOptions {
            profile: NativePathProfile::DrawingmlPolarArcsDraftV1,
            coordinate_tolerance: tolerance,
        },
        NativePathLimits::default(),
        check,
    )?;
    let location = SourcePageLocation {
        part: q.slide.clone(),
        object: None,
    };
    let mut background = mo_presentation_source::source::fill::colors::query(
        index,
        &SourceFillColorQuery {
            expected_source_sha256: q.expected_source_sha256.clone(),
            surface: q.slide.clone(),
            targets: vec![FillTarget::Background {}],
            fill_profile: FillProfile::Drawingml2024DraftV1,
            color_profile: ColorProfile::Ecma3762016DraftV1,
            context: q.color_context.clone(),
        },
        FillColorLimits::default(),
        check,
    )?
    .targets;
    if background.len() != 1 {
        return Err(SourcePageError::Invalid("background query cardinality"));
    }
    let fill = background.pop().expect("one background");
    let background_radial = gradient_circle::LayoutContext {
        page_size: size,
        paths: None,
        tolerance,
        budget: &mut bounds_budget,
    }
    .layout(&fill, &location, check)?;
    let background_paint = paint::fill(
        &fill,
        &location,
        images_enabled,
        None,
        size,
        background_radial.as_ref(),
        None,
        check,
    )?;
    let mut paint_budget = mo_raster::PaintBudget::default();
    if let Some(paint) = &background_paint {
        paint_budget.include_draw(paint.gradient_stops())?;
    }
    let background = SourcePagePaintBinding {
        location,
        drawing_surface: q.slide.clone(),
        fill,
        picture_fill: None,
        line: None,
        placement: None,
        region: None,
        table_stroke: None,
    };
    let w = Fixed::emu(size.width);
    let h = Fixed::emu(size.height);
    let background_path = vec![
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
    ];
    let mut prepared = vec![];
    let mut placement_error = Fixed::ZERO;
    let mut path_error = Fixed::ZERO;
    let mut arc_segments = 0u32;
    for object in objects {
        cancel(check)?;
        let mut b = object.binding;
        let paints = if let Some(geometry) = &object.geometry {
            // Only circle fills need geometry before paint resolution. Reuse these
            // unrebased paths below; keep the previous lazy path order for all
            // previously supported paints, including background windows.
            let needs_bounds = gradient_circle::required(&b.fill, &b.location)
                || b.picture_fill
                    .as_ref()
                    .is_some_and(|f| gradient_circle::required(f, &b.location));
            let original_paths = if needs_bounds {
                Some(compiler.compile_with_tolerance(geometry, radial_tolerance)?)
            } else {
                None
            };
            let mut radial_context = gradient_circle::LayoutContext {
                page_size: size,
                paths: original_paths.as_deref(),
                tolerance: radial_tolerance,
                budget: &mut bounds_budget,
            };
            let radial = radial_context.layout(&b.fill, &b.location, check)?;
            let fill = paint::fill(
                &b.fill,
                &b.location,
                images_enabled,
                b.placement.as_ref(),
                size,
                radial.as_ref(),
                None,
                check,
            )?;
            let picture_fill = b
                .picture_fill
                .as_ref()
                .map(|f| {
                    let radial = radial_context.layout(f, &b.location, check)?;
                    paint::fill(
                        f,
                        &b.location,
                        images_enabled,
                        b.placement.as_ref(),
                        size,
                        radial.as_ref(),
                        None,
                        check,
                    )
                })
                .transpose()?
                .flatten();
            let line = paint::line(b.line.as_ref().expect("object line"), &b.location)?;
            let p = b.placement.as_ref().expect("object placement");
            let mut paths = vec![];
            if fill.is_some() || picture_fill.is_some() || line.is_some() {
                let original_paths = match original_paths {
                    Some(paths) => paths,
                    None => compiler.compile(geometry)?,
                };
                for mut path in original_paths {
                    // A move-only contour has no ink, even with stroke caps. Keep
                    // its declaration and compilation work, but do not quantize a
                    // distant non-drawing cursor into the device coordinate range.
                    if path.commands.iter().all(|c| matches!(c, C::Move { .. })) {
                        continue;
                    }
                    shifted(&mut path.commands, p.anchor, check)?;
                    if !matches!(
                        path.fill,
                        None | Some(NativePathFill::Normal | NativePathFill::None)
                    ) && (fill.is_some() || picture_fill.is_some())
                    {
                        return Err(mapping(&b.location, SourcePageIssue::PathFillModifier {}));
                    }
                    if !(filled(&path) && (fill.is_some() || picture_fill.is_some())
                        || path.stroke != Some(false) && line.is_some())
                    {
                        continue;
                    }
                    paths.push(path);
                }
            }
            vec![PreparedPaint {
                binding: None,
                paths,
                fill,
                picture_fill,
                line,
            }]
        } else {
            let limits = tables
                .remove(&(
                    b.location.part.clone(),
                    b.location.object.expect("table id"),
                ))
                .ok_or(SourcePageError::Invalid("unadmitted native table"))?;
            let source = source
                .as_mut()
                .ok_or(SourcePageError::Invalid("missing table preparation"))?;
            let reference = mo_presentation_source::source::SourceObjectRef {
                part: b.location.part.clone(),
                native_id: b.location.object.expect("table id"),
            };
            let native = match source.table(&reference, check)? {
                mo_presentation_source::source::prepared::SourceTablePreparation::Prepared {
                    table,
                } => table,
                mo_presentation_source::source::prepared::SourceTablePreparation::InvalidGrid {
                    reason,
                    ..
                } => return Err(crate::source_frame::SourceFrameError::from(
                    crate::source_table::TableGeometryError::Grid(
                        mo_presentation_source::source::table::grid::NativeTableGridError::Invalid(
                            reason,
                        ),
                    ),
                )
                .into()),
            };
            let geometry = std::sync::Arc::new(if let Some(retained) = retained {
                retained.geometry(&native, limits, check)?
            } else {
                crate::source_table::DeclaredTableGeometry::from_prepared(&native, limits, check)
                    .map_err(crate::source_frame::SourceFrameError::from)?
            });
            let table = crate::source_table::SharedTableLayout {
                source: native,
                geometry,
            };
            let paints = super::table::prepare(
                source,
                q,
                &mut b,
                images_enabled,
                tolerance,
                &mut bounds_budget,
                &table.geometry,
                check,
            )?;
            shared_tables.insert((reference.part, reference.native_id), table);
            paints
        };
        let p = b.placement.as_ref().expect("object placement");
        for receiver in &paints {
            let (fill, picture_fill, line) =
                (&receiver.fill, &receiver.picture_fill, &receiver.line);
            for path in &receiver.paths {
                // Admit logical paint work before constructing derived scene
                // instances or allowing image decode/text shaping to start.
                if filled(path) {
                    for paint in [fill, picture_fill].into_iter().flatten() {
                        paint_budget.include_draw(paint.gradient_stops())?;
                    }
                }
                if path.stroke != Some(false) && line.is_some() {
                    paint_budget.include_draw(0)?;
                }
                placement_error =
                    placement_error.max(matrix_budget(&path.commands, &p.uncertainty, v, check)?);
                path_error = path_error.max(geometry_budget(
                    axes(path.coordinate_error_bound).map(I::fixed),
                    &p.affine,
                    &p.uncertainty,
                    v,
                )?);
                arc_segments = arc_segments
                    .checked_add(path.arc_segments)
                    .ok_or(RasterError::Range)?;
            }
        }
        prepared.push(PreparedObject {
            opacity: opacities
                .remove(&(
                    b.location.part.clone(),
                    b.location.object.expect("object identity"),
                ))
                .unwrap_or_default(),
            binding: b,
            paints,
        });
    }
    cancel(check)?;
    Ok(PreparedPage {
        source,
        tables: shared_tables,
        info: SourcePageInfo {
            profile: q.profile,
            source_sha256: index.source_sha256.clone(),
            slide: q.slide.clone(),
            hidden_slide: index.surfaces[&q.slide].hidden,
            layers,
            generated_commands: 0,
            arc_segments,
            placement_coordinate_error_bound: placement_error,
            path_coordinate_error_bound: path_error,
            image_clip_coordinate_error_bound: Fixed::ZERO,
        },
        viewport: v.clone(),
        background,
        background_paint,
        background_path,
        clip_page,
        objects: prepared,
    })
}

impl PreparedPage<'_> {
    pub fn image_uses(&self) -> Vec<(u32, &SourcePagePaintBinding, &SourceFillColorResult)> {
        let mut uses = vec![];
        if matches!(self.background_paint, Some(FillPaint::Image)) {
            uses.push((0, &self.background, &self.background.fill));
        }
        let mut next = 1u32;
        for object in &self.objects {
            let base = next;
            next += 1;
            for paint in &object.paints {
                let (id, owner) = match &paint.binding {
                    Some(binding) => {
                        let id = next;
                        next += 1;
                        (id, binding)
                    }
                    None => (base, &object.binding),
                };
                if !paint.paths.iter().any(filled) {
                    continue;
                }
                if matches!(paint.fill, Some(FillPaint::Image)) {
                    uses.push((id, owner, &owner.fill));
                }
                if matches!(paint.picture_fill, Some(FillPaint::Image)) {
                    uses.push((
                        id,
                        owner,
                        owner.picture_fill.as_ref().expect("picture declaration"),
                    ));
                }
            }
        }
        uses
    }
}
