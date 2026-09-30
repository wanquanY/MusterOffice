use super::super::{SourceCompatibility, SourceObjectRef};
use super::{SourceChartAxis, SourceChartLayout};
use mo_common::{ByteLength, Digest};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SourceChartQuery {
    pub expected_source_sha256: Digest,
    /// Exact physical surface; inherited chart placement is a separate calculation.
    pub surface: String,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SourceCharts {
    pub source_sha256: Digest,
    pub surface: String,
    pub bindings: Vec<SourceChartBinding>,
    /// Shared native chart parts are inspected once, indexed by bindings.
    pub charts: Vec<SourceChartPart>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SourceChartBinding {
    pub object: SourceObjectRef,
    pub source_ordinal: u32,
    pub relationship_id: String,
    pub chart: u32,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SourceChartPart {
    pub part: String,
    pub sha256: Digest,
    pub byte_length: ByteLength,
    pub compatibility: SourceCompatibility,
    /// Opaque extension roots remain physically bound even though inspection
    /// does not interpret their descendants. Consumers must not silently admit.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub extension_ordinals: Vec<u32>,
    pub plots: Vec<SourceChartPlot>,
    /// Source declarations; reference resolution, defaults and geometry are separate.
    pub axes: Vec<SourceChartAxis>,
    pub external_data: Option<SourceChartExternalData>,
    /// Cache declarations are snapshots, not recalculated workbook values.
    pub data_authority: ChartDataAuthority,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum ChartDataAuthority {
    SourceCacheSnapshot,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SourceChartPlot {
    pub source_ordinal: u32,
    /// Native local name (barChart, doughnutChart, ...), not a rendering capability.
    pub native_kind: String,
    pub axis_ids: Vec<u32>,
    pub series: Vec<SourceChartSeries>,
    pub layout: SourceChartLayout,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SourceChartSeries {
    pub source_ordinal: u32,
    pub index: u32,
    pub order: u32,
    pub channels: Vec<SourceChartChannel>,
    #[serde(default, skip_serializing_if = "SourceChartLayout::is_empty")]
    pub layout: SourceChartLayout,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub point_overrides: Vec<SourceChartPointOverride>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SourceChartPointOverride {
    pub source_ordinal: u32,
    pub index: u32,
    pub layout: SourceChartLayout,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum ChartChannelRole {
    Title,
    Categories,
    Values,
    XValues,
    YValues,
    BubbleSize,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum ChartCacheKind {
    Number,
    String,
    MultiLevelString,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SourceChartChannel {
    pub source_ordinal: u32,
    pub role: ChartChannelRole,
    /// Native container name; distinguishes reference, literal and title text.
    pub native_kind: String,
    pub formula: Option<String>,
    pub literal_text: Option<String>,
    pub cache: Option<SourceChartCache>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SourceChartCache {
    pub source_ordinal: u32,
    pub kind: ChartCacheKind,
    pub declared_point_count: Option<u32>,
    pub format_code: Option<String>,
    /// Source level order; no expansion of sparse point indices or missing values.
    pub levels: Vec<Vec<SourceChartPoint>>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SourceChartPoint {
    pub source_ordinal: u32,
    pub index: u32,
    /// Exact lexical value. Missing v, empty v, zero and errors remain distinct.
    pub value: Option<String>,
    pub format_code: Option<String>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SourceChartExternalData {
    pub source_ordinal: u32,
    pub relationship_id: String,
    /// None means omitted; no eager refresh is performed by inspection.
    pub auto_update: Option<bool>,
    pub target: ChartWorkbookTarget,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub enum ChartWorkbookTarget {
    Embedded {
        part: String,
        content_type: String,
        sha256: Digest,
        byte_length: ByteLength,
    },
    External {
        uri: String,
    },
}
#[derive(Debug, Clone, Copy)]
pub struct SourceChartLimits {
    pub max_charts: usize,
    pub max_bindings: usize,
    pub max_series: usize,
    pub max_axes: usize,
    pub max_points: usize,
    pub max_elements: usize,
    /// Logical retained XML cost, identical on native and WASM.
    pub max_metadata_bytes: usize,
    pub max_part_bytes: usize,
    pub max_total_part_bytes: usize,
    pub max_relationship_steps: usize,
}
impl Default for SourceChartLimits {
    fn default() -> Self {
        Self {
            max_charts: 256,
            max_bindings: 4096,
            max_series: 16384,
            max_axes: 4096,
            max_points: 1_000_000,
            max_elements: 262_144,
            max_metadata_bytes: 16 * 1024 * 1024,
            max_part_bytes: 16 * 1024 * 1024,
            max_total_part_bytes: 64 * 1024 * 1024,
            max_relationship_steps: 1_000_000,
        }
    }
}
