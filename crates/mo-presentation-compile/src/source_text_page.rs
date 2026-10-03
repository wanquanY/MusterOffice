//! Text-enabled source page compilation. The existing source-page engine owns
//! layer order, shape paint and raster publication; this adds native text at each
//! object's paint position, using source-bound preparation and explicit fonts.
mod clip;
mod decorations;
mod glyphs;
mod observations;
pub(crate) mod placement;
mod precision;
mod prepare;
pub(crate) mod retained;
#[cfg(all(test, any(target_os = "macos", target_os = "linux")))]
mod table_tests;
pub(crate) use placement::Painter;
mod types;
use crate::{
    path_scene::SceneBuilder,
    source_frame::{self, *},
    source_page::*,
};
use mo_geometry::{Fixed, Point};
use mo_presentation_source::source::{
    SourceIndex, SourceObjectRef,
    text::paint::{self, TextPaint, TextRunPaint},
};
use mo_raster::{RasterBackend, RasterError, RasterViewport};
use mo_text::{backend::TextBackend, manifest::PreparedManifest};
use std::collections::BTreeMap;
pub use types::*;
pub const PROFILE: &str = "drawingml-solid-text-page-q32-draft-v1";
fn cancel(check: &dyn Fn() -> bool) -> Result<(), SourcePageError> {
    if check() {
        Err(RasterError::Cancelled.into())
    } else {
        Ok(())
    }
}
struct Pending {
    frame: source_frame::PreparedFrame,
    paints: Vec<Vec<TextRunPaint>>,
}
pub(crate) struct Compiler<'a, 'm, 'font> {
    manifest: &'a PreparedManifest<'m, 'font>,
    backend: mo_text::backend::FontSession<'a, 'font>,
    limits: TextPageLimits,
    pending: BTreeMap<(String, u32), Vec<Pending>>,
    preparation: prepare::PreparationBudget,
    // A failed multi-frame transaction cannot publish its already-computed prefix.
    failed: bool,
    bindings: Vec<TextPageBinding>,
    sources: Vec<TextPagePaintSource>,
    decoration_sources: Vec<TextDecorationSource>,
    work: FrameWork,
    interaction_limits: Option<interaction::FrameInteractionLimits>,
    interaction_maps: Vec<Vec<mo_text::interaction::InteractionMap>>,
    interaction_clips: Vec<Option<u32>>,
    interaction_cells: usize,
    interaction_lines: usize,
}
impl<'a, 'm, 'font> Compiler<'a, 'm, 'font> {
    pub(crate) fn new(
        manifest: &'a PreparedManifest<'m, 'font>,
        backend: &'a mut dyn TextBackend,
        limits: TextPageLimits,
    ) -> Self {
        Self {
            manifest,
            backend: manifest.font_session(backend),
            limits,
            pending: BTreeMap::new(),
            preparation: prepare::PreparationBudget::new(limits),
            failed: false,
            bindings: vec![],
            sources: vec![],
            decoration_sources: vec![],
            work: FrameWork::default(),
            interaction_limits: None,
            interaction_maps: vec![],
            interaction_clips: vec![],
            interaction_cells: 0,
            interaction_lines: 0,
        }
    }
    pub(crate) fn shape_charts(
        &mut self,
        charts: &mut crate::source_chart_page::Charts,
        check: &dyn Fn() -> bool,
    ) -> Result<(), SourcePageError> {
        self.ensure_ready()?;
        self.failed = true;
        let mut backend = source_frame::backend::FrameBackend {
            inner: &mut self.backend,
            work: self.work.clone(),
            limits: self.limits.work,
        };
        let mut path_bytes = charts.values().map(|c| c.info.path_bytes).sum();
        for chart in charts.values_mut() {
            crate::source_chart_page::text::shape(
                chart,
                self.manifest,
                &mut backend,
                &mut path_bytes,
                check,
            )?;
        }
        self.work = backend.work;
        self.failed = false;
        Ok(())
    }
    pub(crate) fn retain_interaction(
        mut self,
        limits: interaction::FrameInteractionLimits,
    ) -> Self {
        self.interaction_limits = Some(limits);
        self
    }
    pub(crate) fn finish(self) -> Result<TextPageContent, SourcePageError> {
        self.finish_with_interaction().map(|(text, _)| text)
    }
    pub(crate) fn finish_with_interaction(
        self,
    ) -> Result<(TextPageContent, Option<TextPageInteraction>), SourcePageError> {
        self.ensure_ready()?;
        if !self.pending.is_empty() {
            return Err(SourcePageError::Invalid("unpainted prepared source text"));
        }
        if self.interaction_limits.is_some() && self.interaction_maps.len() != self.bindings.len() {
            return Err(SourcePageError::Invalid("incomplete page text interaction"));
        }
        Ok((
            TextPageContent {
                texts: self.bindings,
                text_sources: self.sources,
                decoration_sources: self.decoration_sources,
                text_work: self.work,
            },
            self.interaction_limits.map(|limits| TextPageInteraction {
                maps: self.interaction_maps,
                clips: self.interaction_clips,
                limits,
            }),
        ))
    }

