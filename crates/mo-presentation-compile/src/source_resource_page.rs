//! Resource-enabled native pages, using the existing SourcePage engine. The
//! caller supplies an inspected immutable package, decoder and optional native
//! font/shaper context. No browser layout, filesystem or font discovery enters.
pub mod protocol;
mod resources;
mod retained;
mod retained_frame;
mod sampling;
pub use retained_frame::PreparedResourceFrame;
pub(crate) use sampling::DecodePolicy;
mod types;
use crate::{
    source_page::{self, *},
    source_text_page,
};
use mo_image::ImageDecoder;
use mo_opc::PackageRead;
use mo_presentation_source::source::{
    SourceIndex, images::PackageImages, page_resources::PageInput,
};
use mo_raster::{PreparedImages, RasterBackend, RasterError};
pub(crate) use retained::ResourceViewportUpdate;
pub use retained::{ResourcePagePlan, ResourcePreparationInfo};
pub use types::*;
pub const PROFILE: &str = "drawingml-resource-page-q32-v1-draft";
fn cancel(check: &dyn Fn() -> bool) -> Result<(), SourcePageError> {
    if check() {
        Err(RasterError::Cancelled.into())
    } else {
        Ok(())
    }
}
/// Owns the pending page and its decoded resources. It cannot be constructed
/// from host-provided substitute metadata. Final lowering and result validation
/// happen exactly once in either plan() or render(); no partial page escapes.
pub struct PreparedResourcePage {
    tables: Option<crate::source_table::RetainedTables>,
    charts: std::sync::Arc<crate::source_chart_page::Charts>,
    built: source_page::BuiltPage,
    text: Option<source_text_page::TextPageContent>,
    images: resources::Resources,
    interaction: Option<source_text_page::TextPageInteraction>,
}
pub(crate) struct PageView<'a> {
    pub source_owner: Option<&'a std::sync::Arc<SourceIndex>>,
    pub request: &'a SourcePageRequest,
    pub transforms: Option<&'a crate::source_placement::SourceProperties>,
    pub decode_policy: DecodePolicy<'a>,
    pub interaction_limits: Option<crate::source_frame::interaction::FrameInteractionLimits>,
}
pub fn prepare(
    package: &dyn PackageRead,
    index: &SourceIndex,
    q: &SourcePageRequest,
    decoder: &mut dyn ImageDecoder,
    text: Option<TextPageContext<'_, '_, '_>>,
    options: ResourcePageOptions,
    check: &dyn Fn() -> bool,
) -> Result<PreparedResourcePage, SourcePageError> {
    prepare_view(
        package,
        index,
        PageView {
            source_owner: None,
            request: q,
            transforms: None,
            decode_policy: DecodePolicy::Viewport,
            interaction_limits: None,
        },
        decoder,
        text,
        options,
        check,
    )
}
pub(crate) fn prepare_view(
    package: &dyn PackageRead,
    index: &SourceIndex,
    view: PageView<'_>,
    decoder: &mut dyn ImageDecoder,
    text: Option<TextPageContext<'_, '_, '_>>,
    options: ResourcePageOptions,
    check: &dyn Fn() -> bool,
) -> Result<PreparedResourcePage, SourcePageError> {
    prepare_input_view(
        &PackageImages(package),
        index,
        view,
        decoder,
        text,
        options,
        check,
    )
}

/// Compile an author plan or inspected source through the same resource, text,
/// geometry, placement and paint engines. ImageInput keeps provenance explicit.
pub fn prepare_input(
    input: &dyn PageInput,
    index: &SourceIndex,
    q: &SourcePageRequest,
    decoder: &mut dyn ImageDecoder,
    text: Option<TextPageContext<'_, '_, '_>>,
    options: ResourcePageOptions,
    check: &dyn Fn() -> bool,
) -> Result<PreparedResourcePage, SourcePageError> {
    prepare_input_view(
        input,
        index,
        PageView {
            source_owner: None,
            request: q,
            transforms: None,
            decode_policy: DecodePolicy::Viewport,
            interaction_limits: None,
        },
        decoder,
        text,
        options,
        check,
    )
}

