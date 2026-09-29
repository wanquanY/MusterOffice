//! A native table retains every physical grid cell, including covered cells.
//! Merging changes ownership of the visible rectangle, never deletes text or IDs.
mod grid;
use crate::{Fill, Inherited, Stroke, TextBody};
pub use grid::{TableGrid, TableGridError, TableRectangle};
use mo_common::{CellId, ColumnId, Emu, RowId};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Table {
    pub columns: Vec<TableColumn>,
    pub rows: Vec<TableRow>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TableColumn {
    pub id: ColumnId,
    pub width: Emu,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TableRow {
    pub id: RowId,
    pub height: Emu,
    /// One entry per grid column, even when covered by another cell.
    pub cells: Vec<TableCell>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TableCell {
    /// Row/column/cell identities are table-scoped. Text identities remain
    /// document-scoped so existing anchors retain their unambiguous meaning.
    pub id: CellId,
    #[serde(default)]
    pub merge: TableCellMerge,
    pub text: Option<TextBody>,
    #[serde(default)]
    pub style: TableCellStyle,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "camelCase", deny_unknown_fields)]
pub enum TableCellMerge {
    /// The origin is the upper-left cell. Counts include the origin.
    Span {
        rows: u32,
        columns: u32,
    },
    Covered {
        origin: CellId,
    },
}
impl Default for TableCellMerge {
    fn default() -> Self {
        Self::Span {
            rows: 1,
            columns: 1,
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(default, rename_all = "camelCase", deny_unknown_fields)]
pub struct TableCellStyle {
    pub fill: Inherited<Fill>,
    pub borders: TableCellBorders,
    pub vertical_alignment: Inherited<TableVerticalAlignment>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(default, rename_all = "camelCase", deny_unknown_fields)]
pub struct TableCellBorders {
    pub left: Inherited<Stroke>,
    pub right: Inherited<Stroke>,
    pub top: Inherited<Stroke>,
    pub bottom: Inherited<Stroke>,
    pub top_left_to_bottom_right: Inherited<Stroke>,
    pub bottom_left_to_top_right: Inherited<Stroke>,
}
impl TableCellBorders {
    pub fn edges(&self) -> [(&'static str, &Inherited<Stroke>); 6] {
        [
            ("left", &self.left),
            ("right", &self.right),
            ("top", &self.top),
            ("bottom", &self.bottom),
            ("topLeftToBottomRight", &self.top_left_to_bottom_right),
            ("bottomLeftToTopRight", &self.bottom_left_to_top_right),
        ]
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum TableVerticalAlignment {
    Top,
    Center,
    Bottom,
    Justified,
    Distributed,
}
