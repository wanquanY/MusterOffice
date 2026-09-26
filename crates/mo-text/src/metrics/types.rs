use crate::{EffectiveVariation, ShapeVariation};
use mo_common::Digest;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, JsonSchema,
)]
#[serde(rename_all = "camelCase")]
pub enum FontMetric {
    HorizontalAscender,
    HorizontalDescender,
    HorizontalLineGap,
    HorizontalClippingAscent,
    HorizontalClippingDescent,
    VerticalAscender,
    VerticalDescender,
    VerticalLineGap,
    HorizontalCaretRise,
    HorizontalCaretRun,
    HorizontalCaretOffset,
    VerticalCaretRise,
    VerticalCaretRun,
    VerticalCaretOffset,
    XHeight,
    CapHeight,
    SubscriptXSize,
    SubscriptYSize,
    SubscriptXOffset,
    SubscriptYOffset,
    SuperscriptXSize,
    SuperscriptYSize,
    SuperscriptXOffset,
    SuperscriptYOffset,
    StrikeoutSize,
    StrikeoutOffset,
    UnderlineSize,
    UnderlineOffset,
}
impl FontMetric {
    pub fn tag(self) -> &'static str {
        use FontMetric::*;
        match self {
            HorizontalAscender => "hasc",
            HorizontalDescender => "hdsc",
            HorizontalLineGap => "hlgp",
            HorizontalClippingAscent => "hcla",
            HorizontalClippingDescent => "hcld",
            VerticalAscender => "vasc",
            VerticalDescender => "vdsc",
            VerticalLineGap => "vlgp",
            HorizontalCaretRise => "hcrs",
            HorizontalCaretRun => "hcrn",
            HorizontalCaretOffset => "hcof",
            VerticalCaretRise => "vcrs",
            VerticalCaretRun => "vcrn",
            VerticalCaretOffset => "vcof",
            XHeight => "xhgt",
            CapHeight => "cpht",
            SubscriptXSize => "sbxs",
            SubscriptYSize => "sbys",
            SubscriptXOffset => "sbxo",
            SubscriptYOffset => "sbyo",
            SuperscriptXSize => "spxs",
            SuperscriptYSize => "spys",
            SuperscriptXOffset => "spxo",
            SuperscriptYOffset => "spyo",
            StrikeoutSize => "strs",
            StrikeoutOffset => "stro",
            UnderlineSize => "unds",
            UnderlineOffset => "undo",
        }
    }
    pub(crate) fn word(self) -> u32 {
        u32::from_be_bytes(self.tag().as_bytes().try_into().unwrap())
    }
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FontMetricsRequest {
    pub expected_sha256: Digest,
    pub face_index: u32,
    pub instances: Vec<FontMetricsInstance>,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FontMetricsInstance {
    pub variations: Vec<ShapeVariation>,
    pub metrics: Vec<FontMetric>,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FontMetricsResult {
    pub font_sha256: Digest,
    pub face_index: u32,
    pub units_per_em: u16,
    pub position_units_per_em: u32,
    pub profile: String,
    pub instances: Vec<MeasuredInstance>,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MeasuredInstance {
    pub effective_variations: Vec<EffectiveVariation>,
    pub values: Vec<MeasuredMetric>,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MeasuredMetric {
    pub metric: FontMetric,
    /// None means the component could not read this metric; zero is a real value.
    pub position: Option<i32>,
}
