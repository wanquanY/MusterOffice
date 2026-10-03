use crate::{Direction, EffectiveVariation, ShapeVariation};
use mo_common::Digest;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FontCaretsRequest {
    pub expected_sha256: Digest,
    pub face_index: u32,
    pub instances: Vec<FontCaretsInstance>,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FontCaretsInstance {
    pub variations: Vec<ShapeVariation>,
    pub direction: Direction,
    pub glyph_ids: Vec<u32>,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FontCaretsResult {
    pub font_sha256: Digest,
    pub face_index: u32,
    pub units_per_em: u16,
    pub position_units_per_em: u32,
    pub profile: String,
    pub instances: Vec<MeasuredCarets>,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MeasuredCarets {
    pub effective_variations: Vec<EffectiveVariation>,
    pub direction: Direction,
    pub glyphs: Vec<GlyphCarets>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct GlyphCarets {
    pub glyph_id: u32,
    /// GDEF positions in the font's order, before shaping placement/kerning.
    /// Empty means the font supplies no carets; zero is a real position.
    pub positions: Vec<i32>,
}
