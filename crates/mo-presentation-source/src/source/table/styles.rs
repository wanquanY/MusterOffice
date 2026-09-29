//! Native table style declarations. Fonts, fills, lines and effects share the
//! same typed DrawingML values as the rest of the source model.
mod part;
mod read;
mod select;
use crate::source::{
    SourceCompatibility, drawingml::*, effects::*, fill::*, line::*,
    text::NativeFontCollectionIndex, theme::SourceFontCollection,
};
use mo_common::Digest;
pub(in crate::source) use part::load;
pub(in crate::source) use read::Reader;
use schemars::JsonSchema;
pub use select::*;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SourceTableStylePart {
    pub part: String,
    pub sha256: Digest,
    pub source_ordinal: u32,
    pub default_style_id: String,
    /// Keys normalize GUID letter case; style_id preserves the declaration.
    pub styles: BTreeMap<String, SourceTableStyle>,
    pub retained_ordinals: Vec<u32>,
    pub compatibility: SourceCompatibility,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SourceTableStyle {
    pub source_ordinal: u32,
    pub style_id: String,
    pub name: String,
    pub background: Option<SourceTableBackgroundStyle>,
    pub parts: BTreeMap<TableStyleRegion, SourceTablePartStyle>,
    pub retained_ordinals: Vec<u32>,
    pub effect_nodes: BTreeMap<u32, SourceEffectNode>,
}
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, JsonSchema,
)]
#[serde(rename_all = "camelCase")]
pub enum TableStyleRegion {
    WholeTbl,
    Band1H,
    Band2H,
    Band1V,
    Band2V,
    LastCol,
    FirstCol,
    LastRow,
    SeCell,
    SwCell,
    FirstRow,
    NeCell,
    NwCell,
}
impl TableStyleRegion {
    pub(in crate::source::table) const ORDER: [Self; 13] = [
        Self::WholeTbl,
        Self::Band1H,
        Self::Band2H,
        Self::Band1V,
        Self::Band2V,
        Self::LastCol,
        Self::FirstCol,
        Self::LastRow,
        Self::SeCell,
        Self::SwCell,
        Self::FirstRow,
        Self::NeCell,
        Self::NwCell,
    ];
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SourceTablePartStyle {
    pub source_ordinal: u32,
    pub text: Option<SourceTableTextStyle>,
    pub cell: Option<SourceTableCellStyle>,
    pub retained_ordinals: Vec<u32>,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum TableOnOff {
    On,
    Off,
    Def,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SourceTableTextStyle {
    pub source_ordinal: u32,
    /// Absent and explicit def remain distinct declarations.
    pub bold: Option<TableOnOff>,
    pub italic: Option<TableOnOff>,
    pub font: Option<SourceTableFontStyle>,
    pub color: Option<SourceColor>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub enum SourceTableFontStyle {
    Collection {
        source_ordinal: u32,
        fonts: SourceFontCollection,
    },
    Reference {
        source_ordinal: u32,
        index: NativeFontCollectionIndex,
        color: Option<SourceColor>,
    },
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub enum SourceTableStyleFill {
    Direct { fill: Box<SourceFill> },
    Reference { reference: SourceFillReference },
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub enum SourceTableStyleLine {
    Direct { line: SourceLine },
    Reference { reference: SourceLineReference },
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub enum SourceTableStyleEffects {
    Direct { effects: SourceEffectProperties },
    Reference { reference: SourceEffectReference },
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SourceTableBackgroundStyle {
    pub source_ordinal: u32,
    pub fill: Option<SourceTableStyleFill>,
    pub effects: Option<SourceTableStyleEffects>,
    pub retained_ordinals: Vec<u32>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SourceTableCellStyle {
    pub source_ordinal: u32,
    pub borders: Option<SourceTableBorderStyle>,
    pub fill: Option<SourceTableStyleFill>,
    /// Preserved native 3D declaration; resolution remains explicit downstream.
    pub cell_3d_ordinal: Option<u32>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SourceTableBorderStyle {
    pub source_ordinal: u32,
    /// left, right, top, bottom, insideH, insideV, tl2br, tr2bl.
    pub edges: [Option<SourceTableStyleLine>; 8],
}
