use super::*;
use crate::{
    source_image_layout::ImageSourceLayoutPlan,
    source_image_paint::NativeImagePaint,
    source_text_page::{TextPageContent, TextPageLimits},
};
use mo_image::DecodedImageInfo;
use mo_pptx::source::images::{ImageSourceSelection, SourceImageBinding};
use mo_raster::ImageSampling;
use serde::Serialize;

/// Per-page input; immutable document and font resources belong to the session.
#[derive(Debug, Clone, Serialize, serde::Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ResourcePageRequest {
    pub page: SourcePageRequest,
    pub image_source: ImageSourceSelection,
    pub sampling: ImageSampling,
}

#[derive(Debug, Clone, Copy)]
pub struct ResourcePageOptions {
    pub selection: ImageSourceSelection,
    pub sampling: ImageSampling,
    pub text_limits: TextPageLimits,
}
pub struct TextPageContext<'a, 'm, 'font> {
    pub manifest: &'a mo_text::manifest::PreparedManifest<'m, 'font>,
    pub backend: &'a mut dyn mo_text::backend::TextBackend,
}
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PageImageBinding {
    pub binding: u32,
    pub source: SourceImageBinding,
    pub layout: ImageSourceLayoutPlan,
    pub paint: NativeImagePaint,
}
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PageImageResources {
    pub selection: ImageSourceSelection,
    pub sampling: ImageSampling,
    /// Distinct encoded source bytes admitted, after content deduplication.
    pub encoded_bytes: u64,
    pub decoded: Vec<DecodedImageInfo>,
    pub bindings: Vec<PageImageBinding>,
    pub gather_copy_bytes: u32,
}
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SourceResourcePagePlan {
    pub profile: String,
    /// Common shape-policy/coordinate/provenance plan. The enclosing resource
    /// profile adds native images and, with an explicit manifest, source text.
    pub page: SourcePagePlan,
    pub text: Option<TextPageContent>,
    pub images: PageImageResources,
    pub image_work: mo_raster::ImageWork,
    pub resources_sha256: mo_common::Digest,
}
#[derive(Debug, Clone, Serialize, serde::Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SourceResourcePageRasterInfo {
    pub profile: String,
    pub page: SourcePageRasterInfo,
    pub text_frames: u32,
    pub text_work: crate::source_frame::FrameWork,
    pub images: mo_raster::ImageWork,
    pub resources_sha256: mo_common::Digest,
    pub decoded_images: Vec<DecodedImageInfo>,
    pub encoded_bytes: u64,
    pub gather_copy_bytes: u32,
}
pub struct SourceResourcePageImage {
    pub info: SourceResourcePageRasterInfo,
    pub pixels: Vec<u8>,
}
