use super::*;
use mo_charts::sectors::{ExactWeightRatios, NegativeWeights, SectorLimits};
use mo_common::Digest;
use mo_presentation_source::source::SourceObjectRef;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub enum ChartLabelProfile {
    #[serde(rename = "source-chart-label-bindings-v1-draft")]
    DeclaredBindingsV1,
}
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, JsonSchema,
)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ChartLabelTarget {
    pub series_index: u32,
    pub point_index: u32,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SourceChartLabelRequest {
    pub expected_source_sha256: Digest,
    pub object: SourceObjectRef,
    pub plot_source_ordinal: u32,
    pub profile: ChartLabelProfile,
    pub negative_weights: NegativeWeights,
    /// Explicit stable identities; no allocation of a dense array from sparse idx.
    pub targets: Vec<ChartLabelTarget>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SourceChartLabels {
    pub profile: ChartLabelProfile,
    pub source_sha256: Digest,
    pub object: SourceObjectRef,
    pub chart_part: String,
    pub chart_sha256: Digest,
    pub plot_source_ordinal: u32,
    pub data_authority: ChartDataAuthority,
    pub negative_weights: NegativeWeights,
    pub labels: Vec<ChartLabelPlan>,
    /// Each required series is normalized once. Fractions are not sector angles.
    pub normalizations: Vec<ChartLabelNormalization>,
    /// Unique native text cascades, shared by labels with the same declaration chain.
    pub text_cascades: Vec<mo_presentation_source::source::text::cascade::ChartTextOutcome>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ChartLabelNormalization {
    pub series_index: u32,
    pub channel_source_ordinal: u32,
    pub ratios: ExactWeightRatios,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ChartLabelPlan {
    pub target: ChartLabelTarget,
    /// Most specific first. Missing declarations do not clear inherited fields.
    pub annotation_chain: Vec<u32>,
    pub settings: ChartLabelSettings,
    /// Named data bindings, not final display order or formatted display text.
    pub components: Vec<ChartLabelComponent>,
    pub custom_text_source: Option<u32>,
    /// Index into text_cascades. None requires chart-style/default text resolution.
    pub text_cascade: Option<u32>,
    /// Office displays a legend key only alongside a selected text component or tx.
    pub legend_key_visible: Option<bool>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ChartLabelValue<T> {
    pub value: T,
    pub source_ordinal: u32,
    /// Present CT_Boolean/numFmt with omitted val/sourceLinked uses schema true.
    pub schema_defaulted: bool,
}
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, JsonSchema,
)]
#[serde(rename_all = "camelCase")]
pub enum ChartLabelFlag {
    LegendKey,
    Value,
    CategoryName,
    SeriesName,
    Percent,
    BubbleSize,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ChartLabelSettings {
    pub deleted: Option<ChartLabelValue<bool>>,
    pub flags: BTreeMap<ChartLabelFlag, ChartLabelValue<bool>>,
    /// Missing throughout the chain: application/chart-style defaults still needed.
    pub unresolved_flags: Vec<ChartLabelFlag>,
    pub position: Option<ChartLabelValue<ChartLabelPosition>>,
    pub separator: Option<ChartLabelSeparator>,
    pub number_format: Option<ChartLabelNumberFormat>,
    pub show_leader_lines: Option<ChartLabelValue<bool>>,
    pub text_property_roots: Vec<u32>,
    pub shape_property_roots: Vec<u32>,
    pub layout_source_ordinal: Option<u32>,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub enum ChartLabelPosition {
    #[serde(rename = "bestFit")]
    BestFit,
    #[serde(rename = "b")]
    Bottom,
    #[serde(rename = "ctr")]
    Center,
    #[serde(rename = "inBase")]
    InsideBase,
    #[serde(rename = "inEnd")]
    InsideEnd,
    #[serde(rename = "l")]
    Left,
    #[serde(rename = "outEnd")]
    OutsideEnd,
    #[serde(rename = "r")]
    Right,
    #[serde(rename = "t")]
    Top,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub enum ChartLabelSeparator {
    Declared { value: String, source_ordinal: u32 },
    CommaDefault {},
    PieCategoryPercentLineBreak {},
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ChartLabelNumberFormat {
    pub source_ordinal: u32,
    pub code: String,
    pub source_linked: ChartLabelValue<bool>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ChartLabelDataFormat {
    pub source_ordinal: u32,
    pub code: String,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub enum ChartLabelComponent {
    Text {
        role: ChartLabelFlag,
        channel_source_ordinal: u32,
        source_ordinal: u32,
        value: String,
    },
    Number {
        role: ChartLabelFlag,
        channel_source_ordinal: u32,
        source_ordinal: u32,
        value: mo_charts::DecimalNumber,
        format: Option<ChartLabelDataFormat>,
    },
    Percent {
        normalization: u32,
        point_index: u32,
        format: Option<ChartLabelDataFormat>,
    },
}
#[derive(Debug, Clone, Copy)]
pub struct SourceChartLabelLimits {
    pub source: SourceChartLimits,
    pub ratios: SectorLimits,
    pub max_targets: usize,
    pub max_data_points: usize,
    pub max_retained_bytes: usize,
    pub text: mo_presentation_source::source::text::cascade::TextCascadeLimits,
}
impl Default for SourceChartLabelLimits {
    fn default() -> Self {
        Self {
            source: Default::default(),
            ratios: Default::default(),
            max_targets: 4096,
            max_data_points: 65536,
            max_retained_bytes: 8 * 1024 * 1024,
            text: Default::default(),
        }
    }
}
#[derive(Debug, thiserror::Error)]
pub enum ChartLabelError {
    #[error(transparent)]
    Source(#[from] PptxError),
    #[error(transparent)]
    Ratios(#[from] mo_charts::sectors::SectorError),
    #[error("unresolved chart label at ordinal {source_ordinal}: {reason}")]
    Unresolved {
        source_ordinal: u32,
        series_index: Option<u32>,
        point_index: Option<u32>,
        reason: &'static str,
    },
}
