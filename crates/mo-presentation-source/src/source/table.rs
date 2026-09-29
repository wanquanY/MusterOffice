//! DrawingML table declarations retain physical cells and attribute absence.
//! Cell text has one owner: SourceObject.paragraphs. Each cell names its range
//! and its text-style root in the surface catalog, including covered cells.
pub mod borders;
mod edges;
pub mod grid;
pub use edges::*;
mod read;
pub mod styles;
use super::{
    drawingml::NativeCoordinate,
    effects::SourceEffectProperties,
    fill::SourceFill,
    line::SourceLine,
    text::{NativeTextAnchor, NativeTextHorizontalOverflow, NativeTextVertical},
};
pub(super) use read::{Budget, Reader};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

pub(crate) const TABLE_URI: &str = "http://schemas.openxmlformats.org/drawingml/2006/table";

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, JsonSchema,
)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SourceCellAddress {
    pub row: u32,
    pub column: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SourceTable {
    pub source_ordinal: u32,
    pub properties: Option<SourceTableProperties>,
    pub grid_ordinal: u32,
    pub columns: Vec<SourceTableColumn>,
    pub rows: Vec<SourceTableRow>,
    pub retained_ordinals: Vec<u32>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SourceTableColumn {
    pub source_ordinal: u32,
    pub width: NativeCoordinate,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SourceTableRow {
    pub source_ordinal: u32,
    pub height: NativeCoordinate,
    pub cells: Vec<SourceTableCell>,
}
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SourceTableProperties {
    pub source_ordinal: u32,
    pub right_to_left: Option<bool>,
    pub first_row: Option<bool>,
    pub first_column: Option<bool>,
    pub last_row: Option<bool>,
    pub last_column: Option<bool>,
    pub band_rows: Option<bool>,
    pub band_columns: Option<bool>,
    pub fill: Option<SourceFill>,
    pub effects: Option<SourceEffectProperties>,
    pub style_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub inline_style: Option<Box<styles::SourceTableStyle>>,
}
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SourceTableCell {
    pub source_ordinal: u32,
    pub native_id: Option<String>,
    pub row_span: Option<i32>,
    pub grid_span: Option<i32>,
    pub horizontal_merge: Option<bool>,
    pub vertical_merge: Option<bool>,
    pub text_body_ordinal: Option<u32>,
    pub paragraph_start: u32,
    pub paragraph_count: u32,
    pub properties: Option<SourceTableCellProperties>,
}
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SourceTableCellProperties {
    pub source_ordinal: u32,
    pub margins: SourceTableMargins,
    pub vertical: Option<NativeTextVertical>,
    pub horizontal_overflow: Option<NativeTextHorizontalOverflow>,
    pub vertical_alignment: Option<NativeTextAnchor>,
    pub center_anchor: Option<bool>,
    pub fill: Option<SourceFill>,
    /// DrawingML order: left, right, top, bottom, TL-to-BR, BL-to-TR.
    pub borders: [Option<SourceLine>; 6],
}
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SourceTableMargins {
    pub left: Option<NativeCoordinate>,
    pub right: Option<NativeCoordinate>,
    pub top: Option<NativeCoordinate>,
    pub bottom: Option<NativeCoordinate>,
}
