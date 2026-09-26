//! Text-enabled source page compilation. The existing source-page engine owns
//! layer order, shape paint and raster publication; this adds native text at each
//! object's paint position, using source-bound preparation and explicit fonts.
mod decorations;
mod glyphs;
mod placement;
mod precision;
pub(crate) mod retained;
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
    backend: &'a mut dyn TextBackend,
    limits: TextPageLimits,
    pending: BTreeMap<(String, u32), Pending>,
    bindings: Vec<TextPageBinding>,
    sources: Vec<TextPagePaintSource>,
    decoration_sources: Vec<TextDecorationSource>,
    work: FrameWork,
}
impl<'a, 'm, 'font> Compiler<'a, 'm, 'font> {
    pub(crate) fn new(
        manifest: &'a PreparedManifest<'m, 'font>,
        backend: &'a mut dyn TextBackend,
        limits: TextPageLimits,
    ) -> Self {
        Self {
            manifest,
            backend,
            limits,
            pending: BTreeMap::new(),
            bindings: vec![],
            sources: vec![],
            decoration_sources: vec![],
            work: FrameWork::default(),
        }
    }
    pub(crate) fn finish(self) -> Result<TextPageContent, SourcePageError> {
        if !self.pending.is_empty() {
            return Err(SourcePageError::Invalid("unpainted prepared source text"));
        }
        Ok(TextPageContent {
            texts: self.bindings,
            text_sources: self.sources,
            decoration_sources: self.decoration_sources,
            text_work: self.work,
        })
    }

    pub(crate) fn preflight<'b>(
        &mut self,
        index: &SourceIndex,
        q: &SourcePageRequest,
        objects: impl Iterator<Item = &'b SourcePagePaintBinding>,
        check: &dyn Fn() -> bool,
    ) -> Result<(), SourcePageError> {
        let mut paragraphs = 0usize;
        let mut runs = 0usize;
        let mut plan_bytes = 0usize;
        let mut indexed =
            BTreeMap::<&str, BTreeMap<u32, &mo_presentation_source::source::SourceObject>>::new();
        for object in objects {
            cancel(check)?;
            let at = &object.location;
            let native_id = at.object.expect("object location");
            if let std::collections::btree_map::Entry::Vacant(entry) = indexed.entry(&at.part) {
                let mut objects = BTreeMap::new();
                for object in &index.surfaces[&at.part].objects {
                    cancel(check)?;
                    objects.insert(object.native_id, object);
                }
                entry.insert(objects);
            }
            let native = indexed[at.part.as_str()][&native_id];
            if native.text_body_ordinal.is_none() {
                continue;
            }
            paragraphs = paragraphs
                .checked_add(native.paragraphs.len())
                .ok_or(RasterError::Limit("page paragraphs"))?;
            runs = native
                .paragraphs
                .iter()
                .try_fold(runs, |n, p| n.checked_add(p.len()))
                .ok_or(RasterError::Limit("page text runs"))?;
            if self.pending.len() >= self.limits.max_frames
                || paragraphs > self.limits.max_paragraphs
                || runs > self.limits.max_runs
            {
                return Err(RasterError::Limit("page text preflight").into());
            }
            let frame = source_frame::prepare(
                index,
                &SourceFrameRequest {
                    expected_source_sha256: q.expected_source_sha256.clone(),
                    object: SourceObjectRef {
                        part: at.part.clone(),
                        native_id,
                    },
                    bounds_tolerance: Fixed::from_raw(1 << 24),
                },
                self.manifest,
                self.limits.work,
                check,
            )
            .map_err(|e| SourcePageError::from(e).at(at))?;
            plan_bytes = plan_bytes
                .checked_add(frame.accounted_plan_bytes())
                .filter(|n| *n <= self.limits.max_prepared_plan_bytes)
                .ok_or(RasterError::Limit("page prepared text plans"))?;
            let paints = paint::resolve(
                index,
                frame.source(),
                &q.color_context,
                mo_presentation_source::source::color::ColorLimits::default(),
                check,
            )
            .map_err(|e| SourcePageError::from(e).at(at))?;
            // Unknown working colors are a prerequisite failure, never black.
            for (paragraph, runs) in paints.iter().enumerate() {
                for (run, value) in runs.iter().enumerate() {
                    glyphs::colors(value).map_err(|e| {
                        SourcePageError::from(e.at_run(paint::TextPaintLocation::at(
                            paragraph as u32,
                            &frame.source().paragraphs[paragraph].runs[run],
                        )))
                        .at(at)
                    })?;
                }
            }
            self.pending
                .insert((at.part.clone(), native_id), Pending { frame, paints });
        }
        cancel(check)
    }
    pub(crate) fn append(
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
        let mut limits = self.limits.work;
        limits.max_component_calls -= self.work.component_calls;
        limits.max_font_upload_bytes -= self.work.font_upload_bytes;
        limits.max_request_words -= self.work.request_words;
        limits.max_glyphs -= self.work.glyphs;
        limits.max_path_commands -= self.work.path_commands;
        let frame =
            source_frame::compute(pending.frame, self.manifest, self.backend, limits, check)?;
        let (clusters, owners) = glyphs::bind(&frame, &pending.paints, check)?;
        let mut worker = source_frame::backend::FrameBackend {
            inner: self.backend,
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
        let mut position_error = Fixed::ZERO;
        let mut geometry_error = Fixed::ZERO;
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
                },
                check,
            )?;
            position_error = position_error.max(position);
            geometry_error = geometry_error.max(geometry);
            Ok(instance)
        };
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
        self.bindings.push(TextPageBinding {
            binding,
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
    let (profile, page, downstream_coordinate_error_bound, text_frames, text_work, compiled) = {
        let (plan, compiled) = prepare(index, q, manifest, text_backend, limits, check)?;
        (
            plan.profile,
            plan.page.info,
            plan.page.downstream_coordinate_error_bound,
            plan.texts.len() as u32,
            plan.text_work,
            compiled,
        )
    };
    let image = mo_render::render_compiled(compiled, raster_backend, check)?;
    Ok(SourceTextPageImage {
        info: SourceTextPageRasterInfo {
            profile,
            text_frames,
            text_work,
            page: SourcePageRasterInfo {
                page,
                scene: image.info,
                downstream_coordinate_error_bound,
            },
        },
        pixels: image.pixels,
    })
}
