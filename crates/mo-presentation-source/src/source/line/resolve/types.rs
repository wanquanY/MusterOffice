use super::super::*;
use crate::source::{SourceObjectRef, SourcePlaceholderMatch, drawingml::*};
use mo_common::Digest;

/// Explicit interpretation of the documented multi-pass DrawingML rules.
/// This is not a certificate for a particular Office/WPS version or renderer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub enum LineProfile {
    #[serde(rename = "ms-oi29500-lines-2024-draft-v1")]
    Drawingml2024DraftV1,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SourceLineQuery {
    pub expected_source_sha256: Digest,
    pub surface: String,
    pub objects: Vec<u32>,
    pub profile: LineProfile,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SourceLineStyles {
    pub source_sha256: Digest,
    pub surface: String,
    pub profile: LineProfile,
    /// Request order is retained, including duplicates.
    pub objects: Vec<SourceLineResult>,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SourceLineResult {
    pub native_id: u32,
    pub outcome: LineOutcome,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "status", rename_all = "camelCase", deny_unknown_fields)]
pub enum LineOutcome {
    Resolved { line: Box<EffectiveLine> },
    Unresolved { reason: LineUnresolved },
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub enum LineOrigin {
    /// Physical DrawingML chart declaration, not a slide shape/native ID.
    Chart {
        part: String,
        source_ordinal: u32,
    },
    TableCell {
        object: SourceObjectRef,
        cell: crate::source::table::SourceCellAddress,
        edge: crate::source::table::TableCellEdge,
        source_ordinal: u32,
    },
    TableStyle {
        part: String,
        object: SourceObjectRef,
        region: crate::source::table::styles::TableStyleRegion,
        edge: crate::source::table::TableStyleEdge,
        source_ordinal: u32,
    },
    TableTheme {
        part: String,
        source_ordinal: u32,
        via: SourceObjectRef,
        region: crate::source::table::styles::TableStyleRegion,
        edge: crate::source::table::TableStyleEdge,
        reference_ordinal: u32,
        style_index: u32,
    },
    Object {
        object: SourceObjectRef,
        source_ordinal: u32,
    },
    Theme {
        part: String,
        source_ordinal: u32,
        via: SourceObjectRef,
        reference_ordinal: u32,
        style_index: u32,
    },
    ProfileDefault {},
}
impl LineOrigin {
    pub(super) fn at(&self, ordinal: u32) -> Self {
        let mut result = self.clone();
        match &mut result {
            Self::Object { source_ordinal, .. }
            | Self::Chart { source_ordinal, .. }
            | Self::Theme { source_ordinal, .. }
            | Self::TableCell { source_ordinal, .. }
            | Self::TableStyle { source_ordinal, .. }
            | Self::TableTheme { source_ordinal, .. } => *source_ordinal = ordinal,
            Self::ProfileDefault {} => (),
        }
        result
    }
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct LineValue<T> {
    pub value: T,
    pub declared_by: LineOrigin,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct LineColorTerm {
    pub value: SourceColorValue,
    pub transforms: Vec<SourceColorTransform>,
    pub declared_by: LineOrigin,
}
/// Retains native color operations and their owning style context. Colors are
/// not sampled/quantized during line inheritance; color evaluation is separate.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct LineColorExpression {
    pub color: LineColorTerm,
    pub placeholder: Option<LineColorTerm>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub enum EffectiveLineFill {
    None {
        declared_by: LineOrigin,
    },
    Solid {
        declared_by: LineOrigin,
        color: Box<LineColorExpression>,
    },
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub enum EffectiveLineDash {
    Preset {
        declared_by: LineOrigin,
        value: LineValue<NativePresetDash>,
    },
    Custom {
        declared_by: LineOrigin,
        stops: Vec<SourceDashStop>,
    },
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub enum EffectiveLineJoin {
    Round {
        declared_by: LineOrigin,
    },
    Bevel {
        declared_by: LineOrigin,
    },
    Miter {
        declared_by: LineOrigin,
        limit: LineValue<NativePercentage>,
    },
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct EffectiveLineEnd {
    pub declared_by: LineOrigin,
    pub kind: LineValue<NativeLineEnd>,
    pub width: LineValue<NativeLineEndSize>,
    pub length: LineValue<NativeLineEndSize>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct EffectiveLine {
    pub width: LineValue<Emu>,
    pub cap: LineValue<NativeLineCap>,
    pub compound: LineValue<NativeCompoundLine>,
    pub alignment: LineValue<NativePenAlignment>,
    pub fill: EffectiveLineFill,
    pub dash: EffectiveLineDash,
    pub join: EffectiveLineJoin,
    pub head: EffectiveLineEnd,
    pub tail: EffectiveLineEnd,
}
/// Stroke geometry is independent of paint. Table lines can therefore use the
/// complete fill engine, including gradient/pattern paint, without losing dash,
/// compound, alignment, cap, join or endpoint declarations.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct EffectiveLineGeometry {
    pub width: LineValue<Emu>,
    pub cap: LineValue<NativeLineCap>,
    pub compound: LineValue<NativeCompoundLine>,
    pub alignment: LineValue<NativePenAlignment>,
    pub dash: EffectiveLineDash,
    pub join: EffectiveLineJoin,
    pub head: EffectiveLineEnd,
    pub tail: EffectiveLineEnd,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "status", rename_all = "camelCase", deny_unknown_fields)]
pub enum LineGeometryOutcome {
    Resolved {
        geometry: Box<EffectiveLineGeometry>,
    },
    Unresolved {
        reason: LineUnresolved,
    },
}
impl EffectiveLineGeometry {
    pub(super) fn with_fill(self, fill: EffectiveLineFill) -> EffectiveLine {
        EffectiveLine {
            width: self.width,
            cap: self.cap,
            compound: self.compound,
            alignment: self.alignment,
            dash: self.dash,
            join: self.join,
            head: self.head,
            tail: self.tail,
            fill,
        }
    }
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub enum LineUnresolved {
    TableGrid {
        object: SourceObjectRef,
        reason: crate::source::table::grid::NativeTableGridIssue,
    },
    TableStyle {
        object: SourceObjectRef,
        reason: crate::source::table::styles::TableStyleSelectionError,
    },
    UnsupportedObject {
        object: SourceObjectRef,
    },
    Placeholder {
        object: SourceObjectRef,
        matching: SourcePlaceholderMatch,
    },
    MissingFormatScheme {
        object: SourceObjectRef,
    },
    StyleIndexOutOfRange {
        object: SourceObjectRef,
        index: u32,
        available: u32,
    },
    RetainedContent {
        origin: LineOrigin,
    },
    UnsupportedFill {
        origin: LineOrigin,
        native_kind: NativeRetainedLineFill,
    },
}
#[derive(Debug, Clone, Copy)]
pub struct LineResolveLimits {
    pub max_queries: usize,
    pub max_steps: usize,
    /// Charges all traversed declarations before cloning result values.
    pub max_values: usize,
    pub max_lexical_bytes: usize,
}
impl Default for LineResolveLimits {
    fn default() -> Self {
        Self {
            max_queries: 256,
            max_steps: 1_000_000,
            max_values: 262_144,
            max_lexical_bytes: 4 * 1024 * 1024,
        }
    }
}
