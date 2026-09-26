use mo_common::Digest;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ShapeRequest {
    pub expected_sha256: Digest,
    pub face_index: u32,
    /// Logical text including context, with no implicit normalization.
    pub text: String,
    pub runs: Vec<ShapeRun>,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ShapeRun {
    /// Half-open Unicode scalar range in the request text (not UTF-16).
    pub start: u32,
    pub end: u32,
    pub direction: Direction,
    pub script: String,
    pub language: String,
    pub cluster_level: ClusterLevel,
    pub flags: ShapeFlags,
    pub features: Vec<ShapeFeature>,
    pub variations: Vec<ShapeVariation>,
    pub max_glyphs: u32,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum Direction {
    LeftToRight,
    RightToLeft,
    TopToBottom,
    BottomToTop,
}
impl Direction {
    pub(crate) fn word(self) -> u32 {
        match self {
            Self::LeftToRight => 4,
            Self::RightToLeft => 5,
            Self::TopToBottom => 6,
            Self::BottomToTop => 7,
        }
    }
    pub(crate) fn backward(self) -> bool {
        matches!(self, Self::RightToLeft | Self::BottomToTop)
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum ClusterLevel {
    MonotoneGraphemes,
    MonotoneCharacters,
    Characters,
    Graphemes,
}
impl ClusterLevel {
    pub(crate) fn word(self) -> u32 {
        match self {
            Self::MonotoneGraphemes => 0,
            Self::MonotoneCharacters => 1,
            Self::Characters => 2,
            Self::Graphemes => 3,
        }
    }
}
#[derive(Debug, Clone, Copy, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ShapeFlags {
    pub beginning_of_text: bool,
    pub end_of_text: bool,
    pub ignorables: Ignorables,
    pub suppress_dotted_circle: bool,
    pub unsafe_to_concat: bool,
    pub safe_to_insert_tatweel: bool,
}
#[derive(Debug, Clone, Copy, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum Ignorables {
    Default,
    Preserve,
    Remove,
}
impl ShapeFlags {
    pub(crate) fn word(self) -> u32 {
        u32::from(self.beginning_of_text)
            | (u32::from(self.end_of_text) << 1)
            | match self.ignorables {
                Ignorables::Default => 0,
                Ignorables::Preserve => 4,
                Ignorables::Remove => 8,
            }
            | (u32::from(self.suppress_dotted_circle) << 4)
            | (u32::from(self.unsafe_to_concat) << 6)
            | (u32::from(self.safe_to_insert_tatweel) << 7)
    }
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ShapeFeature {
    pub tag: String,
    pub value: u32,
    pub start: u32,
    /// None means through the end of the full logical text.
    pub end: Option<u32>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ShapeVariation {
    pub tag: String,
    /// Exact requested OpenType 16.16 design coordinate.
    pub value_16_16: i32,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct EffectiveVariation {
    pub tag: String,
    pub requested_16_16: i32,
    /// Effective IEEE754 binary32 design coordinate. Rounding is explicit.
    pub effective_f32_bits: u32,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ShapedText {
    pub font_sha256: Digest,
    pub face_index: u32,
    pub units_per_em: u16,
    pub position_units_per_em: u32,
    pub profile: String,
    pub runs: Vec<ShapedRun>,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ShapedRun {
    pub start: u32,
    pub end: u32,
    pub direction: Direction,
    pub effective_variations: Vec<EffectiveVariation>,
    /// A computational success may still contain missing glyphs; never hide it.
    pub missing_glyph_clusters: Vec<u32>,
    pub glyphs: Vec<ShapedGlyph>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ShapedGlyph {
    pub glyph_id: u32,
    pub cluster: u32,
    pub unsafe_to_break: bool,
    pub unsafe_to_concat: bool,
    pub safe_to_insert_tatweel: bool,
    pub x_advance: i32,
    pub y_advance: i32,
    pub x_offset: i32,
    pub y_offset: i32,
}
#[derive(Debug, Clone, Copy)]
pub struct TextLimits {
    pub max_scalars: usize,
    pub max_runs: usize,
    pub max_context_scalars: usize,
    pub max_glyphs: usize,
}
impl Default for TextLimits {
    fn default() -> Self {
        Self {
            max_scalars: 65536,
            max_runs: 256,
            max_context_scalars: 1048576,
            max_glyphs: 262144,
        }
    }
}