pub(crate) fn prepare_input_view(
    input: &dyn PageInput,
    index: &SourceIndex,
    view: PageView<'_>,
    decoder: &mut dyn ImageDecoder,
    mut text: Option<TextPageContext<'_, '_, '_>>,
    options: ResourcePageOptions,
    check: &dyn Fn() -> bool,
) -> Result<PreparedResourcePage, SourcePageError> {
    cancel(check)?;
    if input.identity() != &index.source_sha256 {
        return Err(SourcePageError::SourceConflict);
    }
    let q = view.request;
    let charts = if let Some(package) = input.native_package() {
        crate::source_chart_page::prepare(package, index, q, view.transforms, &mut text, check)?
    } else {
        Default::default()
    };
    let mut prepared = source_page::preflight_resources(
        index,
        q,
        text.is_some(),
        true,
        view.transforms,
        None,
        charts,
        check,
    )?;
    let mut text = text.map(|t| {
        let compiler = source_text_page::Compiler::new(t.manifest, t.backend, options.text_limits);
        if let Some(limits) = view.interaction_limits {
            compiler.retain_interaction(limits)
        } else {
            compiler
        }
    });
    if let Some(text) = &mut text {
        text.preflight_shared(
            index,
            q,
            prepared.objects.iter().map(|o| &o.binding),
            &prepared.tables,
            check,
        )?;
    }
    if let Some(text) = &mut text {
        text.shape_charts(
            std::sync::Arc::get_mut(&mut prepared.charts).ok_or(SourcePageError::Invalid(
                "chart preparation unexpectedly shared",
            ))?,
            check,
        )?;
        prepared.info.charts = prepared.charts.values().map(|c| c.info.clone()).collect();
    }
    let charts = prepared.charts.clone();
    let tables = match (view.source_owner, prepared.source.take()) {
        (Some(owner), Some(source)) => Some(crate::source_table::RetainedTables::capture(
            std::sync::Arc::clone(owner),
            source,
            &prepared.tables,
            check,
        )?),
        _ => None,
    };
    prepared.tables.clear();
    let images = resources::prepare(
        input,
        index,
        &prepared,
        options,
        view.decode_policy,
        decoder,
        check,
    )?;
    let built = source_page::build(
        prepared,
        text.as_mut()
            .map(|t| t as &mut dyn source_text_page::Painter),
        &images.paints,
        check,
    )?;
    let (text, interaction) = match text {
        Some(text) => {
            let (content, interaction) = text.finish_with_interaction()?;
            (Some(content), interaction)
        }
        None => (None, None),
    };
    cancel(check)?;
    Ok(PreparedResourcePage {
        tables,
        charts,
        built,
        text,
        images,
        interaction,
    })
}
impl PreparedResourcePage {
    pub fn plan(self, check: &dyn Fn() -> bool) -> Result<SourceResourcePagePlan, SourcePageError> {
        self.plan_with_picking(None, check).map(|(plan, _)| plan)
    }
    fn plan_with_picking(
        self,
        interaction: Option<&mut source_text_page::TextPageInteraction>,
        check: &dyn Fn() -> bool,
    ) -> Result<
        (
            SourceResourcePagePlan,
            Option<mo_raster::picking::CompiledPicking>,
        ),
        SourcePageError,
    > {
        let images = PreparedImages::new(&self.images.manifest, &self.images.pixels, check)?;
        let retain = interaction.is_some();
        let compiled = compile_editor_clips(&self.built.raster, &images, interaction, check)?;
        let picking = retain
            .then(|| compiled.raster().picking(check))
            .transpose()?;
        let page = self.built.finish(
            compiled.work().clone(),
            compiled.raster().work().coordinate_error_bound,
        )?;
        cancel(check)?;
        Ok((
            SourceResourcePagePlan {
                profile: PROFILE.into(),
                page,
                text: self.text,
                images: self.images.info,
                image_work: compiled.raster().work().clone(),
                resources_sha256: images.sha256().clone(),
            },
            picking,
        ))
    }
    pub fn render(
        self,
        backend: &mut dyn RasterBackend,
        check: &dyn Fn() -> bool,
    ) -> Result<SourceResourcePageImage, SourcePageError> {
        let images = PreparedImages::new(&self.images.manifest, &self.images.pixels, check)?;
        let compiled = mo_render::compile_images(&self.built.raster, &images, check)?;
        let (info, downstream) = {
            let page = self.built.finish(
                compiled.work().clone(),
                compiled.raster().work().coordinate_error_bound,
            )?;
            (page.info, page.downstream_coordinate_error_bound)
        };
        let text_capacity = source_text_page::capacity(
            self.text.as_ref().map_or(&[], |text| text.texts.as_slice()),
            check,
        )?;
        let (text_frames, text_work) = self
            .text
            .map(|t| (t.texts.len() as u32, t.text_work))
            .unwrap_or_default();
        let resource_info = self.images.info;
        // Release calculation-only bindings before rasterization, while keeping
        // the immutable pixel bundle borrowed by the compiled image batch.
        drop(self.images.paints);
        drop(resource_info.bindings);
        let image = mo_render::render_compiled_images(compiled, backend, check)?;
        Ok(SourceResourcePageImage {
            info: SourceResourcePageRasterInfo {
                profile: PROFILE.into(),
                page: SourcePageRasterInfo {
                    page: info,
                    scene: image.info.scene,
                    downstream_coordinate_error_bound: downstream,
                },
                text_frames,
                text_work,
                text_capacity: Some(text_capacity),
                images: image.info.images,
                resources_sha256: image.info.resources_sha256,
                decoded_images: resource_info.decoded,
                encoded_bytes: resource_info.encoded_bytes,
                gather_copy_bytes: resource_info.gather_copy_bytes,
            },
            pixels: image.pixels,
        })
    }
}

