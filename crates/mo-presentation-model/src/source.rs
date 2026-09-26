//! Immutable provenance and partially understood native content. Editable text
//! and coordinates live in the same object registry and revision as authored
//! objects; the source package is retained through an authorized resource ID.
use crate::*;
use mo_common::*;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub enum SourceBindingProfile {
    #[serde(rename = "presentationml-retained-fields-v1-draft")]
    PresentationmlRetainedFieldsV1,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SourceBindings {
    pub profile: SourceBindingProfile,
    pub resource: ResourceId,
    pub slides: BTreeMap<SlideId, String>,
    pub masters: BTreeMap<MasterId, String>,
    pub layouts: BTreeMap<LayoutId, String>,
    pub themes: BTreeMap<ThemeId, String>,
    pub objects: BTreeMap<ObjectId, NativeObjectBinding>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct NativeObjectBinding {
    pub part: String,
    pub native_id: u32,
    /// None means the complete direct transform can be changed in place.
    pub transform_constraint: Option<NativeEditConstraint>,
    pub runs: BTreeMap<RunId, NativeRunBinding>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct NativeRunBinding {
    pub paragraph: u32,
    pub run: u32,
    pub constraint: Option<NativeEditConstraint>,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum NativeEditConstraint {
    MissingDirectTransform,
    RetainedTransform,
    CompatibilityBranch,
    StructuredLeaf,
    DynamicField,
    TimingReferences,
    RetainedReferences,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum RetainedObjectKind {
    Shape,
    Picture,
    Group,
    Connector,
    GraphicFrame,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RetainedParagraph {
    pub id: ParagraphId,
    pub runs: Vec<RetainedTextRun>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RetainedTextRun {
    pub id: RunId,
    pub kind: RetainedRunKind,
    pub text: String,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum RetainedRunKind {
    Text,
    Break,
    Field,
}
impl RetainedTextRun {
    pub fn scalar_len(&self) -> usize {
        if self.kind == RetainedRunKind::Break {
            1
        } else {
            self.text.chars().count()
        }
    }
}
impl ObjectContent {
    pub fn children(&self) -> Option<&[ObjectId]> {
        match self {
            Self::Group { children, .. }
            | Self::RetainedSource {
                native_kind: RetainedObjectKind::Group,
                children,
                ..
            } => Some(children),
            _ => None,
        }
    }
}

/// Range of directly editable PresentationML coordinates/extents. Shared by
/// snapshot admission and native leaf planning so commit cannot admit values
/// that are only discovered to be unwritable at export.
pub const PRESENTATIONML_COORDINATE_MIN: i64 = -27_273_042_329_600;
pub const PRESENTATIONML_COORDINATE_MAX: i64 = 27_273_042_316_900;
