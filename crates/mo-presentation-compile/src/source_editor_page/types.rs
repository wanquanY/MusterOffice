use crate::source_frame::interaction::{FrameCaret, FrameSelectionFragment, FrameTextPosition};
use mo_geometry::{Fixed, Point};
use mo_text::interaction::CaretMove;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub enum PageTextAction {
    Move {
        position: FrameTextPosition,
        movement: CaretMove,
        /// Sticky inline coordinate in the local text frame, before group transforms.
        #[serde(default)]
        preferred_x: Option<Fixed>,
    },
    Caret {
        position: FrameTextPosition,
    },
    /// Q32 page EMU, before the device viewport transform.
    Hit {
        point: Point,
    },
    Selection {
        anchor: FrameTextPosition,
        focus: FrameTextPosition,
    },
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PageTextQuery {
    /// Index in this immutable page's text binding list; includes table cells.
    pub frame: u32,
    pub action: PageTextAction,
}
#[derive(Debug, Clone, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct PageCaret {
    pub local: FrameCaret,
    pub edge: [Point; 2],
    pub visible: Option<[Point; 2]>,
    /// Outward Q32 page EMU error from the same native placement certification.
    pub coordinate_error_bound: Fixed,
}
#[derive(Debug, Clone, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct PageSelectionFragment {
    pub local: FrameSelectionFragment,
    /// Four actual corners, not an axis-aligned envelope of rotated text.
    pub quad: [Point; 4],
    pub visible: Option<[Point; 4]>,
    pub coordinate_error_bound: Fixed,
}
#[derive(Debug, Clone, Serialize, JsonSchema)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum PageTextQueryResult {
    Moved {
        frame: u32,
        caret: Box<PageCaret>,
        preferred_x: Option<Fixed>,
        exhausted: bool,
    },
    Caret {
        frame: u32,
        caret: Box<PageCaret>,
    },
    /// None means the frame's sampled transform is exactly singular.
    Hit {
        frame: u32,
        caret: Option<Box<PageCaret>>,
        inside: bool,
    },
    Selection {
        frame: u32,
        anchor: Box<PageCaret>,
        focus: Box<PageCaret>,
        fragments: Vec<PageSelectionFragment>,
        paragraph_breaks: Vec<u32>,
    },
}
