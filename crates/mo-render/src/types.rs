use mo_geometry::{Affine, Fixed};
use mo_raster::{FillPath, RasterInfo, RasterViewport, StrokeStyle};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TransformNode {
    /// Optional outer transform. Parents must precede children.
    pub parent: Option<u32>,
    pub affine: Affine,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PathInstance {
    #[serde(default, skip_serializing_if = "mo_raster::BlendMode::is_default")]
    pub blend: mo_raster::BlendMode,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub clip: Option<u32>,
    pub path: u32,
    /// None is the identity transform.
    pub transform: Option<u32>,
    /// Already evaluated world-space paint, independent of this path transform.
    pub brush: mo_raster::Brush,
    /// Evaluated world-width stroke; not transformed by the geometry matrix.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stroke: Option<StrokeStyle>,
}
/// Evaluated resources/instances, independent of authoring document semantics.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DrawScene {
    /// Intervals refer to instances; lowering preserves their order and count.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub opacity_groups: Vec<mo_raster::OpacityGroup>,
    pub paths: Vec<FillPath>,
    pub transforms: Vec<TransformNode>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub clips: Vec<ClipNode>,
    pub instances: Vec<PathInstance>,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SceneRasterRequest {
    pub viewport: RasterViewport,
    pub scene: DrawScene,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SceneWork {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub clips: Option<SceneClipWork>,
    /// 2 means the shared matrix candidate failed numeric/precision checks and
    /// lowering used the original node chain before any backend call.
    pub lowering_attempts: u32,
    pub source_paths: u32,
    pub source_commands: u32,
    pub transforms: u32,
    pub maximum_depth: u32,
    pub evaluated_points: u32,
    /// Conservative point/node work per attempt, including identity instances.
    pub point_transform_work: u32,
    pub compiled_paths: u32,
    pub compiled_commands: u32,
    /// Quantization/composition error bound, raw Q32 document units (EMU).
    pub transform_error_bound: Fixed,
    /// Transform + final device quantization, raw Q32 pixels.
    pub combined_coordinate_error_bound: Fixed,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SceneRasterInfo {
    pub profile: String,
    pub raster: RasterInfo,
    pub work: SceneWork,
}

/// Evaluated intersection path; geometry has its own transform. Parent clipping
/// remains in world coordinates and is never transformed by a child or draw.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ClipNode {
    pub parent: Option<u32>,
    pub path: u32,
    pub transform: Option<u32>,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SceneClipWork {
    pub source_nodes: u32,
    pub compiled_nodes: u32,
}
