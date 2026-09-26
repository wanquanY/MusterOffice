use crate::TextBoundary;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use thiserror::Error;
#[derive(Debug, Clone, Copy, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum ParagraphDirection {
    AutoLeftToRight,
    LeftToRight,
    RightToLeft,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct BidiParagraphRequest {
    pub text: String,
    pub direction: ParagraphDirection,
    /// Unicode scalar line ends supplied by layout, increasing through text end.
    /// Empty means one complete line, not automatic width-based wrapping.
    pub line_ends: Vec<u32>,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct BidiParagraphResult {
    pub profile: String,
    pub paragraph_level: u8,
    /// I1/I2 result, before line-specific L1 resets. None means removed by X9.
    pub resolved_levels: Vec<Option<u8>>,
    pub lines: Vec<BidiLine>,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct BidiLine {
    pub start: TextBoundary,
    pub end: TextBoundary,
    /// L1-adjusted levels, one per scalar in this line. X9 entries remain None.
    pub levels: Vec<Option<u8>>,
    /// L2 visual-to-logical scalar indices relative to the full paragraph.
    /// X9 entries are omitted. This is not a glyph/cluster painting order (L3/L4).
    pub visual_order: Vec<u32>,
}
#[derive(Debug, Clone, Copy)]
pub struct BidiLimits {
    pub max_scalars: usize,
    pub max_bytes: usize,
    pub max_lines: usize,
}
impl Default for BidiLimits {
    fn default() -> Self {
        Self {
            max_scalars: 65536,
            max_bytes: 262144,
            max_lines: 65536,
        }
    }
}
#[derive(Debug, Error)]
pub enum BidiError {
    #[error("invalid bidi request: {0}")]
    Invalid(&'static str),
    #[error("bidi budget exceeded: {0}")]
    Limit(&'static str),
    #[error("bidi analysis cancelled")]
    Cancelled,
}
