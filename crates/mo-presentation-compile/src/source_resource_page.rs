//! Resource-enabled native pages, using the existing SourcePage engine. The
//! caller supplies an inspected immutable package, decoder and optional native
//! font/shaper context. No browser layout, filesystem or font discovery enters.
pub mod protocol;
mod resources;
mod retained;
mod types;
use crate::{
    source_page::{self, *},
    source_text_page,
};
use mo_image::ImageDecoder;
use mo_opc::PackageRead;
use mo_presentation_source::source::SourceIndex;
use mo_raster::{PreparedImages, RasterBackend, RasterError};
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
    built: source_page::BuiltPage,
    text: Option<source_text_page::TextPageContent>,
    images: resources::Resources,
}
pub(crate) struct PageView<'a> {
    pub request: &'a SourcePageRequest,
    pub rotations: Option<&'a crate::source_placement::SourceRotations>,
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
            request: q,
            rotations: None,
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
    cancel(check)?;
    if package.sha256() != &index.source_sha256 {
        return Err(SourcePageError::SourceConflict);
    }
    let q = view.request;
    let prepared =
        source_page::preflight_sampled(index, q, text.is_some(), true, view.rotations, check)?;
    let mut text =
        text.map(|t| source_text_page::Compiler::new(t.manifest, t.backend, options.text_limits));
    if let Some(text) = &mut text {
        text.preflight(index, q, prepared.objects.iter().map(|o| &o.binding), check)?;
    }
    let images = resources::prepare(package, index, &prepared, options, decoder, check)?;
    let built = source_page::build(
        prepared,
        text.as_mut()
            .map(|t| t as &mut dyn source_text_page::Painter),
        &images.paints,
        check,
    )?;
    let text = text.map(source_text_page::Compiler::finish).transpose()?;
    cancel(check)?;
    Ok(PreparedResourcePage {
        built,
        text,
        images,
    })
}
impl PreparedResourcePage {
    pub fn plan(self, check: &dyn Fn() -> bool) -> Result<SourceResourcePagePlan, SourcePageError> {
        let images = PreparedImages::new(&self.images.manifest, &self.images.pixels, check)?;
        let compiled = mo_render::compile_images(&self.built.raster, &images, check)?;
        let page = self.built.finish(
            compiled.work().clone(),
            compiled.raster().work().coordinate_error_bound,
        )?;
        cancel(check)?;
        Ok(SourceResourcePagePlan {
            profile: PROFILE.into(),
            page,
            text: self.text,
            images: self.images.info,
            image_work: compiled.raster().work().clone(),
            resources_sha256: images.sha256().clone(),
        })
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
