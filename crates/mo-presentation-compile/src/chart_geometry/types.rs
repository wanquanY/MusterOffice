use mo_charts::sectors::{SectorLayout, SectorLimits, SectorRequest};
use mo_geometry::{Fixed, PathCommand, Point};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

pub const PROFILE: &str = "circular-chart-sectors-q96-hermite-v1-draft";

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ChartGeometryRequest {
    pub sectors: SectorRequest,
    /// Explicit local chart coordinates, Q32 EMU. No layout/theme defaults.
    pub center: Point,
    pub outer_radius: Fixed,
    /// Zero creates pie wedges; positive creates a doughnut. Must be < outer.
    pub inner_radius: Fixed,
    /// Maximum per-axis local error, including decimal angular allocation,
    /// interval/control conversion and curve interpolation, Q32 EMU.
    pub coordinate_tolerance: Fixed,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ChartSectorPath {
    pub point_index: u32,
    /// Empty for zero-weight sectors. Positive sub-resolution sectors fail with
    /// Precision rather than disappearing from a successful geometry result.
    /// Full rings use opposite winding subpaths, without a radial stroke seam.
    pub commands: Vec<PathCommand>,
    pub numeric_error_bound: Fixed,
    pub curve_error_bound: Fixed,
    pub angular_error_bound: Fixed,
    pub coordinate_error_bound: Fixed,
    pub arc_segments: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ChartGeometry {
    pub profile: String,
    pub layout: SectorLayout,
    pub fill_rule: mo_raster::FillRule,
    pub paths: Vec<ChartSectorPath>,
    pub work: ChartGeometryWork,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ChartGeometryWork {
    pub paths: u32,
    pub commands: u32,
    pub arc_segments: u32,
    /// Bounded arithmetic stages; each trigonometric evaluation charges 24.
    pub steps: u32,
}

#[derive(Debug, Clone, Copy)]
pub struct ChartGeometryLimits {
    pub sectors: SectorLimits,
    pub max_paths: u32,
    pub max_commands: u32,
    pub max_arc_segments: u32,
    pub max_steps: u32,
}
impl Default for ChartGeometryLimits {
    fn default() -> Self {
        Self {
            sectors: SectorLimits::default(),
            max_paths: 4096,
            max_commands: 262144,
            max_arc_segments: 131072,
            max_steps: 4_000_000,
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ChartGeometryError {
    #[error(transparent)]
    Sectors(#[from] mo_charts::sectors::SectorError),
    #[error("invalid chart geometry: {0}")]
    Invalid(&'static str),
    #[error("chart geometry limit: {0}")]
    Limit(&'static str),
    #[error("chart geometry coordinate tolerance cannot be met")]
    Precision,
    #[error("chart geometry numeric range exceeded")]
    Range,
    #[error("chart geometry cancelled")]
    Cancelled,
}
