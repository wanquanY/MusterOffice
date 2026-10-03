use crate::{
    EffectiveVariation,
    flow::{ParagraphLayoutRequest, ParagraphLayoutResult},
    geometry::FragmentRef,
};
use mo_common::{Digest, Emu};
use mo_geometry::{Fixed, PathCommand, Point, Rect};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ParagraphPathsRequest {
    pub layout: ParagraphLayoutRequest,
    /// Raw Q32 EMU, minimum 256 and maximum 2^32 (1 EMU).
    pub bounds_tolerance: Fixed,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ParagraphPathsResult {
    pub layout: ParagraphLayoutResult,
    pub scene: Option<ParagraphPathScene>,
    pub issues: Vec<PathSceneIssue>,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ParagraphGeometryPaths {
    pub paths: ParagraphPathsResult,
    pub precise: Option<crate::geometry::PreciseGeometryLayout>,
}
/// One layout evaluation owns both rendering and editing coordinates. The map
/// is computed data, not a deserializable replacement supplied by the host.
#[derive(Debug, Clone, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ParagraphEditorGeometry {
    pub geometry: ParagraphGeometryPaths,
    pub interaction: Option<crate::interaction::InteractionMap>,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "camelCase", deny_unknown_fields)]
pub enum PathSceneIssue {
    ColorRepresentationRequired {
        font: u32,
        variations: Vec<EffectiveVariation>,
    },
    OutlineUnavailable {
        font: u32,
        variations: Vec<EffectiveVariation>,
        glyph_id: u32,
    },
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SceneFont {
    pub font_sha256: Digest,
    pub face_index: u32,
    pub effective_variations: Vec<EffectiveVariation>,
    pub position_units_per_em: u32,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct GlyphPath {
    pub font: u32,
    pub glyph_id: u32,
    pub font_size: Emu,
    /// Local Q32 EMU, y down, unhinted, nonzero fill; no color/stroke/effects.
    pub commands: Vec<PathCommand>,
    /// Geometric curve bounds, not filled/stroked/effect ink bounds.
    pub bounds: Option<Rect>,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct GlyphDraw {
    pub path: u32,
    pub origin: Point,
    pub line: u32,
    pub source: FragmentRef,
    pub glyph: u32,
}
#[derive(Debug, Clone, Default, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PathSceneWork {
    pub outline_calls: u32,
    pub unique_source_glyphs: u32,
    pub path_commands: u32,
    pub bounds_nodes: u32,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ParagraphPathScene {
    pub profile: String,
    pub bounds_tolerance: Fixed,
    pub fonts: Vec<SceneFont>,
    pub paths: Vec<GlyphPath>,
    pub glyphs: Vec<GlyphDraw>,
    pub bounds: Option<Rect>,
    pub work: PathSceneWork,
}
