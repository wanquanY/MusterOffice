use crate::{CharacterStyle, ParagraphStyle};
use mo_common::{Emu, ParagraphId, RunId};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TextBody {
    pub paragraphs: Vec<Paragraph>,
    pub style: CharacterStyle,
    pub insets: Insets,
    pub wrap: bool,
    pub overflow: OverflowPolicy,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Insets {
    pub left: Emu,
    pub top: Emu,
    pub right: Emu,
    pub bottom: Emu,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum OverflowPolicy {
    Report,
    Clip,
    GrowShape,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Paragraph {
    pub id: ParagraphId,
    pub style: ParagraphStyle,
    pub default_run_style: CharacterStyle,
    pub runs: Vec<TextRun>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TextRun {
    pub id: RunId,
    pub style: CharacterStyle,
    pub content: InlineContent,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "camelCase", deny_unknown_fields)]
pub enum InlineContent {
    Text { text: String },
    Break,
    Tab,
}

impl InlineContent {
    pub fn scalar_len(&self) -> usize {
        match self {
            Self::Text { text } => text.chars().count(),
            Self::Break | Self::Tab => 1,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum Affinity {
    Before,
    After,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TextAnchor {
    pub paragraph: ParagraphId,
    pub scalar_offset: u32,
    pub affinity: Affinity,
}
