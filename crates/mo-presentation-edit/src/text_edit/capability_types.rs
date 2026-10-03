use super::*;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub enum TextEditRestriction {
    UnsupportedTarget,
    CoveredCell,
    MissingTextBody,
    TextBodyExists,
    SelectionRequired,
    NonemptySelectionRequired,
    NativeStructureRequired,
    RetainedParagraphBoundary,
    NativeInsertionTarget,
    NativeRun {
        run: RunId,
        constraint: NativeEditConstraint,
    },
    StructuredRun {
        run: RunId,
        run_kind: RetainedRunKind,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub enum TextEditAvailability {
    Available,
    Unavailable { reason: TextEditRestriction },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum TextReplacementPolicy {
    /// Paragraph breaks and typed inline tabs, with authored style inheritance.
    AuthoredBody,
    /// Text leaves within one existing paragraph, with original run identities.
    /// Inserted CR/LF/paragraph or line separators require native structure work.
    RetainedTextLeaves,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TextCapabilitiesQuery {
    pub object: ObjectId,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cell: Option<CellId>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub selection: Option<TextSelection>,
}

/// Model/selection prerequisites only, not host authorization, font availability
/// or a promise that arbitrary replacement text passes document limits.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TextEditingCapabilities {
    pub document_id: DocumentId,
    pub revision: Digest,
    pub object: ObjectId,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cell: Option<CellId>,
    pub initialize: TextEditAvailability,
    pub replace: TextEditAvailability,
    /// Deletion has no insertion-style owner; it can be legal beside a protected run.
    pub delete: TextEditAvailability,
    pub character_style: TextEditAvailability,
    pub replacement_policy: Option<TextReplacementPolicy>,
}
