//! Missing font selection is a resource prerequisite, not malformed text.
use super::FontStyle;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum FontSelectionReason {
    UnmappedTypeface,
    MissingStyle,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, thiserror::Error)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[error("font selection required for style {style}: {typeface} ({font_style:?}, {reason:?})")]
pub struct FontSelectionFailure {
    /// Index in the actual ManifestParagraphInput.styles, not a native run id.
    pub style: u32,
    pub typeface: String,
    pub font_style: FontStyle,
    pub reason: FontSelectionReason,
}
