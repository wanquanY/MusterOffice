//! Native physical border identities. Direct cell and table-style diagonals
//! keep their original names/directions rather than being flattened to an index.
use super::*;

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, JsonSchema,
)]
#[serde(rename_all = "camelCase")]
pub enum TableCellEdge {
    Left,
    Right,
    Top,
    Bottom,
    TopLeftToBottomRight,
    BottomLeftToTopRight,
}
impl TableCellEdge {
    pub const ALL: [Self; 6] = [
        Self::Left,
        Self::Right,
        Self::Top,
        Self::Bottom,
        Self::TopLeftToBottomRight,
        Self::BottomLeftToTopRight,
    ];
    pub fn index(self) -> usize {
        match self {
            Self::Left => 0,
            Self::Right => 1,
            Self::Top => 2,
            Self::Bottom => 3,
            Self::TopLeftToBottomRight => 4,
            Self::BottomLeftToTopRight => 5,
        }
    }
    /// Physical neighbour across an orthogonal edge. Column declarations are
    /// ordered before RTL placement; left/right themselves are visual sides.
    pub fn neighbour(
        self,
        at: SourceCellAddress,
        table: &SourceTable,
    ) -> Option<SourceCellAddress> {
        let rtl = table
            .properties
            .as_ref()
            .is_some_and(|p| p.right_to_left == Some(true));
        let delta = match self {
            Self::Left => (0, if rtl { 1 } else { -1 }),
            Self::Right => (0, if rtl { -1 } else { 1 }),
            Self::Top => (-1, 0),
            Self::Bottom => (1, 0),
            Self::TopLeftToBottomRight | Self::BottomLeftToTopRight => return None,
        };
        let row = at.row.checked_add_signed(delta.0)?;
        let column = at.column.checked_add_signed(delta.1)?;
        ((row as usize) < table.rows.len() && (column as usize) < table.columns.len())
            .then_some(SourceCellAddress { row, column })
    }
}

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, JsonSchema,
)]
#[serde(rename_all = "camelCase")]
pub enum TableStyleEdge {
    Left,
    Right,
    Top,
    Bottom,
    InsideHorizontal,
    InsideVertical,
    TopLeftToBottomRight,
    TopRightToBottomLeft,
}
impl TableStyleEdge {
    pub const ALL: [Self; 8] = [
        Self::Left,
        Self::Right,
        Self::Top,
        Self::Bottom,
        Self::InsideHorizontal,
        Self::InsideVertical,
        Self::TopLeftToBottomRight,
        Self::TopRightToBottomLeft,
    ];
    pub fn index(self) -> usize {
        match self {
            Self::Left => 0,
            Self::Right => 1,
            Self::Top => 2,
            Self::Bottom => 3,
            Self::InsideHorizontal => 4,
            Self::InsideVertical => 5,
            Self::TopLeftToBottomRight => 6,
            Self::TopRightToBottomLeft => 7,
        }
    }
    pub(in crate::source::table) fn for_cell(edge: TableCellEdge, inside: bool) -> Self {
        match edge {
            TableCellEdge::Left if inside => Self::InsideVertical,
            TableCellEdge::Right if inside => Self::InsideVertical,
            TableCellEdge::Top if inside => Self::InsideHorizontal,
            TableCellEdge::Bottom if inside => Self::InsideHorizontal,
            TableCellEdge::Left => Self::Left,
            TableCellEdge::Right => Self::Right,
            TableCellEdge::Top => Self::Top,
            TableCellEdge::Bottom => Self::Bottom,
            TableCellEdge::TopLeftToBottomRight => Self::TopLeftToBottomRight,
            TableCellEdge::BottomLeftToTopRight => Self::TopRightToBottomLeft,
        }
    }
}
