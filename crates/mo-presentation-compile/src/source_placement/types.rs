use crate::AffineUncertainty;
use mo_common::Digest;
use mo_geometry::{Affine, Point};
use mo_pptx::source::{SourceObjectRef, SourcePlaceholderMatch};
use mo_presentation_model::{Point as NativePoint, Size};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
#[derive(Debug, Clone, Copy, Serialize, Deserialize, JsonSchema)]
pub enum SourcePlacementProfile {
    #[serde(rename = "drawingml-source-sector-scale-q96-v1-draft")]
    DrawingmlSourceDraftV1,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SourcePlacementQuery {
    pub expected_source_sha256: Digest,
    pub surface: String,
    pub objects: Vec<u32>,
    pub profile: SourcePlacementProfile,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "camelCase", deny_unknown_fields)]
pub enum TransformValueSource {
    Declaration {
        object: SourceObjectRef,
    },
    /// Schema/profile default at the selected transform's owner.
    Default {
        object: SourceObjectRef,
    },
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TransformValue<T> {
    pub value: T,
    pub source: TransformValueSource,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ResolvedNativeTransform {
    pub origin: TransformValue<NativePoint>,
    pub size: TransformValue<Size>,
    pub rotation: TransformValue<i32>,
    pub flip_horizontal: TransformValue<bool>,
    pub flip_vertical: TransformValue<bool>,
    pub child_origin: Option<TransformValue<NativePoint>>,
    pub child_size: Option<TransformValue<Size>>,
    /// Office ignores a graphic frame's own orientation attributes; parent
    /// group orientation still applies. The original values remain above.
    pub graphic_frame_orientation_ignored: bool,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct NativePlacement {
    pub transform: ResolvedNativeTransform,
    /// Effective coordinate viewport. A zero/missing group child extent uses
    /// the corresponding target extent, producing unit scale on that axis.
    pub source_size: Size,
    pub source_origin: NativePoint,
    pub anchor: Point,
    /// Already in surface coordinates; subtract anchor, then apply once.
    pub affine: Affine,
    pub uncertainty: AffineUncertainty,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub enum PlacementCause {
    MissingOrigin,
    MissingSize,
    Inheritance { status: SourcePlaceholderMatch },
    RetainedTransform { source_ordinal: u32 },
    InvalidCoordinate { field: String },
    NumericRange,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PlacementUnresolved {
    pub object: SourceObjectRef,
    pub cause: PlacementCause,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "status", rename_all = "camelCase", deny_unknown_fields)]
pub enum SourcePlacementOutcome {
    Resolved { placement: Box<NativePlacement> },
    Unresolved { reason: PlacementUnresolved },
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct NativeObjectPlacement {
    pub native_id: u32,
    pub parent_group: Option<u32>,
    pub depth: u32,
    pub outcome: SourcePlacementOutcome,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SourcePlacements {
    pub source_sha256: Digest,
    pub surface: String,
    pub profile: SourcePlacementProfile,
    /// Presentation spTree transform declarations are retained but do not move
    /// page contents in this profile. Nested grpSp transforms do participate.
    pub root_transform_ignored: bool,
    pub objects: Vec<NativeObjectPlacement>,
    pub unique_angles: u32,
}
#[derive(Debug, Clone, Copy)]
pub struct SourcePlacementLimits {
    pub max_queries: usize,
    pub max_indexed_objects: usize,
    pub max_steps: usize,
    pub max_depth: u32,
}
impl Default for SourcePlacementLimits {
    fn default() -> Self {
        Self {
            max_queries: 256,
            max_indexed_objects: 100_000,
            max_steps: 1_000_000,
            max_depth: 128,
        }
    }
}
#[derive(Debug, thiserror::Error)]
pub enum SourcePlacementError {
    #[error("source placement digest conflict")]
    SourceConflict,
    #[error("invalid source placement: {0}")]
    Invalid(&'static str),
    #[error("source placement limit: {0}")]
    Limit(&'static str),
    #[error("source placement cancelled")]
    Cancelled,
}
