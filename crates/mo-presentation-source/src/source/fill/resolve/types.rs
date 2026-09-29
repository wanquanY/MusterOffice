use crate::source::{SourcePlaceholderMatch, drawingml::*, fill::*};
use mo_common::Digest;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// An explicit interpretation, not a certificate for any Office/WPS version.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, JsonSchema)]
pub enum FillProfile {
    #[serde(rename = "ms-oi29500-fills-2024-draft-v1")]
    Drawingml2024DraftV1,
}
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, JsonSchema)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub enum FillTarget {
    Object {
        native_id: u32,
    },
    Line {
        native_id: u32,
    },
    Picture {
        native_id: u32,
    },
    /// Physical cell formatting. Covered cells remain queryable; page painting
    /// selects merge origins separately.
    TableCell {
        native_id: u32,
        cell: crate::source::table::SourceCellAddress,
    },
    TableCellBorder {
        native_id: u32,
        cell: crate::source::table::SourceCellAddress,
        edge: crate::source::table::TableCellEdge,
    },
    TableBackground {
        native_id: u32,
    },
    /// A real table-style declaration instantiated by this native table. None
    /// selects tblBg. Also identifies the exact lazy phClr reference context.
    TableStyleFill {
        native_id: u32,
        region: Option<crate::source::table::styles::TableStyleRegion>,
    },
    TableStyleBorder {
        native_id: u32,
        region: crate::source::table::styles::TableStyleRegion,
        edge: crate::source::table::TableStyleEdge,
    },
    RootGroup {},
    Background {},
}
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FillOwner {
    pub part: String,
    pub target: FillTarget,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SourceFillQuery {
    pub expected_source_sha256: Digest,
    pub surface: String,
    pub targets: Vec<FillTarget>,
    pub profile: FillProfile,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SourceFillStyles {
    pub source_sha256: Digest,
    pub surface: String,
    pub profile: FillProfile,
    pub targets: Vec<SourceFillResult>,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SourceFillResult {
    pub target: FillTarget,
    pub outcome: FillOutcome,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "status", rename_all = "camelCase", deny_unknown_fields)]
pub enum FillOutcome {
    Resolved {
        fill: Box<EffectiveFill>,
        redirects: Vec<FillRedirect>,
    },
    Unresolved {
        reason: FillUnresolved,
    },
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub enum FillOrigin {
    Declaration {
        owner: FillOwner,
        source_ordinal: u32,
    },
    TableStyle {
        part: String,
        source_ordinal: u32,
        via: FillOwner,
    },
    Theme {
        part: String,
        source_ordinal: u32,
        via: FillOwner,
        reference_ordinal: u32,
        style_index: u32,
    },
    SchemaDefault {
        part: String,
        source_ordinal: u32,
    },
    ProfileDefault {},
}
impl FillOrigin {
    pub(crate) fn part(&self) -> Option<&str> {
        match self {
            Self::Declaration { owner, .. } => Some(&owner.part),
            Self::Theme { part, .. }
            | Self::TableStyle { part, .. }
            | Self::SchemaDefault { part, .. } => Some(part),
            Self::ProfileDefault {} => None,
        }
    }
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FillValue<T> {
    pub value: T,
    pub declared_by: FillOrigin,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FillColorTerm {
    pub value: SourceColorValue,
    pub transforms: Vec<SourceColorTransform>,
    pub declared_by: FillOrigin,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FillColorExpression {
    pub color: FillColorTerm,
    /// Resolve a style reference color in this native owner only if color
    /// evaluation actually reaches phClr. Inheritance does not sample colors.
    pub context_owner: Option<FillOwner>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FillRedirect {
    pub declared_by: FillOrigin,
    pub target: FillOwner,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub enum EffectiveFill {
    None {
        declared_by: FillOrigin,
    },
    Solid {
        declared_by: FillOrigin,
        color: Box<FillColorExpression>,
    },
    Gradient {
        declared_by: FillOrigin,
        gradient: Box<EffectiveGradientFill>,
    },
    Pattern {
        declared_by: FillOrigin,
        pattern: Box<EffectivePatternFill>,
    },
    Image {
        declared_by: FillOrigin,
        image: Box<EffectiveImageFill>,
    },
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct EffectiveGradientFill {
    pub stops: FillValue<Vec<EffectiveGradientStop>>,
    pub shade: EffectiveGradientShade,
    pub tile_rect: EffectiveFillRect,
    pub flip: FillValue<NativeTileFlip>,
    pub rotate_with_shape: FillValue<bool>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct EffectiveGradientStop {
    pub position: FillValue<NativePercentage>,
    pub color: FillColorExpression,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub enum EffectiveGradientShade {
    Linear {
        declared_by: FillOrigin,
        angle: FillValue<u32>,
        scaled: FillValue<bool>,
    },
    Path {
        declared_by: FillOrigin,
        path: FillValue<NativePathShade>,
        fill_to_rect: Box<EffectiveFillRect>,
    },
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct EffectiveFillRect {
    pub declared_by: FillOrigin,
    pub left: FillValue<NativePercentage>,
    pub top: FillValue<NativePercentage>,
    pub right: FillValue<NativePercentage>,
    pub bottom: FillValue<NativePercentage>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct EffectivePatternFill {
    pub preset: FillValue<NativePattern>,
    pub foreground: FillColorExpression,
    pub background: FillColorExpression,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct EffectiveImageFill {
    /// Each relationship belongs to the part in its own declaring origin.
    /// Empty strings are explicit or profile-default empty relationship IDs.
    pub embed: FillValue<String>,
    pub link: FillValue<String>,
    pub compression: FillValue<NativeBlipCompression>,
    pub source_rect: EffectiveFillRect,
    pub mode: EffectiveImageMode,
    pub dpi: FillValue<u32>,
    pub rotate_with_shape: FillValue<bool>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub enum EffectiveImageMode {
    Tile {
        declared_by: FillOrigin,
        tile: EffectiveFillTile,
    },
    Stretch {
        declared_by: FillOrigin,
        fill_rect: EffectiveFillRect,
    },
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct EffectiveFillTile {
    pub translate_x: FillValue<NativeCoordinate>,
    pub translate_y: FillValue<NativeCoordinate>,
    pub scale_x: FillValue<NativePercentage>,
    pub scale_y: FillValue<NativePercentage>,
    pub flip: FillValue<NativeTileFlip>,
    pub alignment: FillValue<NativeFillAlignment>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub enum FillUnresolved {
    TableGrid {
        owner: FillOwner,
        reason: crate::source::table::grid::NativeTableGridIssue,
    },
    TableStyle {
        owner: FillOwner,
        reason: crate::source::table::styles::TableStyleSelectionError,
    },
    UnsupportedTarget {
        owner: FillOwner,
    },
    Placeholder {
        owner: FillOwner,
        matching: SourcePlaceholderMatch,
    },
    MissingFormatScheme {
        owner: FillOwner,
    },
    StyleIndexOutOfRange {
        owner: FillOwner,
        index: u32,
        available: u32,
    },
    RetainedContent {
        origin: FillOrigin,
    },
    EffectEvaluationRequired {
        origin: FillOrigin,
    },
    MissingImage {
        origin: FillOrigin,
    },
    GroupWithoutParent {
        origin: FillOrigin,
    },
    UnsupportedBackgroundMode {
        owner: FillOwner,
    },
}
#[derive(Debug, Clone, Copy)]
pub struct FillResolveLimits {
    pub max_queries: usize,
    pub max_steps: usize,
    pub max_values: usize,
    pub max_lexical_bytes: usize,
    pub max_group_hops: usize,
}
impl Default for FillResolveLimits {
    fn default() -> Self {
        Self {
            max_queries: 256,
            max_steps: 1_000_000,
            max_values: 262_144,
            max_lexical_bytes: 4 * 1024 * 1024,
            max_group_hops: 256,
        }
    }
}
