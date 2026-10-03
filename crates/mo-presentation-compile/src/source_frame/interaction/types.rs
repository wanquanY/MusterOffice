use mo_geometry::{Point, Rect};
use mo_text::interaction::{CaretEdge, ResolvedCaret, TextPosition};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
#[derive(Debug, Clone, Copy)]
pub struct FrameInteractionLimits {
    pub max_cells: usize,
    pub max_lines: usize,
    pub max_query_work: usize,
    pub max_selection_fragments: usize,
}
impl Default for FrameInteractionLimits {
    fn default() -> Self {
        Self {
            max_cells: 262144,
            max_lines: 16384,
            max_query_work: 1048576,
            max_selection_fragments: 65536,
        }
    }
}
#[derive(Debug, Clone, Copy, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FrameTextPosition {
    /// Index in this frame's native paragraphs, not an XML run index.
    pub paragraph: u32,
    pub position: TextPosition,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "camelCase", deny_unknown_fields)]
pub enum FrameTextQuery {
    Caret {
        position: FrameTextPosition,
    },
    Hit {
        point: Point,
    },
    Selection {
        anchor: FrameTextPosition,
        focus: FrameTextPosition,
    },
}
#[derive(Debug, Clone, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct FrameCaret {
    pub paragraph: u32,
    /// Includes the actual frame line translation (insets, paragraph alignment,
    /// native indentation, vertical anchor and paragraph spacing).
    pub caret: ResolvedCaret,
    pub visible: Option<CaretEdge>,
}
#[derive(Debug, Clone, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct FrameSelectionFragment {
    pub paragraph: u32,
    pub fragment: mo_text::interaction::SelectionFragment,
    /// Native overflow clips the overlay along active local axes only.
    pub visible: Option<Rect>,
}
#[derive(Debug, Clone, Serialize, JsonSchema)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum FrameTextQueryResult {
    Caret {
        caret: FrameCaret,
    },
    Hit {
        caret: FrameCaret,
        inside: bool,
    },
    Selection {
        anchor: Box<FrameCaret>,
        focus: Box<FrameCaret>,
        fragments: Vec<FrameSelectionFragment>,
        /// Selected structural separators after these paragraph indices.
        /// They do not invent scalar positions or positive-width glyph boxes.
        paragraph_breaks: Vec<u32>,
    },
}
