use crate::source::{SourceColorMapRef, color::*, fill::resolve::*, theme::SourceThemeSchemeRef};
use mo_common::Digest;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SourceFillColorQuery {
    pub expected_source_sha256: Digest,
    pub surface: String,
    pub targets: Vec<FillTarget>,
    pub fill_profile: FillProfile,
    pub color_profile: ColorProfile,
    pub context: ColorContext,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SourceFillColors {
    pub source_sha256: Digest,
    pub surface: String,
    pub fill_profile: FillProfile,
    pub color_profile: ColorProfile,
    pub color_mapping: Option<SourceColorMapRef>,
    pub color_scheme: Option<SourceThemeSchemeRef>,
    pub targets: Vec<SourceFillColorResult>,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SourceFillColorResult {
    pub target: FillTarget,
    pub style: FillOutcome,
    pub colors: FillPaintColors,
    /// A background redirect may change context within the same drawing batch.
    /// Absent means the enclosing batch's color mapping and scheme apply.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub context_override: Option<FillColorSurface>,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FillColorSurface {
    pub surface: String,
    pub color_mapping: Option<SourceColorMapRef>,
    pub color_scheme: Option<SourceThemeSchemeRef>,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "camelCase", deny_unknown_fields)]
pub enum FillPaintColors {
    None {},
    UnresolvedStyle {},
    /// Image relationships remain in style; this is not a transparent paint.
    ImageResourcesRequired {},
    Solid {
        color: FillColorEvaluation,
    },
    /// Same order and cardinality as style.gradient.stops; never sorted/deduped.
    Gradient {
        stops: Vec<FillColorEvaluation>,
    },
    Pattern {
        foreground: FillColorEvaluation,
        background: FillColorEvaluation,
    },
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FillColorEvaluation {
    pub outcome: ColorSample,
    /// Present only when evaluation actually consulted a native reference.
    pub placeholder: Option<Box<FillPlaceholderBinding>>,
    pub dependencies: Vec<ColorDependency>,
    pub notices: Vec<ColorNotice>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FillPlaceholderBinding {
    pub owner: FillOwner,
    /// Shared table declarations live outside the consuming object's surface.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_part: Option<String>,
    pub reference_ordinal: u32,
    pub color_ordinal: Option<u32>,
}
#[derive(Debug, Clone, Copy)]
pub struct FillColorLimits {
    pub fills: FillResolveLimits,
    /// max_queries counts selected color slots, including each gradient stop.
    /// Steps and percentage bytes are shared by all slots and native lookups.
    pub colors: ColorLimits,
}
impl Default for FillColorLimits {
    fn default() -> Self {
        Self {
            fills: FillResolveLimits::default(),
            colors: ColorLimits {
                max_queries: 65_536,
                ..ColorLimits::default()
            },
        }
    }
}
