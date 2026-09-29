use crate::{CompileError, PagePlacementRequest};
use mo_common::{Digest, ObjectId, SlideId};
use mo_geometry::Fixed;
use mo_presentation_model::{ContainerId, Rgba, ThemeColor};
use mo_raster::{RasterError, RasterViewport};
use mo_render::{SceneRasterInfo, SceneRasterRequest, SceneWork};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

pub const PAGE_PROFILE: &str = "author-page-solid-paths-certified-cpu-v2-draft";
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PagePaintDefaults {
    pub theme_colors: BTreeMap<ThemeColor, Rgba>,
    pub page_background: Rgba,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PageRenderRequest {
    pub page: PagePlacementRequest,
    /// Zero origin, uniform scale, and dimensions rounded up to whole pixels.
    /// A fractional final pixel retains the exact document boundary clip.
    pub viewport: RasterViewport,
    pub defaults: PagePaintDefaults,
}
#[derive(Debug, Clone, Copy, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum PageFeature {
    Table,
    ShapeText,
    Picture,
    Connector,
    InheritedFill,
    InheritedStroke,
    UnresolvedStrokeParameters,
    GroupStroke,
    MissingThemeColor,
}
#[derive(Debug, thiserror::Error)]
pub enum PageError {
    #[error(transparent)]
    Placement(#[from] CompileError),
    #[error(transparent)]
    Raster(#[from] RasterError),
    #[error("page theme color is unresolved: {slot:?} ({object:?})")]
    MissingThemeColor {
        object: Option<ObjectId>,
        slot: ThemeColor,
    },
    #[error("page feature not implemented: {feature:?} ({object:?})")]
    Unsupported {
        object: Option<ObjectId>,
        feature: PageFeature,
    },
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PagePaintSource {
    /// None denotes the page background. Groups create spaces, not paint draws.
    pub object: Option<ObjectId>,
    pub container: ContainerId,
    pub instance: u32,
    pub paint: PagePaintKind,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum PagePaintKind {
    Fill,
    Stroke,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PagePlanInfo {
    pub profile: String,
    pub placement_profile: String,
    pub document_sha256: Digest,
    pub slide: SlideId,
    pub hidden: bool,
    pub source_objects: u32,
    pub generated_commands: u32,
    pub curve_segments: u32,
    /// Raw Q32 pixels, including author matrix quantization at control points.
    pub author_coordinate_error_bound: Fixed,
    /// Raw Q32 pixels: curve approximation + control-coordinate quantization.
    pub geometry_coordinate_error_bound: Fixed,
    /// Author percentage -> Draw IR conversion, dimensionless raw Q32.
    /// Separate from device miter quantization and pixel coordinate budgets.
    pub author_miter_limit_error_bound: Fixed,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PagePlan {
    pub info: PagePlanInfo,
    /// Reduced device tolerance reserves the explicit author/geometry budgets.
    pub raster: SceneRasterRequest,
    pub paint_sources: Vec<PagePaintSource>,
    pub device_work: SceneWork,
    pub combined_coordinate_error_bound: Fixed,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PageRasterInfo {
    pub page: PagePlanInfo,
    pub scene: SceneRasterInfo,
    /// All three stages, in raw Q32 pixels; not a raster coverage error bound.
    pub combined_coordinate_error_bound: Fixed,
}
pub struct PageImage {
    pub info: PageRasterInfo,
    pub pixels: Vec<u8>,
}
