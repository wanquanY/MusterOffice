use crate::source::{SourceResolvedValue, geometry::NativePathFill};
use mo_common::Digest;
use mo_presentation_model::Size;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub enum GeometryProfile {
    #[serde(rename = "ecma376-2016-ms-presets-draft-v2")]
    Drawingml2016PresetsDraftV2,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SourceGeometryQuery {
    pub expected_source_sha256: Digest,
    pub surface: String,
    pub objects: Vec<u32>,
    pub profile: GeometryProfile,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SourceGeometryValues {
    pub source_sha256: Digest,
    pub surface: String,
    pub profile: GeometryProfile,
    pub objects: Vec<GeometryResult>,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct GeometryResult {
    pub native_id: u32,
    pub outcome: GeometryOutcome,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "status", rename_all = "camelCase", deny_unknown_fields)]
pub enum GeometryOutcome {
    Resolved { geometry: Box<EvaluatedGeometry> },
    Unresolved { reason: GeometryUnresolved },
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub enum GeometryUnresolved {
    MissingDeclaration,
    UnsupportedObject,
    MissingExtent,
    InvalidExtent,
    RetainedContent {
        origin: GeometryOrigin,
    },
    Formula {
        origin: GeometryOrigin,
        issue: FormulaIssue,
    },
    InvalidCoordinate {
        origin: GeometryOrigin,
    },
    InvalidAngle {
        origin: GeometryOrigin,
    },
    UnknownReference {
        origin: GeometryOrigin,
        token: String,
    },
    ReservedGuide {
        origin: GeometryOrigin,
    },
    InvalidGuideName {
        origin: GeometryOrigin,
    },
    InvalidHandleReference {
        origin: GeometryOrigin,
    },
    NumericRange {
        origin: GeometryOrigin,
    },
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum FormulaIssue {
    UnknownOperation,
    Arity,
    DivisionByZero,
    UndefinedDirection,
    TangentPole,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct EvaluatedGeometry {
    pub source_ordinal: u32,
    pub extent: SourceResolvedValue<Size>,
    /// Source order is preserved. Repeated names bind to the last prior value.
    pub adjustments: Vec<GuideValue>,
    pub guides: Vec<GuideValue>,
    pub handles: Vec<EvaluatedHandle>,
    pub connections: Vec<EvaluatedConnection>,
    /// Absence is not replaced with an invented native text rectangle.
    pub text_rect: Option<EvaluatedRect>,
    pub paths: Vec<EvaluatedPath>,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct GuideValue {
    pub origin: GeometryOrigin,
    pub name: String,
    pub value: f64,
    pub dependencies: Vec<GuideDependency>,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub enum GuideDependency {
    Builtin { name: String },
    Guide { origin: GeometryOrigin },
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct EvaluatedPoint {
    pub origin: GeometryOrigin,
    pub x: f64,
    pub y: f64,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct EvaluatedRect {
    pub origin: GeometryOrigin,
    pub left: f64,
    pub top: f64,
    pub right: f64,
    pub bottom: f64,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub enum EvaluatedHandle {
    Xy {
        origin: GeometryOrigin,
        position: EvaluatedPoint,
        guide_x: Option<GeometryOrigin>,
        min_x: Option<f64>,
        max_x: Option<f64>,
        guide_y: Option<GeometryOrigin>,
        min_y: Option<f64>,
        max_y: Option<f64>,
    },
    Polar {
        origin: GeometryOrigin,
        position: EvaluatedPoint,
        guide_radius: Option<GeometryOrigin>,
        min_radius: Option<f64>,
        max_radius: Option<f64>,
        guide_angle: Option<GeometryOrigin>,
        min_angle: Option<f64>,
        max_angle: Option<f64>,
    },
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct EvaluatedConnection {
    pub origin: GeometryOrigin,
    pub angle: f64,
    pub position: EvaluatedPoint,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct EvaluatedPath {
    pub origin: GeometryOrigin,
    /// Native path-space extents. Scaling into shape space is a later step.
    pub width: Option<mo_common::Emu>,
    pub height: Option<mo_common::Emu>,
    pub fill: Option<NativePathFill>,
    pub stroke: Option<bool>,
    pub extrusion_ok: Option<bool>,
    pub commands: Vec<EvaluatedCommand>,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub enum EvaluatedCommand {
    Move {
        origin: GeometryOrigin,
        to: EvaluatedPoint,
    },
    Line {
        origin: GeometryOrigin,
        to: EvaluatedPoint,
    },
    Arc {
        origin: GeometryOrigin,
        width_radius: f64,
        height_radius: f64,
        start_angle: f64,
        sweep_angle: f64,
    },
    Quadratic {
        origin: GeometryOrigin,
        control: EvaluatedPoint,
        to: EvaluatedPoint,
    },
    Cubic {
        origin: GeometryOrigin,
        control1: EvaluatedPoint,
        control2: EvaluatedPoint,
        to: EvaluatedPoint,
    },
    Close {
        origin: GeometryOrigin,
    },
}
#[derive(Debug, Clone, Copy)]
pub struct GeometryLimits {
    pub max_queries: usize,
    pub max_steps: usize,
    pub max_lexical_bytes: usize,
    pub max_values: usize,
}
impl Default for GeometryLimits {
    fn default() -> Self {
        Self {
            max_queries: 256,
            max_steps: 1_000_000,
            max_lexical_bytes: 8 * 1024 * 1024,
            max_values: 262_144,
        }
    }
}

/// Ordinals are zero-based element preorder in the indicated XML resource.
/// Preset ordinals address the pinned, generated catalog definition, not the
/// document part. A document override always retains its physical source.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub enum GeometryOrigin {
    Document {
        source_ordinal: u32,
    },
    Preset {
        preset: super::super::NativeShapeType,
        definition_ordinal: u32,
    },
}
impl GeometryOrigin {
    pub fn document(source_ordinal: u32) -> Self {
        Self::Document { source_ordinal }
    }
}
