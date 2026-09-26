use crate::source::{
    SourceColorMapRef,
    color::{ColorContext, ColorDependency, ColorLimits, ColorNotice, ColorProfile},
    line::resolve::{LineOutcome, LineProfile, LineResolveLimits},
    theme::SourceThemeSchemeRef,
};
use mo_common::Digest;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SourceLineColorQuery {
    pub expected_source_sha256: Digest,
    pub surface: String,
    pub objects: Vec<u32>,
    pub line_profile: LineProfile,
    pub color_profile: ColorProfile,
    /// File style context takes precedence for outer phClr. This host context
    /// supplies system colors and any phClr inside that style's own expression.
    pub context: ColorContext,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SourceLineColors {
    pub source_sha256: Digest,
    pub surface: String,
    pub line_profile: LineProfile,
    pub color_profile: ColorProfile,
    pub color_mapping: Option<SourceColorMapRef>,
    pub color_scheme: Option<SourceThemeSchemeRef>,
    pub objects: Vec<SourceLineColorResult>,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SourceLineColorResult {
    pub native_id: u32,
    pub style: LineOutcome,
    pub paint: LinePaintColor,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "camelCase", deny_unknown_fields)]
pub enum LinePaintColor {
    None {},
    UnresolvedStyle {},
    Solid {
        outcome: LineColorSample,
        dependencies: Vec<ColorDependency>,
        notices: Vec<ColorNotice>,
    },
}
pub use crate::source::color::ColorSample as LineColorSample;
#[derive(Debug, Clone, Copy, Default)]
pub struct LineColorLimits {
    pub lines: LineResolveLimits,
    pub colors: ColorLimits,
}
