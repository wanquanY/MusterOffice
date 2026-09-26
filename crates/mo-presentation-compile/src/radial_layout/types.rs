use crate::native_paths::{NativePathError, NativePathLimits};
use mo_geometry::{Fixed, GeometryError};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Explicit development policies; original source values are never rewritten.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CircleFocusBasis {
    /// Initial interpretation: both focus position and size use circle bounds.
    CircumscribedSquare,
    /// Focus position uses the anchor; scales apply to the outer circle about it.
    AnchorRectangle,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RadialLayoutOptions {
    /// Per-axis Q32 local EMU path/bounds tolerance, relative to evaluated guides.
    /// Derived focus errors may be larger. Excludes page placement and application differences.
    pub coordinate_tolerance: Fixed,
}
#[derive(Debug, Clone, Copy)]
pub struct RadialLayoutLimits {
    pub max_targets: usize,
    pub max_bounds_steps: u32,
    pub paths: NativePathLimits,
}
impl Default for RadialLayoutLimits {
    fn default() -> Self {
        Self {
            max_targets: 4096,
            max_bounds_steps: 4_000_000,
            paths: NativePathLimits::default(),
        }
    }
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[serde(bound(serialize = "[Fixed; N]: Serialize"))]
#[schemars(bound = "[Fixed; N]: JsonSchema")]
pub struct RadialEstimate<const N: usize> {
    /// Local Q32 EMU, or dimensionless Q32 for focusScale. Rectangle order LTRB.
    pub values: [Fixed; N],
    /// Nonnegative absolute error per returned value. Not a pixel color bound.
    pub errors: [Fixed; N],
}
#[derive(Debug, Clone, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct NativeRadialLayout {
    pub profile: String,
    pub path_bounds: RadialEstimate<4>,
    pub tile_rectangle: RadialEstimate<4>,
    pub outer_center: RadialEstimate<2>,
    pub outer_radius: RadialEstimate<1>,
    pub inner_center: RadialEstimate<2>,
    pub inner_radii: RadialEstimate<2>,
    pub focus_point: RadialEstimate<2>,
    pub focus_scale: RadialEstimate<2>,
    /// Preserve declared orientation. Page placement is not applied here.
    pub rotate_with_shape: bool,
    pub work: RadialLayoutWork,
}
#[derive(Debug, Clone, Copy, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RadialLayoutWork {
    pub paths: u32,
    pub commands: u32,
    pub bounds_steps: u32,
    pub arc_segments: u32,
}
#[derive(Debug, thiserror::Error)]
pub enum RadialLayoutError {
    #[error("radial layout input: {0}")]
    Invalid(&'static str),
    #[error("radial layout limit: {0}")]
    Limit(&'static str),
    #[error("radial layout outside Q32 range")]
    Range,
    #[error("radial layout cancelled")]
    Cancelled,
    #[error(transparent)]
    Path(#[from] NativePathError),
    #[error(transparent)]
    Bounds(#[from] GeometryError),
}
