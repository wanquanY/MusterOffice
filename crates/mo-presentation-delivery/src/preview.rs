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

pub struct PreviewRequest {
    pub page: SourcePageRequest,
    pub fonts: Option<FontManifest>,
    pub image_source: mo_pptx::source::images::ImageSourceSelection,
    pub sampling: mo_raster::ImageSampling,
}
/// An injected computation capability. Native hosts isolate unsafe components;
/// WASM hosts supply their own bridge. Errors carry actual renderer diagnostics.
pub trait PreviewRenderer {
    fn identity(&self) -> RendererIdentity;
    fn render(
        &mut self,
        request: &PreviewRequest,
        source: Content<'_>,
        fonts: Content<'_>,
        check: &dyn Fn() -> bool,
    ) -> Result<SourceResourcePageImage, DeliveryError>;
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
        fonts: settings.fonts.clone(),
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
