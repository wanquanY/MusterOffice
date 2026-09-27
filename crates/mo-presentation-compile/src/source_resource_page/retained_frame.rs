//! An owned sampled page; decoded pixels are shared immutably with its plan.
use super::*;

pub struct PreparedResourceFrame {
    pub(super) compiled: mo_render::CompiledImageScene<'static>,
    pub(super) page: SourcePageInfo,
    pub(super) downstream: mo_geometry::Fixed,
    pub(super) text_frames: u32,
    pub(super) text_work: crate::source_frame::FrameWork,
    pub(super) decoded: Vec<mo_image::DecodedImageInfo>,
    pub(super) encoded_bytes: u64,
}
impl PreparedResourceFrame {
    pub fn raster(&self) -> &mo_raster::CompiledImageRaster<'_> {
        self.compiled.raster()
    }
    pub fn complete(
        self,
        reply: mo_raster::BackendReply,
        check: &dyn Fn() -> bool,
    ) -> Result<SourceResourcePageImage, SourcePageError> {
        let Self {
            compiled,
            page,
            downstream,
            text_frames,
            text_work,
            decoded,
            encoded_bytes,
        } = self;
        let image = compiled.complete(reply, check)?;
        Ok(finish(
            page,
            downstream,
            text_frames,
            text_work,
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
            text_frames,
            text_work,
            decoded,
            encoded_bytes,
        } = self;
        let image = mo_render::render_compiled_images(compiled, backend, check)?;
        Ok(finish(
            page,
            downstream,
            text_frames,
            text_work,
            decoded,
            encoded_bytes,
            image,
        ))
    }
}
fn finish(
    page: SourcePageInfo,
    downstream: mo_geometry::Fixed,
    text_frames: u32,
    text_work: crate::source_frame::FrameWork,
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
            text_frames,
            text_work,
            decoded_images: decoded,
            encoded_bytes,
            gather_copy_bytes: 0,
            images: image.info.images,
            resources_sha256: image.info.resources_sha256,
        },
        pixels: image.pixels,
    }
}
