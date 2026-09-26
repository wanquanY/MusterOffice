//! Normalized pixels are explicit resources, independent of path placement.
//! Decoding, ICC conversion and file authority are outside this interface.
mod compile;
mod precision;
mod resources;
use crate::{CompiledRaster, PathRasterRequest, RasterBackend, RasterError, RasterImage, cancel};
pub(crate) use compile::ImageBrushes;
use mo_common::Digest;
use mo_geometry::{Fixed, Point};
pub use resources::PreparedImages;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

pub const MAX_IMAGE_FRAME_WORDS: usize = crate::MAX_FRAME_WORDS + 2 + 4096 * 18;
pub const IMAGE_PROFILE: &str = "skia-8d6d37b-q32-image-brushes-srgb-premul-rgba8-v5-draft";
pub const IMAGE_DOMAIN_PROFILE: &str = "skia-8d6d37b-q32-image-domains-srgb-premul-rgba8-v6-draft";
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum ImageAlpha {
    Straight,
    Premultiplied,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum ImageSampling {
    Nearest,
    Linear,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum ImageTile {
    Clamp,
    Repeat,
    Mirror,
    Decal,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ImageBrush {
    pub resource: u32,
    /// World Q32 EMU position of source pixel boundary (0, 0).
    pub origin: Point,
    /// World Q32 EMU displacement per source pixel, not endpoint coordinates.
    pub x_step: Point,
    pub y_step: Point,
    pub tile_x: ImageTile,
    pub tile_y: ImageTile,
    pub sampling: ImageSampling,
    /// Optional continuous source pixel domain; may extend beyond the resource
    /// into transparent space. Coordinates are Q32 pixels, not world EMU.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_domain: Option<ImageSourceDomain>,
    /// Outward errors from upstream layout/placement. These participate in the
    /// same device budget as float conversion, including repeated tile phase.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub uncertainty: Option<Box<ImageBrushUncertainty>>,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ImageBrushUncertainty {
    /// Nonnegative Q32 world EMU errors; rebase never changes these bounds.
    pub origin: Point,
    /// Nonnegative Q32 world EMU per source pixel.
    pub x_step: Point,
    pub y_step: Point,
    /// Nonnegative Q32 source pixel errors, left/top/right/bottom. Must be zero
    /// when the brush has no explicit source domain.
    pub source_domain: [Fixed; 4],
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ImageSourceDomain {
    pub left: Fixed,
    pub top: Fixed,
    pub right: Fixed,
    pub bottom: Fixed,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ImageResource {
    pub width: u32,
    pub height: u32,
    pub alpha: ImageAlpha,
    /// Hash of tightly packed, row-major RGBA8 bytes for this resource.
    pub sha256: Digest,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ImageWork {
    pub resources: u32,
    pub resource_bytes: u32,
    pub brushes: u32,
    pub draws: u32,
    /// Bound for upstream uncertainty and affine/domain quantization, in Q32
    /// device pixels. Domain repetition is bounded over the viewport. Excludes
    /// inverse/shader arithmetic, filter output changes and sample coverage.
    pub coordinate_error_bound: Fixed,
}
pub struct CompiledImageRaster<'a> {
    raster: CompiledRaster,
    images: &'a PreparedImages<'a>,
    work: ImageWork,
}
impl CompiledImageRaster<'_> {
    pub fn frame(&self) -> &[u32] {
        self.raster.frame()
    }
    pub fn work(&self) -> &ImageWork {
        &self.work
    }
    /// Geometry and other shared paint metrics; does not expose a resource-free
    /// executable batch that could lose the bound image bundle.
    pub fn raster_work(&self) -> &crate::RasterWork {
        self.raster.work()
    }
}
pub struct ImageRasterImage {
    pub raster: RasterImage,
    pub work: ImageWork,
    /// Hash of the complete resource bundle; together with frame/profile this
    /// identifies pixel inputs. A frame hash alone is insufficient for images.
    pub resources_sha256: Digest,
}
pub fn compile_images<'a>(
    request: &PathRasterRequest,
    images: &'a PreparedImages<'a>,
    check: &dyn Fn() -> bool,
) -> Result<CompiledImageRaster<'a>, RasterError> {
    cancel(check)?;
    let (raster, work) = crate::compile::compile_inner(request, Some(images), check)?;
    Ok(CompiledImageRaster {
        raster,
        images,
        work,
    })
}
pub fn render_images(
    request: &PathRasterRequest,
    images: &PreparedImages<'_>,
    backend: &mut dyn RasterBackend,
    check: &dyn Fn() -> bool,
) -> Result<ImageRasterImage, RasterError> {
    render_compiled_images(compile_images(request, images, check)?, backend, check)
}
pub fn render_compiled_images(
    compiled: CompiledImageRaster<'_>,
    backend: &mut dyn RasterBackend,
    check: &dyn Fn() -> bool,
) -> Result<ImageRasterImage, RasterError> {
    let raster = crate::execute(compiled.raster, backend, Some(compiled.images), check)?;
    Ok(ImageRasterImage {
        raster,
        work: compiled.work,
        resources_sha256: compiled.images.sha256().clone(),
    })
}
