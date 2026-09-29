//! Source declarations, not a computed chart layout or applied default values.
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SourceChartLayout {
    /// Source order; missing property, missing val and explicit lexical val differ.
    pub properties: Vec<SourceChartProperty>,
    /// Complex markup remains bound to its original part and physical ordinal.
    pub markup: Vec<SourceChartMarkup>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SourceChartProperty {
    pub source_ordinal: u32,
    pub kind: ChartPropertyKind,
    /// XML attribute value, without numeric conversion or schema defaulting.
    pub value: Option<String>,
}
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, JsonSchema,
)]
#[serde(rename_all = "camelCase")]
pub enum ChartPropertyKind {
    BarDirection,
    Grouping,
    VaryColors,
    GapWidth,
    GapDepth,
    Overlap,
    FirstSliceAngle,
    HoleSize,
    Delete,
    AxisPosition,
    MajorTickMark,
    MinorTickMark,
    TickLabelPosition,
    CrossAxis,
    Crosses,
    CrossesAt,
    CrossBetween,
    MajorUnit,
    MinorUnit,
    Auto,
    LabelAlignment,
    LabelOffset,
    TickLabelSkip,
    TickMarkSkip,
    NoMultiLevelLabels,
    BaseTimeUnit,
    MajorTimeUnit,
    MinorTimeUnit,
    LogBase,
    Orientation,
    Minimum,
    Maximum,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SourceChartMarkup {
    pub source_ordinal: u32,
    pub kind: ChartMarkupKind,
}
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, JsonSchema,
)]
#[serde(rename_all = "camelCase")]
pub enum ChartMarkupKind {
    ShapeProperties,
    TextProperties,
    Title,
    MajorGridlines,
    MinorGridlines,
    DisplayUnits,
    DataLabels,
    SeriesLines,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SourceChartAxis {
    pub source_ordinal: u32,
    pub kind: ChartAxisKind,
    pub id: u32,
    pub layout: SourceChartLayout,
    pub scaling: Option<SourceChartScaling>,
    pub number_format: Option<SourceChartNumberFormat>,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum ChartAxisKind {
    Category,
    Value,
    Date,
    Series,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SourceChartScaling {
    pub source_ordinal: u32,
    pub properties: Vec<SourceChartProperty>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SourceChartNumberFormat {
    pub source_ordinal: u32,
    pub format_code: Option<String>,
    /// Preserve omission and the actual lexical boolean; no workbook lookup.
    pub source_linked: Option<String>,
}
