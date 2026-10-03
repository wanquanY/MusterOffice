use super::*;
use serde::{Deserialize, Serialize};
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GlyphClusterPaint {
    pub paragraph: u32,
    pub start: u32,
    pub end: u32,
    pub runs: Vec<u32>,
    pub rgba: Option<[u8; 4]>,
    pub underline: bool,
    pub underline_rgba: Option<[u8; 4]>,
    pub strike: bool,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum DecorationKind {
    Underline,
    Strike,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TextDecoration {
    pub paragraph: u32,
    pub line: u32,
    pub kind: DecorationKind,
    pub clusters: Vec<u32>,
    pub rect: mo_geometry::Rect,
    pub rgba: [u8; 4],
}
#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TextDecorationIssue {
    pub paragraph: u32,
    pub run: u32,
    pub source_ordinal: u32,
    pub font: u32,
    pub metric: mo_text::metrics::FontMetric,
    /// None means unavailable; nonpositive thickness is an unusable metric.
    pub value: Option<i32>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TextPageBinding {
    /// Object entry in the shared page plan; geometry and text paint at the same
    /// position in source order, text following this object's shape paths.
    pub binding: u32,
    /// Local envelope of actually painted, nontransparent glyph outlines.
    pub painted_ink: Option<mo_geometry::Rect>,
    pub page_ink: crate::source_frame::capacity::PageTextInk,
    pub frame: SourceFramePlan,
    pub paints: Vec<Vec<TextRunPaint>>,
    pub clusters: Vec<GlyphClusterPaint>,
    pub decorations: Vec<TextDecoration>,
    pub local_coordinate_error_bound: Fixed,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TextPagePaintSource {
    pub instance: u32,
    pub text_binding: u32,
    pub glyph: u32,
    pub cluster: u32,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TextDecorationSource {
    pub instance: u32,
    pub text_binding: u32,
    pub decoration: u32,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SourceTextPagePlan {
    pub profile: String,
    /// Shared geometry/raster plan. Its profile selects the existing shape
    /// policy; the enclosing profile additionally certifies the text operation.
    pub page: SourcePagePlan,
    pub texts: Vec<TextPageBinding>,
    pub text_sources: Vec<TextPagePaintSource>,
    pub decoration_sources: Vec<TextDecorationSource>,
    pub text_work: FrameWork,
}
pub struct SourceTextPageImage {
    pub info: SourceTextPageRasterInfo,
    pub pixels: Vec<u8>,
}
#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SourceTextPageRasterInfo {
    pub profile: String,
    pub page: SourcePageRasterInfo,
    pub text_frames: u32,
    pub text_work: FrameWork,
    /// Missing in historical responses; absence never means measured to fit.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text_capacity: Option<crate::source_frame::capacity::TextCapacity>,
}
#[derive(Debug, Clone, Copy)]
pub struct TextPageLimits {
    pub max_frames: usize,
    pub max_paragraphs: usize,
    pub max_runs: usize,
    /// Conservative source computation-plan accounting, not total heap/RSS.
    /// Source catalogs, cascade and the in-flight frame have separate bounds.
    pub max_prepared_plan_bytes: usize,
    /// Aggregate physical table topology and coordinate declarations per page.
    /// Covered cells are charged even though only merge origins paint text.
    pub tables: crate::source_table::TableGeometryLimits,
    /// Work allowances are shared by all frames, never reset per object.
    pub work: SourceFrameLimits,
}
impl Default for TextPageLimits {
    fn default() -> Self {
        Self {
            max_frames: 128,
            max_paragraphs: 512,
            max_runs: 4096,
            max_prepared_plan_bytes: 64 * 1024 * 1024,
            tables: crate::source_table::TableGeometryLimits::default(),
            work: SourceFrameLimits::default(),
        }
    }
}

/// Native text metadata without a second copy of the shared page geometry.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TextPageContent {
    pub texts: Vec<TextPageBinding>,
    pub text_sources: Vec<TextPagePaintSource>,
    pub decoration_sources: Vec<TextDecorationSource>,
    pub text_work: FrameWork,
}

pub(crate) fn capacity(
    texts: &[TextPageBinding],
    check: &dyn Fn() -> bool,
) -> Result<crate::source_frame::capacity::TextCapacity, SourcePageError> {
    use crate::source_frame::capacity;
    let frames = texts
        .iter()
        .map(|text| capacity::measure(&text.frame, check).map_err(SourcePageError::from))
        .collect::<Result<Vec<_>, _>>()?;
    Ok(capacity::TextCapacity {
        profile: capacity::PROFILE.into(),
        frames,
        page_ink: Some(texts.iter().map(|text| text.page_ink.clone()).collect()),
    })
}

/// Private compiler-owned maps in exactly the text binding order of this page.
pub(crate) struct TextPageInteraction {
    pub maps: Vec<Vec<mo_text::interaction::InteractionMap>>,
    pub limits: crate::source_frame::interaction::FrameInteractionLimits,
}