impl PreparedResourcePage {
    pub(crate) fn editor_plan(
        mut self,
        check: &dyn Fn() -> bool,
    ) -> Result<crate::source_editor_page::SourceEditorPage, SourcePageError> {
        let mut interaction = self.interaction.take().ok_or(SourcePageError::Invalid(
            "page was not prepared for interaction",
        ))?;
        let (plan, picking) = self.plan_with_picking(Some(&mut interaction), check)?;
        crate::source_editor_page::SourceEditorPage::new(
            plan,
            interaction,
            picking.expect("retained picking"),
            check,
        )
    }
    pub(crate) fn editor_render(
        self,
        backend: &mut dyn RasterBackend,
        check: &dyn Fn() -> bool,
    ) -> Result<crate::source_editor_page::EditorPageImage, SourcePageError> {
        let mut interaction = self.interaction.ok_or(SourcePageError::Invalid(
            "page was not prepared for interaction",
        ))?;
        let images = PreparedImages::new(&self.images.manifest, &self.images.pixels, check)?;
        let compiled =
            compile_editor_clips(&self.built.raster, &images, Some(&mut interaction), check)?;
        let picking = compiled.raster().picking(check)?;
        let page = self.built.finish(
            compiled.work().clone(),
            compiled.raster().work().coordinate_error_bound,
        )?;
        let plan = SourceResourcePagePlan {
            profile: PROFILE.into(),
            page,
            text: self.text,
            images: self.images.info,
            image_work: compiled.raster().work().clone(),
            resources_sha256: images.sha256().clone(),
        };
        let page =
            crate::source_editor_page::SourceEditorPage::new(plan, interaction, picking, check)?;
        let image = mo_render::render_compiled_images(compiled, backend, check)?;
        cancel(check)?;
        Ok(crate::source_editor_page::EditorPageImage {
            page,
            pixels: image.pixels,
        })
    }
}
fn compile_editor_clips<'a>(
    raster: &mo_render::SceneRasterRequest,
    images: &'a PreparedImages<'a>,
    interaction: Option<&mut source_text_page::TextPageInteraction>,
    check: &dyn Fn() -> bool,
) -> Result<mo_render::CompiledImageScene<'a>, SourcePageError> {
    let Some(interaction) = interaction else {
        return Ok(mo_render::compile_images(raster, images, check)?);
    };
    let clips: std::collections::BTreeSet<_> =
        interaction.clips.iter().flatten().copied().collect();
    let clips: Vec<_> = clips.into_iter().collect();
    let compiled = mo_render::compile_images_retaining_clips(raster, images, &clips, check)?;
    for clip in interaction.clips.iter_mut().flatten() {
        cancel(check)?;
        *clip = compiled
            .lowered_clip(*clip)
            .ok_or(SourcePageError::Invalid("editor clip not retained"))?;
    }
    Ok(compiled)
}
