use super::super::{SourceObjectRef, SourceTransform};
use mo_common::Digest;
use mo_presentation_model::{Point, Size};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Direct native declarations, never resolved placement or inherited values.
/// Angles use native 1/60000 degree units; signed/multi-turn values are retained.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SourceTransformValues {
    pub origin: Option<Point>,
    pub size: Option<Size>,
    pub child_origin: Option<Point>,
    pub child_size: Option<Size>,
    pub rotation: Option<i32>,
    pub flip_horizontal: Option<bool>,
    pub flip_vertical: Option<bool>,
}
impl From<&SourceTransform> for SourceTransformValues {
    fn from(v: &SourceTransform) -> Self {
        Self {
            origin: v.origin,
            size: v.size,
            child_origin: v.child_origin,
            child_size: v.child_size,
            rotation: v.rotation,
            flip_horizontal: v.flip_horizontal,
            flip_vertical: v.flip_vertical,
        }
    }
}
impl SourceTransformValues {
    pub(super) fn apply(&self, v: &mut SourceTransform) {
        v.origin = self.origin;
        v.size = self.size;
        v.child_origin = self.child_origin;
        v.child_size = self.child_size;
        v.rotation = self.rotation;
        v.flip_horizontal = self.flip_horizontal;
        v.flip_vertical = self.flip_vertical;
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SourceTransformEdit {
    /// Immutable source part and native object ID, including the shape-tree ID.
    pub target: SourceObjectRef,
    pub expected: SourceTransformValues,
    pub replacement: SourceTransformValues,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SourceTransformEdits {
    pub expected_source_sha256: Digest,
    pub edits: Vec<SourceTransformEdit>,
}
