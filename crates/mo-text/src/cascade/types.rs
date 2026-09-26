use crate::*;
use mo_common::{ByteLength, Digest};
use mo_font::CoverageOutcome;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Explicit resource bundle bindings, not system font names or legal permissions.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CascadeFont {
    pub expected_sha256: Digest,
    pub face_index: u32,
    pub offset: ByteLength,
    pub byte_length: ByteLength,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FontCandidate {
    pub font: u32,
    pub variations: Vec<ShapeVariation>,
}
/// One indivisible shaping item. The caller supplies script/direction/language
/// itemization; this layer never invents script or splits a shaping dependency.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CascadeItem {
    pub start: u32,
    pub end: u32,
    pub direction: Direction,
    pub script: String,
    pub language: String,
    pub features: Vec<ShapeFeature>,
    pub beginning_of_text: bool,
    pub end_of_text: bool,
    pub suppress_dotted_circle: bool,
    pub max_glyphs: u32,
    pub candidates: Vec<FontCandidate>,
}
impl CascadeItem {
    pub(crate) fn run(&self, candidate: &FontCandidate) -> ShapeRun {
        ShapeRun {
            start: self.start,
            end: self.end,
            direction: self.direction,
            script: self.script.clone(),
            language: self.language.clone(),
            cluster_level: ClusterLevel::MonotoneGraphemes,
            flags: ShapeFlags {
                beginning_of_text: self.beginning_of_text,
                end_of_text: self.end_of_text,
                ignorables: Ignorables::Default,
                suppress_dotted_circle: self.suppress_dotted_circle,
                unsafe_to_concat: true,
                safe_to_insert_tatweel: false,
            },
            features: self.features.clone(),
            variations: candidate.variations.clone(),
            max_glyphs: self.max_glyphs,
        }
    }
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CascadeRequest {
    pub text: String,
    pub fonts: Vec<CascadeFont>,
    pub items: Vec<CascadeItem>,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct VariationIssue {
    pub selector_offset: u32,
    /// None for a selector without an immediately preceding non-selector scalar.
    pub base_offset: Option<u32>,
    pub outcome: CoverageOutcome,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FontAttempt {
    pub candidate: u32,
    pub font: u32,
    pub missing_glyph_clusters: Vec<u32>,
    /// Conservative cmap-14 evidence, not proof of GSUB-only selector handling.
    pub variation_issues: Vec<VariationIssue>,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "status", rename_all = "camelCase", deny_unknown_fields)]
pub enum ItemSelection {
    Selected {
        font: u32,
        candidate: u32,
        shaped: ShapedText,
        attempts: Vec<FontAttempt>,
    },
    Unresolved {
        start: u32,
        end: u32,
        attempts: Vec<FontAttempt>,
    },
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CascadeResult {
    pub profile: String,
    pub items: Vec<ItemSelection>,
    pub verified_faces: u32,
    pub shaping_calls: u32,
    pub context_scalars: u32,
    pub probed_glyphs: u32,
}
#[derive(Debug, Clone, Copy)]
pub struct CascadeLimits {
    pub max_bundle_bytes: usize,
    pub max_font_bindings: usize,
    pub max_items: usize,
    pub max_candidates_per_item: usize,
    pub max_attempts: usize,
    pub max_context_scalars: usize,
    pub max_probed_glyphs: usize,
    pub max_selected_glyphs: usize,
}
impl Default for CascadeLimits {
    fn default() -> Self {
        Self {
            max_bundle_bytes: 128 * 1024 * 1024,
            max_font_bindings: 32,
            max_items: 256,
            max_candidates_per_item: 32,
            max_attempts: 1024,
            max_context_scalars: 1048576,
            max_probed_glyphs: 1048576,
            max_selected_glyphs: 262144,
        }
    }
}
