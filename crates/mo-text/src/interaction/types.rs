use crate::flow::{ParagraphLayoutRequest, ParagraphLayoutResult};
use mo_geometry::{Fixed, Point, Rect};
use mo_unicode::TextBoundary;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum Affinity {
    Upstream,
    Downstream,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TextPosition {
    pub scalar_offset: u32,
    pub affinity: Affinity,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CaretEdge {
    pub x: Fixed,
    pub top: Fixed,
    pub bottom: Fixed,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum PartitionReason {
    FontCaretsAbsent,
    AmbiguousGlyphs,
    FontCaretCountMismatch,
    NonMonotoneFontCarets,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "camelCase", deny_unknown_fields)]
pub enum CaretPlacement {
    GlyphEdges,
    TabEdges,
    FontLigature,
    ClusterPartition { reason: PartitionReason },
    Invisible,
}
#[derive(Debug, Clone, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct InteractionCell {
    pub start: TextBoundary,
    pub end: TextBoundary,
    pub line: u32,
    pub level: u8,
    pub kind: crate::itemize::TextItemKind,
    pub leading: CaretEdge,
    pub trailing: CaretEdge,
    pub placement: CaretPlacement,
}
#[derive(Debug, Clone, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct InteractionLine {
    pub start: TextBoundary,
    pub end: TextBoundary,
    pub top: Fixed,
    pub bottom: Fixed,
    pub empty_caret: CaretEdge,
    /// Indices into the paragraph's logical-order cells.
    pub cells: Vec<u32>,
    /// Renderer L2 order collapsed to EGCs; removed controls have no visual slot.
    #[serde(skip)]
    #[schemars(skip)]
    pub(crate) visual_cells: Vec<u32>,
}
/// Computed, immutable data. Public queries accept this trusted Rust value,
/// never an unvalidated serialized map supplied by an external caller.
#[derive(Debug, Clone, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct InteractionMap {
    pub profile: String,
    pub paragraph_level: u8,
    pub boundaries: Vec<TextBoundary>,
    pub lines: Vec<InteractionLine>,
    pub cells: Vec<InteractionCell>,
    pub work: InteractionWork,
}
#[derive(Debug, Clone, Default, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct InteractionWork {
    pub caret_calls: u32,
    pub unique_glyphs: u32,
    pub font_instances: u32,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub enum TextQuery {
    Caret {
        position: TextPosition,
    },
    Hit {
        point: Point,
    },
    Selection {
        anchor: TextPosition,
        focus: TextPosition,
    },
    Move {
        position: TextPosition,
        movement: CaretMove,
        /// Paragraph-local sticky x for consecutive vertical moves.
        #[serde(default)]
        preferred_x: Option<Fixed>,
    },
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum CaretMove {
    PreviousGrapheme,
    NextGrapheme,
    Left,
    Right,
    Up,
    Down,
    LineStart,
    LineEnd,
    TextStart,
    TextEnd,
}
#[derive(Debug, Clone, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct CaretNavigation {
    pub caret: ResolvedCaret,
    pub preferred_x: Option<Fixed>,
    /// Directional movement reached the paragraph edge; containers may continue.
    pub exhausted: bool,
}
#[derive(Debug, Clone, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ResolvedCaret {
    pub position: TextPosition,
    pub boundary: TextBoundary,
    pub line: u32,
    pub edge: CaretEdge,
}
#[derive(Debug, Clone, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct SelectionFragment {
    pub line: u32,
    pub start: TextBoundary,
    pub end: TextBoundary,
    pub kind: crate::itemize::TextItemKind,
    pub bounds: Rect,
}
#[derive(Debug, Clone, Serialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum TextQueryResult {
    Caret {
        caret: ResolvedCaret,
    },
    Hit {
        caret: ResolvedCaret,
        inside: bool,
    },
    Moved {
        result: CaretNavigation,
    },
    Selection {
        anchor: ResolvedCaret,
        focus: ResolvedCaret,
        fragments: Vec<SelectionFragment>,
    },
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ParagraphInteractionRequest {
    pub layout: ParagraphLayoutRequest,
    pub queries: Vec<TextQuery>,
}
#[derive(Debug, Clone, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ParagraphInteractionResult {
    pub layout: ParagraphLayoutResult,
    /// Absent when existing layout prerequisites are unresolved; no partial map.
    pub map: Option<InteractionMap>,
    pub results: Vec<TextQueryResult>,
}
