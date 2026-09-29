//! An owned sampled page; decoded pixels are shared immutably with its plan.
use super::*;

pub struct PreparedResourceFrame {
    pub(super) compiled: mo_render::CompiledImageScene<'static>,
    pub(super) page: SourcePageInfo,
    pub(super) downstream: mo_geometry::Fixed,
    pub(super) text_work: crate::source_frame::FrameWork,
    pub(super) text_capacity: crate::source_frame::capacity::TextCapacity,
    pub(super) decoded: Vec<mo_image::DecodedImageInfo>,
    pub(super) encoded_bytes: u64,
}
impl PreparedResourceFrame {
    pub fn raster(&self) -> &mo_raster::CompiledImageRaster<'_> {
        self.compiled.raster()
    }
    pub fn complete(
        self,
        reply: impl Into<mo_raster::RasterCompletionReply>,
        check: &dyn Fn() -> bool,
    ) -> Result<SourceResourcePageImage, SourcePageError> {
        let Self {
            compiled,
            page,
            downstream,
            text_work,
            text_capacity,
            decoded,
            encoded_bytes,
        } = self;
        let image = compiled.complete(reply, check)?;
        Ok(finish(
            page,
            downstream,
            text_work,
            text_capacity,
            decoded,
            encoded_bytes,
            image,
        ))
    }
    pub fn render(
        self,
        backend: &mut dyn RasterBackend,
        check: &dyn Fn() -> bool,
    ) -> Result<SourceResourcePageImage, SourcePageError> {
        let Self {
            compiled,
            page,
            downstream,
            text_work,
            text_capacity,
            decoded,
            encoded_bytes,
        } = self;
        let image = mo_render::render_compiled_images(compiled, backend, check)?;
        Ok(finish(
            page,
            downstream,
            text_work,
            text_capacity,
            decoded,
            encoded_bytes,
            image,
        ))
    }
}
fn finish(
    page: SourcePageInfo,
    downstream: mo_geometry::Fixed,
    text_work: crate::source_frame::FrameWork,
    text_capacity: crate::source_frame::capacity::TextCapacity,
    decoded: Vec<mo_image::DecodedImageInfo>,
    encoded_bytes: u64,
    image: mo_render::ImageSceneRaster,
) -> SourceResourcePageImage {
    SourceResourcePageImage {
        info: SourceResourcePageRasterInfo {
            profile: PROFILE.into(),
            page: SourcePageRasterInfo {
                page,
                scene: image.info.scene,
                downstream_coordinate_error_bound: downstream,
            },
            text_frames: text_capacity.frames.len() as u32,
            text_work,
            text_capacity: Some(text_capacity),
            decoded_images: decoded,
            encoded_bytes,
            gather_copy_bytes: 0,
            images: image.info.images,
            resources_sha256: image.info.resources_sha256,
        },
        pixels: image.pixels,
    }
}
