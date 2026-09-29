use super::*;
use crate::source::{SourceColorMapRef, theme::SourceThemeSchemeRef};
use mo_common::Digest;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TableBorderTarget {
    pub native_id: u32,
    pub cell: SourceCellAddress,
    pub edge: TableCellEdge,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SourceTableBorderQuery {
    pub expected_source_sha256: Digest,
    pub surface: String,
    pub targets: Vec<TableBorderTarget>,
    pub line_profile: LineProfile,
    pub fill_profile: FillProfile,
    pub color_profile: ColorProfile,
    pub context: ColorContext,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SourceTableBorders {
    pub source_sha256: Digest,
    pub surface: String,
    pub line_profile: LineProfile,
    pub fill_profile: FillProfile,
    pub color_profile: ColorProfile,
    pub color_mapping: Option<SourceColorMapRef>,
    pub color_scheme: Option<SourceThemeSchemeRef>,
    pub targets: Vec<SourceTableBorderResult>,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SourceTableBorderResult {
    pub target: TableBorderTarget,
    pub topology: Option<TableBorderTopology>,
    pub stroke: LineGeometryOutcome,
    pub fill: SourceFillColorResult,
}
/// Shared identity is a declaration-grid segment, not a winner or a device
/// path. Adjacent sides share a key; RTL swaps which side names that segment.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub enum TableBorderPosition {
    Horizontal {
        row: u32,
        column: u32,
    },
    Vertical {
        row: u32,
        column: u32,
    },
    Diagonal {
        cell: SourceCellAddress,
        edge: TableCellEdge,
    },
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TableBorderTopology {
    pub position: TableBorderPosition,
    pub region: grid::NativeTableRegion,
    pub neighbour: Option<SourceCellAddress>,
    pub neighbour_region: Option<grid::NativeTableRegion>,
    pub inside_merge: bool,
}
#[derive(Debug, Clone, Copy, Default)]
pub struct TableBorderLimits {
    pub fills: FillColorLimits,
    pub lines: LineResolveLimits,
}
