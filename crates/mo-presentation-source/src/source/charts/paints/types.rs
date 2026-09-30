use super::*;
use crate::source::{
    SourceColorMapRef, color::*, fill::*, line::SourceLine, theme::SourceThemeSchemeRef,
};
use mo_common::Digest;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SourceChartPaintQuery {
    pub expected_source_sha256: Digest,
    pub object: SourceObjectRef,
    pub profile: ColorProfile,
    pub context: ColorContext,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SourceChartPaints {
    pub source_sha256: Digest,
    pub object: SourceObjectRef,
    pub profile: ColorProfile,
    /// Series and point layout references bind to the declaration ordinals below.
    pub chart: SourceChartPart,
    pub color_mapping: Option<SourceColorMapRef>,
    pub color_scheme: Option<SourceThemeSchemeRef>,
    pub theme_override: Option<ChartThemeOverride>,
    /// Source order. This is not a resolved chart-style/series/point cascade.
    pub declarations: Vec<ChartPaintDeclaration>,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ChartThemeOverride {
    pub part: String,
    pub sha256: Digest,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ChartPaintDeclaration {
    pub source_ordinal: u32,
    pub black_white_mode: Option<NativeBlackWhiteMode>,
    pub fill: Option<SourceFill>,
    pub line: Option<SourceLine>,
    pub effects: Option<crate::source::effects::SourceEffectProperties>,
    pub effect_nodes: BTreeMap<u32, crate::source::effects::SourceEffectNode>,
    /// Geometry, extensions and unknown attributes remain source-bound.
    pub retained_ordinals: Vec<u32>,
    /// Every declared fill/line color is evaluated once, without quantized reuse.
    pub colors: Vec<ChartPaintColor>,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ChartPaintColor {
    pub source_ordinal: u32,
    pub outcome: ColorSample,
    pub dependencies: Vec<ColorDependency>,
    pub notices: Vec<ColorNotice>,
}
#[derive(Debug, Clone, Copy)]
pub struct ChartPaintLimits {
    pub source: SourceChartLimits,
    pub colors: ColorLimits,
    pub max_declarations: usize,
}
impl Default for ChartPaintLimits {
    fn default() -> Self {
        Self {
            source: SourceChartLimits::default(),
            colors: ColorLimits::default(),
            max_declarations: 4096,
        }
    }
}
