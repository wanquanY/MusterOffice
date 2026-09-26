use crate::{EffectiveVariation, ShapeVariation};
use mo_common::Digest;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FontOutlinesRequest {
    pub expected_sha256: Digest,
    pub face_index: u32,
    pub instances: Vec<OutlineInstance>,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct OutlineInstance {
    pub variations: Vec<ShapeVariation>,
    pub glyph_ids: Vec<u32>,
    /// Aggregate across all glyphs of this instance, including discarded prefixes.
    pub max_commands: u32,
    /// HarfBuzz's weighted interpreter budget, not CPU instructions or time.
    pub max_operations: u32,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FontOutlinesResult {
    pub font_sha256: Digest,
    pub face_index: u32,
    pub units_per_em: u16,
    pub position_units_per_em: u32,
    pub profile: String,
    /// Presence only: does not imply that a particular glyph uses these tables.
    pub color_tables: Vec<String>,
    pub instances: Vec<OutlinedInstance>,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct OutlinedInstance {
    pub effective_variations: Vec<EffectiveVariation>,
    pub glyphs: Vec<GlyphOutline>,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct GlyphOutline {
    pub glyph_id: u32,
    /// None is unavailable/failed, Some([]) is a valid empty outline (e.g. space).
    pub path: Option<Vec<OutlineCommand>>,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct OutlinePoint {
    pub x: i32,
    pub y: i32,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "camelCase", deny_unknown_fields)]
pub enum OutlineCommand {
    Move {
        to: OutlinePoint,
    },
    Line {
        to: OutlinePoint,
    },
    Quadratic {
        control: OutlinePoint,
        to: OutlinePoint,
    },
    Cubic {
        control1: OutlinePoint,
        control2: OutlinePoint,
        to: OutlinePoint,
    },
    Close,
}
