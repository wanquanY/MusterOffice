use super::super::{SourceColorMapRef, theme::*};
use mo_common::Digest;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// A named, provisional numerical interpretation, not an Office/WPS certificate.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub enum ColorProfile {
    #[serde(rename = "ecma376-2016-draft-v1")]
    Ecma3762016DraftV1,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ColorContext {
    pub system_colors: BTreeMap<SystemColor, [u8; 3]>,
    /// Explicit unassociated sRGB and alpha, supplied by the owning style.
    pub placeholder: Option<[u8; 4]>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SourceColorQuery {
    pub expected_source_sha256: Digest,
    pub surface: String,
    pub profile: ColorProfile,
    pub colors: Vec<SchemeColor>,
    pub context: ColorContext,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SourceColorPalette {
    pub source_sha256: Digest,
    pub surface: String,
    pub profile: ColorProfile,
    pub color_mapping: Option<SourceColorMapRef>,
    pub color_scheme: Option<SourceThemeSchemeRef>,
    /// Input order, including repeated queries.
    pub colors: Vec<SchemeColorResult>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SchemeColorResult {
    pub scheme: SchemeColor,
    pub outcome: ColorOutcome,
    /// Outer-to-inner dependency path. Original transforms are not rewritten.
    pub dependencies: Vec<ColorDependency>,
    pub notices: Vec<ColorNotice>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(
    tag = "status",
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub enum ColorOutcome {
    Resolved {
        rgba8: [u8; 4],
        rgba16: [u16; 4],
        clipped_for_srgb: bool,
    },
    Unresolved {
        reason: ColorUnresolved,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub enum ColorDependency {
    Theme {
        part: String,
        slot: ColorSlot,
        source_ordinal: u32,
    },
    System {
        color: SystemColor,
        origin: SystemColorOrigin,
    },
    Placeholder,
}
#[derive(Debug, Clone, Copy, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum SystemColorOrigin {
    HostContext,
    FileLastColor,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub enum ColorUnresolved {
    MissingColorMap,
    MissingColorScheme,
    MissingThemeSlot { slot: ColorSlot },
    MissingSystemColor { color: SystemColor },
    MissingPlaceholder,
    RetainedPlaceholderContext { part: String, source_ordinal: u32 },
    SchemeCycle { slot: ColorSlot },
    NumericRange,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum ColorNotice {
    /// ECMA does not give coefficients. This profile explicitly uses 22/72/6.
    GrayWeightsProvisional,
    /// 2016 table says 250,250,120 for ltGoldenrodYellow, unlike its long alias.
    PresetAliasDiscrepancy,
}

#[derive(Debug, Clone, Copy)]
pub struct ColorLimits {
    pub max_queries: usize,
    /// Shared across all queries, referenced colors and transforms.
    pub max_steps: usize,
    pub max_percentage_bytes: usize,
}
impl Default for ColorLimits {
    fn default() -> Self {
        Self {
            max_queries: 256,
            max_steps: 65_536,
            max_percentage_bytes: 65_536,
        }
    }
}
