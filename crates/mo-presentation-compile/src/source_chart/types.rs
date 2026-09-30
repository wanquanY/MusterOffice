use crate::chart_geometry::{ChartGeometry, ChartGeometryLimits};
use mo_charts::sectors::NegativeWeights;
use mo_common::Digest;
use mo_geometry::{Fixed, Point};
use mo_presentation_source::source::{SourceObjectRef, charts::*};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, JsonSchema)]
pub enum SourceCircularProfile {
    /// Explicit source angles/hole size; no guessed application layout defaults.
    #[serde(rename = "source-cache-declared-circular-plot-v1-draft")]
    DeclaredCircularDraftV1,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SourceCircularRequest {
    pub expected_source_sha256: Digest,
    pub object: SourceObjectRef,
    pub plot_source_ordinal: u32,
    pub profile: SourceCircularProfile,
    /// Resolved plot geometry, not the graphicFrame's outer box or a guessed
    /// automatic layout. Labels/legend reserve space in their own layout stage.
    pub center: Point,
    pub outer_radius: Fixed,
    pub coordinate_tolerance: Fixed,
    pub negative_weights: NegativeWeights,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SourceCircularGeometry {
    pub profile: SourceCircularProfile,
    pub source_sha256: Digest,
    pub object: SourceObjectRef,
    pub chart_part: String,
    pub chart_sha256: Digest,
    pub plot_source_ordinal: u32,
    pub native_kind: String,
    pub first_slice_degrees: u16,
    pub hole_percent: u8,
    pub data_authority: ChartDataAuthority,
    pub external_data: Option<SourceChartExternalData>,
    /// Source series order, with the innermost doughnut ring first. Stable series
    /// indices are retained independently of physical XML or drawing order.
    pub series: Vec<SourceCircularSeries>,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SourceCircularSeries {
    pub index: u32,
    pub order: u32,
    pub source_ordinal: u32,
    pub values_source_ordinal: u32,
    pub formula: Option<String>,
    pub points: Vec<SourceCircularPoint>,
    pub layout: SourceChartLayout,
    pub point_overrides: Vec<SourceChartPointOverride>,
    pub inner_radius: Fixed,
    pub outer_radius: Fixed,
    /// Upstream angle/radius conversion error in local Q32 EMU, not included in
    /// geometry's per-path bounds. Consumers must use the combined bound below.
    pub source_geometry_error_bound: Fixed,
    /// Maximum complete per-axis bound across all paths, including source error.
    pub coordinate_error_bound: Fixed,
    pub geometry: ChartGeometry,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SourceCircularPoint {
    pub index: u32,
    pub source_ordinal: u32,
    pub format_code: Option<String>,
}
#[derive(Debug, Clone, Copy, Default)]
pub struct SourceCircularLimits {
    pub source: SourceChartLimits,
    pub geometry: ChartGeometryLimits,
}
#[derive(Debug, thiserror::Error)]
pub enum SourceCircularError {
    #[error(transparent)]
    Source(#[from] mo_presentation_source::PptxError),
    #[error(transparent)]
    Geometry(#[from] crate::chart_geometry::ChartGeometryError),
    #[error("unresolved circular chart at ordinal {source_ordinal}: {reason}")]
    Unresolved {
        source_ordinal: u32,
        series_index: Option<u32>,
        point_index: Option<u32>,
        reason: &'static str,
    },
}
