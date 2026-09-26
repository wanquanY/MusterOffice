use mo_unicode::{
    TextBoundary,
    bidi::{BidiParagraphResult, ParagraphDirection},
    script::Script,
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct StyleSpan {
    pub end: u32,
    pub style: u32,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ItemizationRequest {
    pub text: String,
    pub direction: ParagraphDirection,
    /// Contiguous, exhaustive, grapheme-aligned style spans. Style identities
    /// must already represent effective shaping properties, not author run IDs.
    pub spans: Vec<StyleSpan>,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum TextItemKind {
    Text,
    Tab,
    LineBreak,
    ParagraphBreak,
    BidiControl,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TextItem {
    pub start: TextBoundary,
    pub end: TextBoundary,
    pub style: u32,
    pub script: Script,
    /// Before line-specific L1 resets; not a final glyph painting order.
    pub level: u8,
    pub kind: TextItemKind,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum ItemizationNoticeKind {
    MixedScriptCluster,
    MixedLevelCluster,
    AmbiguousScript,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ItemizationNotice {
    pub start: u32,
    pub end: u32,
    pub kind: ItemizationNoticeKind,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ItemizationResult {
    pub profile: String,
    pub bidi: BidiParagraphResult,
    pub items: Vec<TextItem>,
    pub notices: Vec<ItemizationNotice>,
}
#[derive(Debug, Clone, Copy)]
pub struct ItemizationLimits {
    pub max_styles: usize,
    pub max_items: usize,
    pub max_notices: usize,
}
impl Default for ItemizationLimits {
    fn default() -> Self {
        Self {
            max_styles: 256,
            max_items: 4096,
            max_notices: 65536,
        }
    }
}
