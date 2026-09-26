use crate::{
    ShapedText,
    cascade::{CascadeLimits, FontAttempt, VariationIssue},
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "status", rename_all = "camelCase", deny_unknown_fields)]
pub enum FontFragment {
    Selected {
        start: u32,
        end: u32,
        font: u32,
        candidate: u32,
        shaped: ShapedText,
    },
    Unresolved {
        start: u32,
        end: u32,
    },
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ReshapeRejection {
    pub start: u32,
    pub end: u32,
    pub candidate: u32,
    pub missing_glyph_clusters: Vec<u32>,
    pub variation_issues: Vec<VariationIssue>,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FallbackItem {
    pub start: u32,
    pub end: u32,
    /// Whole-item probe evidence, in the caller's candidate order.
    pub probes: Vec<FontAttempt>,
    /// Grapheme boundaries disallowed by observed clusters/unsafe-to-break.
    pub protected_boundaries: Vec<u32>,
    pub reshape_rejections: Vec<ReshapeRejection>,
    /// Exhaustive, disjoint, logical ranges; not visual painting order.
    pub fragments: Vec<FontFragment>,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FallbackResult {
    pub profile: String,
    pub items: Vec<FallbackItem>,
    pub verified_faces: u32,
    pub shaping_runs: u32,
    pub component_calls: u32,
    pub context_scalars: u32,
    pub probed_glyphs: u32,
}
#[derive(Debug, Clone, Copy)]
pub struct FallbackLimits {
    pub cascade: CascadeLimits,
    pub max_fragments: usize,
    pub max_reshape_rejections: usize,
}
impl Default for FallbackLimits {
    fn default() -> Self {
        Self {
            cascade: CascadeLimits::default(),
            max_fragments: 1024,
            max_reshape_rejections: 1024,
        }
    }
}
