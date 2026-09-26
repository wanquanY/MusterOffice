use crate::{native_paths::NativePathError, source_placement::*};
use mo_common::Digest;
use mo_geometry::Fixed;
use mo_pptx::source::{
    SourceObjectKind, SourcePlaceholderMatch, SourceVisualIssue, SurfaceKind,
    color::ColorContext,
    fill::colors::SourceFillColorResult,
    geometry::evaluate::{GeometryOrigin, GeometryUnresolved},
    line::colors::SourceLineColorResult,
};
use mo_raster::{RasterError, RasterViewport};
use mo_render::{SceneRasterInfo, SceneRasterRequest, SceneWork};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, JsonSchema)]
pub enum SourcePageProfile {
    #[serde(rename = "drawingml-static-solid-page-v1-draft")]
    StaticSolidDraftV1,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SourcePageRequest {
    pub expected_source_sha256: Digest,
    pub slide: String,
    pub profile: SourcePageProfile,
    pub viewport: RasterViewport,
    pub color_context: ColorContext,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SourcePageLocation {
    pub part: String,
    /// None denotes a surface/background declaration.
    pub object: Option<u32>,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub enum SourcePageIssue {
    Visual {
        issue: SourceVisualIssue,
    },
    Object {
        native_kind: SourceObjectKind,
    },
    Text {
        source_ordinal: u32,
    },
    Effects {
        source_ordinal: u32,
    },
    Placeholder {
        matching: SourcePlaceholderMatch,
    },
    SpecialPlaceholder {},
    Placement {
        reason: PlacementUnresolved,
    },
    Geometry {
        reason: GeometryUnresolved,
    },
    Fill {},
    Line {},
    PathFillModifier {},
    FillSpace {
        redirects: Vec<mo_pptx::source::fill::resolve::FillRedirect>,
    },
}
#[derive(Debug, thiserror::Error)]
pub enum SourcePageError {
    #[error("source page object {location:?}: {error}")]
    AtObject {
        location: SourcePageLocation,
        error: Box<SourcePageError>,
    },
    #[error("source page digest conflict")]
    SourceConflict,
    #[error("invalid source page: {0}")]
    Invalid(&'static str),
    #[error("source page mapping required at {location:?}: {issue:?}")]
    Mapping {
        location: SourcePageLocation,
        issue: Box<SourcePageIssue>,
    },
    #[error(transparent)]
    Source(#[from] mo_pptx::PptxError),
    #[error(transparent)]
    Placement(#[from] SourcePlacementError),
    #[error(transparent)]
    Path(#[from] NativePathError),
    #[error(transparent)]
    RadialLayout(#[from] crate::radial_layout::RadialLayoutError),
    #[error(transparent)]
    Raster(#[from] RasterError),
    #[error(transparent)]
    ImageDecode(#[from] mo_image::ImageError),
    #[error(transparent)]
    ImageLayout(#[from] crate::source_image_layout::ImageLayoutError),
    #[error(transparent)]
    ImagePaint(#[from] crate::source_image_paint::ImagePaintError),
    #[error("source page image resource required: {0:?}")]
    ImageResource(Box<mo_pptx::source::images::SourceImageResult>),
    #[error("source text context required at {location:?}")]
    TextContextRequired {
        location: SourcePageLocation,
        source_ordinal: u32,
    },
    #[error(transparent)]
    Text(#[from] crate::source_frame::SourceFrameError),
    #[error(transparent)]
    TextPaint(#[from] mo_pptx::source::text::paint::TextPaintError),
    #[error(transparent)]
    TextGeometry(#[from] mo_geometry::GeometryError),
    #[error("unusable native text decoration metric: {0:?}")]
    TextDecoration(Box<crate::source_text_page::TextDecorationIssue>),
    #[error(
        "native glyph crosses different source paints in paragraph {paragraph}, cluster {start}..{end}"
    )]
    GlyphPaintConflict {
        paragraph: u32,
        start: u32,
        end: u32,
    },
}
impl SourcePageError {
    pub(crate) fn at(self, location: &SourcePageLocation) -> Self {
        Self::AtObject {
            location: location.clone(),
            error: Box::new(self),
        }
    }
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SourcePageLayer {
    pub part: String,
    pub kind: SurfaceKind,
    pub visible: bool,
    pub objects: Vec<u32>,
    pub hidden_objects: Vec<u32>,
    pub template_placeholders: Vec<u32>,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SourcePagePaintBinding {
    pub location: SourcePageLocation,
    /// The object's own drawing surface supplies ordinary color/style context.
    /// useBgFill explicitly redirects to the consuming slide's background.
    /// Placeholder inheritance retains original declaration owners.
    pub drawing_surface: String,
    pub fill: SourceFillColorResult,
    /// p:blipFill is a second fill above spPr fill and below the outline.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub picture_fill: Option<SourceFillColorResult>,
    pub line: Option<SourceLineColorResult>,
    pub placement: Option<NativePlacement>,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SourcePagePaintSource {
    pub instance: u32,
    pub binding: u32,
    pub path: Option<GeometryOrigin>,
    pub paint: crate::PagePaintKind,
    /// Present for the distinct p:blipFill paint. Ordinary shape/text paints
    /// retain their existing provenance through binding/path/text_sources.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fill_target: Option<mo_pptx::source::fill::resolve::FillTarget>,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SourcePageInfo {
    pub profile: SourcePageProfile,
    pub source_sha256: Digest,
    pub slide: String,
    pub hidden_slide: bool,
    pub layers: Vec<SourcePageLayer>,
    pub generated_commands: u32,
    pub arc_segments: u32,
    pub placement_coordinate_error_bound: Fixed,
    /// Relative to evaluated binary64 guide values. Does not bound upstream
    /// guide evaluation, coverage, color or application compatibility.
    pub path_coordinate_error_bound: Fixed,
    /// Upstream image fill-rectangle clip error in Q32 device pixels, before
    /// shared scene lowering. Image brush uncertainty is accounted downstream.
    #[serde(default, skip_serializing_if = "zero")]
    pub image_clip_coordinate_error_bound: Fixed,
}
fn zero(value: &Fixed) -> bool {
    *value == Fixed::ZERO
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SourcePagePlan {
    pub info: SourcePageInfo,
    pub raster: SceneRasterRequest,
    pub bindings: Vec<SourcePagePaintBinding>,
    pub paint_sources: Vec<SourcePagePaintSource>,
    pub device_work: SceneWork,
    pub downstream_coordinate_error_bound: Fixed,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SourcePageRasterInfo {
    pub page: SourcePageInfo,
    pub scene: SceneRasterInfo,
    pub downstream_coordinate_error_bound: Fixed,
}
pub struct SourcePageImage {
    pub info: SourcePageRasterInfo,
    pub pixels: Vec<u8>,
}
