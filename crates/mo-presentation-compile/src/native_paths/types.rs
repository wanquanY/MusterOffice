use mo_geometry::{Fixed, PathCommand, Point};
use mo_pptx::source::geometry::NativePathFill;
use mo_pptx::source::geometry::evaluate::GeometryOrigin;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, JsonSchema)]
pub enum NativePathProfile {
    #[serde(rename = "drawingml-polar-arcs-q96-hermite-v1-draft")]
    DrawingmlPolarArcsDraftV1,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct NativePathOptions {
    pub profile: NativePathProfile,
    /// Per-axis local shape EMU, Q32. Does not include upstream guide error,
    /// page transforms, raster conversion, coverage or application differences.
    pub coordinate_tolerance: Fixed,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct NativePathSpan {
    pub origin: GeometryOrigin,
    pub first_command: u32,
    pub command_count: u32,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CompiledNativePath {
    pub origin: GeometryOrigin,
    pub commands: Vec<PathCommand>,
    pub source_map: Vec<NativePathSpan>,
    /// Source paint modifiers are preserved; paint resolution owns defaults.
    pub fill: Option<NativePathFill>,
    pub stroke: Option<bool>,
    pub extrusion_ok: Option<bool>,
    pub numeric_error_bound: Point,
    pub curve_error_bound: Point,
    pub coordinate_error_bound: Point,
    pub arc_segments: u32,
}
#[derive(Debug, Clone, Copy, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum NativePathIssue {
    InvalidExtent,
    ZeroPathExtent,
    NegativeRadius,
    DegenerateArcRadius,
    NumericRange,
    PrecisionExceeded,
}
#[derive(Debug, thiserror::Error)]
pub enum NativePathError {
    #[error("native path {origin:?}: {issue:?}")]
    Geometry {
        origin: GeometryOrigin,
        issue: NativePathIssue,
    },
    #[error("invalid native path options")]
    Options,
    #[error("native path limit: {0}")]
    Limit(&'static str),
    #[error("native path cancelled")]
    Cancelled,
}
#[derive(Debug, Clone, Copy)]
pub struct NativePathLimits {
    pub max_paths: u32,
    pub max_commands: u32,
    pub max_steps: u32,
    pub max_arc_segments: u32,
    pub max_arc_turns: u32,
}
impl Default for NativePathLimits {
    fn default() -> Self {
        Self {
            max_paths: 4096,
            max_commands: 262144,
            max_steps: 4_000_000,
            max_arc_segments: 262144,
            max_arc_turns: 128,
        }
    }
}
