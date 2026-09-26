use crate::*;
use mo_common::Digest;
use mo_geometry::{Fixed, Point};
use mo_presentation_compile::{
    source_page::{SourcePageProfile, SourcePageRequest},
    source_resource_page::{PROFILE as RESOURCE_PROFILE, SourceResourcePageImage},
};
use mo_raster::{PixelScale, RasterViewport};
use mo_text::manifest::FontManifest;
use sha2::{Digest as _, Sha256};

pub use mo_presentation_compile::source_resource_page::ResourcePageRequest as PreviewRequest;
pub struct PreviewFonts<'a> {
    pub manifest: Option<&'a FontManifest>,
    pub content: Content<'a>,
}
pub enum PreviewInput<'a> {
    /// An inspected source route; the worker revalidates the immutable bytes.
    Source(Content<'a>),
    /// Direct author semantics; the worker reconstructs the same typed plan.
    Author {
        plan: &'a mo_pptx::AuthorPlan<'a>,
        resources: &'a dyn mo_pptx::Resources,
    },
}
/// An injected computation capability. Native hosts isolate unsafe components;
/// WASM hosts supply their own bridge. Errors carry actual renderer diagnostics.
pub trait PreviewRenderer {
    fn identity(&self) -> RendererIdentity;
    /// A document-scoped batch: prepare immutable inputs once, then emit each
    /// page in order. The callback owns output storage and may cancel the batch.
    fn render_pages(
        &mut self,
        requests: &[PreviewRequest],
        input: PreviewInput<'_>,
        fonts: PreviewFonts<'_>,
        check: &dyn Fn() -> bool,
        emit: &mut dyn FnMut(usize, SourceResourcePageImage) -> Result<(), DeliveryError>,
    ) -> Result<(), DeliveryError>;
    fn render(
        &mut self,
        request: &PreviewRequest,
        source: Content<'_>,
        fonts: PreviewFonts<'_>,
        check: &dyn Fn() -> bool,
    ) -> Result<SourceResourcePageImage, DeliveryError> {
        let mut result = None;
        self.render_pages(
            std::slice::from_ref(request),
            PreviewInput::Source(source),
            fonts,
            check,
            &mut |ordinal, image| {
                if ordinal != 0 || result.is_some() {
                    return Err(DeliveryError::Invalid("preview page sequence"));
                }
                result = Some(image);
                Ok(())
            },
        )?;
        result.ok_or(DeliveryError::Invalid("missing preview page"))
    }
}
pub(crate) fn viewport(
    size: mo_presentation_model::Size,
    width: u32,
) -> Result<RasterViewport, DeliveryError> {
    let w = u64::try_from(size.width.get()).map_err(|_| DeliveryError::Invalid("page width"))?;
    let h = u64::try_from(size.height.get()).map_err(|_| DeliveryError::Invalid("page height"))?;
    if w == 0 || h == 0 || width == 0 || width > 8192 {
        return Err(DeliveryError::Invalid("page extent"));
    }
    let height = (u128::from(h) * u128::from(width)).div_ceil(u128::from(w));
    if height == 0
        || height > 8192
        || height * u128::from(width) * 4 > mo_raster::MAX_PIXEL_BYTES as u128
    {
        return Err(DeliveryError::Limit("preview pixels"));
    }
    let mut a = w;
    let mut b = u64::from(width);
    while b != 0 {
        (a, b) = (b, a % b);
    }
    let denominator =
        u32::try_from(w / a).map_err(|_| DeliveryError::Limit("preview scale denominator"))?;
    Ok(RasterViewport {
        width,
        height: height as u32,
        origin: Point {
            x: Fixed::ZERO,
            y: Fixed::ZERO,
        },
        scale: PixelScale {
            numerator: (u64::from(width) / a) as u32,
            denominator,
        },
        coordinate_tolerance: Fixed::from_raw(1 << 20),
        background: [0; 4],
    })
}
pub(crate) fn request(
    settings: &DeliverySettings,
    viewport: &RasterViewport,
    sha: &Digest,
    slide: &str,
) -> PreviewRequest {
    PreviewRequest {
        page: SourcePageRequest {
            expected_source_sha256: sha.clone(),
            slide: slide.into(),
            profile: SourcePageProfile::StaticSolidDraftV1,
            viewport: viewport.clone(),
            color_context: settings.color_context.clone(),
        },
        image_source: settings.image_source,
        sampling: settings.sampling,
    }
}
pub(crate) fn validate(
    request: &PreviewRequest,
    image: &SourceResourcePageImage,
    check: &dyn Fn() -> bool,
) -> Result<(), DeliveryError> {
    cancel(check)?;
    let info = &image.info;
    let page = &info.page.page;
    let raster = &info.page.scene.raster;
    let vp = &request.page.viewport;
    if info.profile != RESOURCE_PROFILE
        || info.page.scene.profile != mo_render::PROFILE
        || page.source_sha256 != request.page.expected_source_sha256
        || page.slide != request.page.slide
        || raster.width != vp.width
        || raster.height != vp.height
        || !mo_raster::accepts_profile(&raster.profile, true)
        || raster.byte_length.get() != image.pixels.len() as u64
        || u64::from(vp.width) * u64::from(vp.height) * 4 != image.pixels.len() as u64
    {
        return Err(DeliveryError::Invalid("preview binding or dimensions"));
    }
    let mut hash = Sha256::new();
    for chunk in image.pixels.chunks(16384) {
        cancel(check)?;
        hash.update(chunk);
    }
    if raster.sha256 != Digest::from_sha256(hash.finalize().into()) {
        return Err(DeliveryError::Invalid("preview pixel digest"));
    }
    Ok(())
}