    fn ensure_ready(&self) -> Result<(), SourcePageError> {
        if self.failed {
            Err(SourcePageError::Invalid("failed source text computation"))
        } else {
            Ok(())
        }
    }

    pub(crate) fn append(
        &mut self,
        binding: u32,
        object: &SourcePagePaintBinding,
        builder: &mut SceneBuilder<SourcePagePaintSource>,
        viewport: &RasterViewport,
        check: &dyn Fn() -> bool,
    ) -> Result<(Fixed, Fixed), SourcePageError> {
        self.ensure_ready()?;
        self.failed = true;
        let result = self.append_object(binding, object, builder, viewport, check);
        self.failed = result.is_err();
        result
    }
    fn append_object(
        &mut self,
        binding: u32,
        object: &SourcePagePaintBinding,
        builder: &mut SceneBuilder<SourcePagePaintSource>,
        viewport: &RasterViewport,
        check: &dyn Fn() -> bool,
    ) -> Result<(Fixed, Fixed), SourcePageError> {
        let key = (
            object.location.part.clone(),
            object.location.object.expect("object"),
        );
        let Some(pending) = self.pending.remove(&key) else {
            return Ok((Fixed::ZERO, Fixed::ZERO));
        };
        let mut position = Fixed::ZERO;
        let mut geometry = Fixed::ZERO;
        for frame in pending {
            let (p, g) = self.append_frame(frame, binding, object, builder, viewport, check)?;
            position = position.max(p);
            geometry = geometry.max(g);
        }
        Ok((position, geometry))
    }
    fn append_frame(
        &mut self,
        pending: Pending,
        binding: u32,
        object: &SourcePagePaintBinding,
        builder: &mut SceneBuilder<SourcePagePaintSource>,
        viewport: &RasterViewport,
        check: &dyn Fn() -> bool,
    ) -> Result<(Fixed, Fixed), SourcePageError> {
        cancel(check)?;
        let mut limits = self.limits.work;
        limits.max_component_calls -= self.work.component_calls;
        limits.max_font_upload_bytes -= self.work.font_upload_bytes;
        limits.max_request_words -= self.work.request_words;
        limits.max_glyphs -= self.work.glyphs;
        limits.max_path_commands -= self.work.path_commands;
        let interaction_limits = self.interaction_limits.map(|mut limits| {
            limits.max_cells -= self.interaction_cells;
            limits.max_lines -= self.interaction_lines;
            limits
        });
        let editor = source_frame::compute_with_interaction(
            pending.frame,
            self.manifest,
            &mut self.backend,
            limits,
            interaction_limits,
            check,
        )?;
        let (frame, maps) = editor.into_parts();
        self.interaction_cells += maps.iter().map(|m| m.cells.len()).sum::<usize>();
        self.interaction_lines += maps.iter().map(|m| m.lines.len()).sum::<usize>();
        let (clusters, owners) = glyphs::bind(&frame, &pending.paints, check)?;
        let mut worker = source_frame::backend::FrameBackend {
            inner: &mut self.backend,
            work: frame.work.clone(),
            limits,
        };
        let decorations = decorations::build(
            &frame,
            &clusters,
            &owners,
            self.manifest,
            &mut worker,
            check,
        )?;
        // Keep the unpainted frame's work/provenance intact. The page owns
        // additional paint computation and accounts for the combined work.
        let mut work = worker.work;
        work.path_commands = work
            .path_commands
            .checked_add(
                (decorations.len() as u32)
                    .checked_mul(5)
                    .ok_or(RasterError::Limit("decoration paths"))?,
            )
            .filter(|n| *n <= limits.max_path_commands)
            .ok_or(RasterError::Limit("frame decoration paths"))?;
        self.work.component_calls += work.component_calls;
        self.work.font_upload_bytes += work.font_upload_bytes;
        self.work.request_words += work.request_words;
        self.work.glyphs += work.glyphs;
        self.work.path_commands += work.path_commands;
        let uncertainty = precision::frame_bound(&frame)?;
        let clip = clip::LocalClip::for_frame(&frame, &decorations, uncertainty)?;
        let (clip, mut position_error, mut geometry_error) =
            clip::append(clip, object, builder, viewport, check)?;
        let text_binding = self.bindings.len() as u32;
        let mut paint_path = |commands: &[mo_geometry::PathCommand],
                              origin: Point,
                              rgba: [u8; 4],
                              uncertainty: Fixed|
         -> Result<u32, SourcePageError> {
            let (instance, position, geometry) = placement::paint(
                binding,
                object,
                builder,
                viewport,
                placement::LocalPath {
                    commands,
                    origin,
                    rgba,
                    uncertainty,
                    clip,
                },
                check,
            )?;
            position_error = position_error.max(position);
            geometry_error = geometry_error.max(geometry);
            Ok(instance)
        };
        let mut painted_ink = None;
        for (i, (g, cluster)) in frame.glyphs.iter().zip(owners).enumerate() {
            cancel(check)?;
            let Some(rgba) = clusters[cluster as usize].rgba else {
                continue;
            };
            let scene = frame.paragraphs[g.paragraph as usize]
                .computed
                .geometry
                .paths
                .scene
                .as_ref()
                .expect("frame scene");
            let path = &scene.paths[scene.glyphs[g.glyph as usize].path as usize];
            if path.commands.is_empty() {
                continue;
            }
            observations::add_ink(&mut painted_ink, path.bounds, g.origin, rgba)?;
            let instance = paint_path(&path.commands, g.origin, rgba, uncertainty)?;
            self.sources.push(TextPagePaintSource {
                instance,
                text_binding,
                glyph: i as u32,
                cluster,
            });
        }
        for (i, decoration) in decorations.iter().enumerate() {
            cancel(check)?;
            let instance = paint_path(
                &decorations::commands(decoration.rect),
                Point {
                    x: Fixed::ZERO,
                    y: Fixed::ZERO,
                },
                decoration.rgba,
                uncertainty.checked_add(Fixed::from_raw(8))?,
            )?;
            self.decoration_sources.push(TextDecorationSource {
                instance,
                text_binding,
                decoration: i as u32,
            });
        }
        let local_coordinate_error_bound = if decorations.is_empty() {
            uncertainty
        } else {
            uncertainty.checked_add(Fixed::from_raw(8))?
        };
        if self.interaction_limits.is_some() {
            self.interaction_maps.push(maps);
            self.interaction_clips.push(builder.effective_clip(clip));
        }
        self.bindings.push(TextPageBinding {
            binding,
            page_ink: observations::place(
                painted_ink,
                uncertainty,
                frame.text.cell,
                frame.clip.is_some(),
                object,
                viewport,
                check,
            )?,
            painted_ink,
            frame,
            paints: pending.paints,
            clusters,
            decorations,
            local_coordinate_error_bound,
        });
        Ok((position_error, geometry_error))
    }
}
fn prepare(
    index: &SourceIndex,
    q: &SourcePageRequest,
    manifest: &PreparedManifest<'_, '_>,
    backend: &mut dyn TextBackend,
    limits: TextPageLimits,
    check: &dyn Fn() -> bool,
) -> Result<(SourceTextPagePlan, mo_render::CompiledScene), SourcePageError> {
    let mut text = Compiler::new(manifest, backend, limits);
    let (page, compiled) = crate::source_page::prepare(index, q, Some(&mut text), check)?;
    let text = text.finish()?;
    Ok((
        SourceTextPagePlan {
            profile: PROFILE.into(),
            page,
            texts: text.texts,
            text_sources: text.text_sources,
            decoration_sources: text.decoration_sources,
            text_work: text.text_work,
        },
        compiled,
    ))
}
pub fn compile(
    index: &SourceIndex,
    q: &SourcePageRequest,
    manifest: &PreparedManifest<'_, '_>,
    backend: &mut dyn TextBackend,
    limits: TextPageLimits,
    check: &dyn Fn() -> bool,
) -> Result<SourceTextPagePlan, SourcePageError> {
    prepare(index, q, manifest, backend, limits, check).map(|(plan, _)| plan)
}
pub fn render(
    index: &SourceIndex,
    q: &SourcePageRequest,
    manifest: &PreparedManifest<'_, '_>,
    text_backend: &mut dyn TextBackend,
    raster_backend: &mut dyn RasterBackend,
    limits: TextPageLimits,
    check: &dyn Fn() -> bool,
) -> Result<SourceTextPageImage, SourcePageError> {
    // Detailed provenance and shaping plans are a compile product, not a
    // mandatory raster payload. Release them before entering the raster host.
    let (
        profile,
        page,
        downstream_coordinate_error_bound,
        text_frames,
        text_work,
        text_capacity,
        compiled,
    ) = {
        let (plan, compiled) = prepare(index, q, manifest, text_backend, limits, check)?;
        let text_capacity = capacity(&plan.texts, check)?;
        (
            plan.profile,
            plan.page.info,
            plan.page.downstream_coordinate_error_bound,
            plan.texts.len() as u32,
            plan.text_work,
            text_capacity,
            compiled,
        )
    };
    let image = mo_render::render_compiled(compiled, raster_backend, check)?;
    Ok(SourceTextPageImage {
        info: SourceTextPageRasterInfo {
            profile,
            text_frames,
            text_work,
            text_capacity: Some(text_capacity),
            page: SourcePageRasterInfo {
                page,
                scene: image.info,
                downstream_coordinate_error_bound,
            },
        },
        pixels: image.pixels,
    })
}
