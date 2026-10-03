//! Source declarations, not a computed chart layout or applied default values.
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SourceChartLayout {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub marker: Option<SourceChartMarker>,
    /// Source order; missing property, missing val and explicit lexical val differ.
    pub properties: Vec<SourceChartProperty>,
    /// Complex markup remains bound to its original part and physical ordinal.
    pub markup: Vec<SourceChartMarkup>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub unrecognized_children: Vec<SourceChartUnknown>,
    /// Known elements carrying attributes that this declaration model does not interpret.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub retained_attribute_ordinals: Vec<u32>,
}
impl SourceChartLayout {
    pub fn is_empty(&self) -> bool {
        self.marker.is_none()
            && self.properties.is_empty()
            && self.markup.is_empty()
            && self.unrecognized_children.is_empty()
            && self.retained_attribute_ordinals.is_empty()
    }
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SourceChartMarker {
    pub source_ordinal: u32,
    pub symbol: Option<String>,
    pub size: Option<u8>,
    pub retained_ordinals: Vec<u32>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SourceChartUnknown {
    pub source_ordinal: u32,
    pub namespace: String,
    pub local_name: String,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SourceChartProperty {
    pub source_ordinal: u32,
    pub kind: ChartPropertyKind,
    /// XML val attribute (or separator character data), without schema defaulting.
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
    Explosion,
    Bubble3D,
    InvertIfNegative,
    Smooth,
    LabelPosition,
    ShowLegendKey,
    ShowValue,
    ShowCategoryName,
    ShowSeriesName,
    ShowPercent,
    ShowBubbleSize,
    Separator,
    ShowLeaderLines,
    LegendPosition,
    Overlay,
    LayoutTarget,
    XMode,
    YMode,
    WidthMode,
    HeightMode,
    X,
    Y,
    Width,
    Height,
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
    Marker,
    PictureOptions,
    Trendline,
    ErrorBars,
    Extensions,
    DataLabel,
    LegendEntry,
    Layout,
    ManualLayout,
    TextSource,
    LeaderLines,
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
