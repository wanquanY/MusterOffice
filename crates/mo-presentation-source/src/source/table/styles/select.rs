//! Borrowed declaration selection. This is not font/color resolution or a claim
//! of Office visual conformance. MS-OI29500 2.1.1265 defines region precedence.
use super::*;
use crate::source::table::{
    SourceCellAddress, SourceTable, TableCellEdge, TableStyleEdge, grid::NativeTableGrid,
};

#[derive(Debug, Clone, thiserror::Error, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub enum TableStyleSelectionError {
    #[error("conflicting inline and referenced table styles")]
    ConflictingStyles,
    #[error("invalid native table style identity")]
    InvalidIdentity,
    #[error("table style definition is absent from the source catalog")]
    MissingDefinition,
    #[error("table style grid differs from the bound source")]
    GridMismatch,
    #[error("table style cell is outside the grid")]
    CellOutsideGrid,
    #[error("unresolved table style declaration at ordinal {source_ordinal}")]
    RetainedDeclaration { source_ordinal: u32 },
    #[error("table style selection cancelled")]
    Cancelled,
}
fn check(cancelled: &dyn Fn() -> bool) -> Result<(), TableStyleSelectionError> {
    if cancelled() {
        Err(TableStyleSelectionError::Cancelled)
    } else {
        Ok(())
    }
}
/// The source location remains separate from each selected native ordinal.
/// Inline styles use the owning object's part; shared styles use this part.
#[derive(Debug, Clone, Copy)]
pub enum TableStyleLocation<'a> {
    Inline,
    Shared(&'a SourceTableStylePart),
}
/// Bind once per table. The caller owns the inspected SourceIndex/digest binding;
/// this API never reads files or substitutes a built-in/default style by name.
pub struct BoundTableStyle<'a> {
    table: &'a SourceTable,
    definition: Option<(&'a SourceTableStyle, TableStyleLocation<'a>)>,
}
/// Source-relative identity, retained only with its immutable source owner.
#[derive(Clone)]
pub(in crate::source) enum RetainedTableStyle {
    None,
    Inline,
    Shared(String),
}
impl<'a> BoundTableStyle<'a> {
    pub(in crate::source) fn retain(&self) -> RetainedTableStyle {
        match self.definition {
            None => RetainedTableStyle::None,
            Some((_, TableStyleLocation::Inline)) => RetainedTableStyle::Inline,
            Some((style, TableStyleLocation::Shared(_))) => {
                RetainedTableStyle::Shared(style.style_id.trim().to_ascii_uppercase())
            }
        }
    }
    pub(in crate::source) fn bind_retained(
        table: &'a SourceTable,
        catalog: Option<&'a SourceTableStylePart>,
        selected: &RetainedTableStyle,
    ) -> Result<Self, TableStyleSelectionError> {
        let definition = match selected {
            RetainedTableStyle::None => None,
            RetainedTableStyle::Inline => Some((
                table
                    .properties
                    .as_ref()
                    .and_then(|p| p.inline_style.as_deref())
                    .ok_or(TableStyleSelectionError::MissingDefinition)?,
                TableStyleLocation::Inline,
            )),
            RetainedTableStyle::Shared(key) => {
                let catalog = catalog.ok_or(TableStyleSelectionError::MissingDefinition)?;
                Some((
                    catalog
                        .styles
                        .get(key)
                        .ok_or(TableStyleSelectionError::MissingDefinition)?,
                    TableStyleLocation::Shared(catalog),
                ))
            }
        };
        Ok(Self { table, definition })
    }

    pub fn bind(
        table: &'a SourceTable,
        catalog: Option<&'a SourceTableStylePart>,
        cancelled: &dyn Fn() -> bool,
    ) -> Result<Self, TableStyleSelectionError> {
        check(cancelled)?;
        let mut out = Self {
            table,
            definition: None,
        };
        let Some(p) = &table.properties else {
            return Ok(out);
        };
        match (&p.inline_style, &p.style_id) {
            (Some(_), Some(_)) => return Err(TableStyleSelectionError::ConflictingStyles),
            (Some(s), None) => {
                read::guid(&s.style_id).map_err(|_| TableStyleSelectionError::InvalidIdentity)?;
                out.definition = Some((s, TableStyleLocation::Inline));
            }
            (None, Some(id)) => {
                let key = read::guid(id).map_err(|_| TableStyleSelectionError::InvalidIdentity)?;
                let c = catalog.ok_or(TableStyleSelectionError::MissingDefinition)?;
                let s = c
                    .styles
                    .get(&key)
                    .ok_or(TableStyleSelectionError::MissingDefinition)?;
                if read::guid(&s.style_id).map_err(|_| TableStyleSelectionError::InvalidIdentity)?
                    != key
                {
                    return Err(TableStyleSelectionError::InvalidIdentity);
                }
                out.definition = Some((s, TableStyleLocation::Shared(c)));
            }
            // tblStyleLst@def is an insertion default, not a missing reference.
            (None, None) => (),
        }
        Ok(out)
    }
    pub fn definition(&self) -> Option<(&'a SourceTableStyle, TableStyleLocation<'a>)> {
        self.definition
    }
    /// Selects declarations for one physical cell side. Region membership on
    /// both sides determines an interior border; separated bands are distinct
    /// regions. This does not pick a winner between adjacent native cell sides.
    pub fn border(
        &self,
        grid: &NativeTableGrid<'_>,
        at: SourceCellAddress,
        edge: TableCellEdge,
        cancelled: &dyn Fn() -> bool,
    ) -> Result<TableBorderStyleLayers<'a>, TableStyleSelectionError> {
        let cells = self.cell(grid, at, cancelled)?;
        let neighbour = edge
            .neighbour(at, self.table)
            .map(|n| active_regions(self.table, n));
        let mut layers = [None; 13];
        for (slot, layer) in cells.layers.iter().enumerate() {
            check(cancelled)?;
            if let Some(layer) = layer {
                let selected = TableStyleEdge::for_cell(edge, neighbour.is_some_and(|n| n[slot]));
                layers[slot] = Some(TableBorderStyleLayer {
                    region: layer.region,
                    edge: selected,
                    part: layer.part,
                    line: layer
                        .part
                        .cell
                        .as_ref()
                        .and_then(|c| c.borders.as_ref())
                        .and_then(|b| b.edges[selected.index()].as_ref()),
                });
            }
        }
        Ok(TableBorderStyleLayers { layers })
    }
    /// O(13) time and constant storage. `at` is a physical native cell address.
    /// A renderer selects a merge origin once; covered payloads stay addressable
    /// for edits. This does not resolve merged-edge border conflict semantics.
    pub fn cell(
        &self,
        grid: &NativeTableGrid<'_>,
        at: SourceCellAddress,
        cancelled: &dyn Fn() -> bool,
    ) -> Result<TableCellStyleLayers<'a>, TableStyleSelectionError> {
        check(cancelled)?;
        if !std::ptr::eq(grid.table(), self.table) {
            return Err(TableStyleSelectionError::GridMismatch);
        }
        if grid.cell(at).is_none() {
            return Err(TableStyleSelectionError::CellOutsideGrid);
        }
        let mut out = TableCellStyleLayers {
            layers: [None; 13],
            definition: self.definition,
        };
        let Some((style, location)) = self.definition else {
            return Ok(out);
        };
        if let TableStyleLocation::Shared(catalog) = location
            && let Some(&source_ordinal) = catalog.retained_ordinals.first()
        {
            return Err(TableStyleSelectionError::RetainedDeclaration { source_ordinal });
        }
        if let Some(&source_ordinal) = style.retained_ordinals.first() {
            return Err(TableStyleSelectionError::RetainedDeclaration { source_ordinal });
        }
        let active = active_regions(self.table, at);
        for (slot, region) in TableStyleRegion::ORDER.into_iter().enumerate() {
            check(cancelled)?;
            if active[slot] {
                out.layers[slot] = style
                    .parts
                    .get(&region)
                    .map(|part| TableStyleLayer { region, part });
            }
        }
        Ok(out)
    }
}

// Native first/last rows and columns are indexed in declaration order, before
// RTL placement. A header is excluded from band counts. The renderer must still
// validate the visual profile for merged cells, RTL and degenerate dimensions.
fn active_regions(table: &SourceTable, at: SourceCellAddress) -> [bool; 13] {
    let Some(p) = &table.properties else {
        return [false; 13];
    };
    let first_row = p.first_row == Some(true) && at.row == 0;
    let last_row = p.last_row == Some(true) && at.row as usize + 1 == table.rows.len();
    let first_col = p.first_column == Some(true) && at.column == 0;
    let last_col = p.last_column == Some(true) && at.column as usize + 1 == table.columns.len();
    let band_row = p.band_rows == Some(true) && !first_row && !last_row;
    let band_col = p.band_columns == Some(true) && !first_col && !last_col;
    let row_first_band = at.row % 2 == u32::from(p.first_row == Some(true));
    let col_first_band = at.column % 2 == u32::from(p.first_column == Some(true));
    [
        true,
        band_row && row_first_band,
        band_row && !row_first_band,
        band_col && col_first_band,
        band_col && !col_first_band,
        last_col,
        first_col,
        last_row,
        last_row && last_col,
        last_row && first_col,
        first_row,
        first_row && last_col,
        first_row && first_col,
    ]
}

#[derive(Debug, Clone, Copy)]
pub struct TableStyleLayer<'a> {
    pub region: TableStyleRegion,
    pub part: &'a SourceTablePartStyle,
}
#[derive(Debug, Clone, Copy)]
pub struct TableBorderStyleLayer<'a> {
    pub region: TableStyleRegion,
    pub edge: TableStyleEdge,
    pub part: &'a SourceTablePartStyle,
    pub line: Option<&'a SourceTableStyleLine>,
}
pub struct TableBorderStyleLayers<'a> {
    layers: [Option<TableBorderStyleLayer<'a>>; 13],
}
impl<'a> TableBorderStyleLayers<'a> {
    pub fn iter(&self) -> impl DoubleEndedIterator<Item = TableBorderStyleLayer<'a>> + '_ {
        self.layers.iter().filter_map(|v| *v)
    }
}
pub struct TableCellStyleLayers<'a> {
    layers: [Option<TableStyleLayer<'a>>; 13],
    definition: Option<(&'a SourceTableStyle, TableStyleLocation<'a>)>,
}
#[derive(Debug, Clone, Copy)]
pub struct SelectedTableStyleValue<T> {
    pub region: TableStyleRegion,
    pub source_ordinal: u32,
    pub value: T,
}
#[derive(Debug, Clone, Copy, Default)]
pub struct SelectedTableTextStyle<'a> {
    pub bold: Option<SelectedTableStyleValue<bool>>,
    pub italic: Option<SelectedTableStyleValue<bool>>,
    pub font: Option<SelectedTableStyleValue<&'a SourceTableFontStyle>>,
    pub color: Option<SelectedTableStyleValue<&'a SourceColor>>,
}
impl<'a> TableCellStyleLayers<'a> {
    /// Low to high precedence, retaining the original declarations and ordinals.
    pub fn iter(&self) -> impl DoubleEndedIterator<Item = TableStyleLayer<'a>> + '_ {
        self.layers.iter().filter_map(|v| *v)
    }
    pub fn definition(&self) -> Option<(&'a SourceTableStyle, TableStyleLocation<'a>)> {
        self.definition
    }
    /// Select each table text property independently. Absent/def preserve the
    /// lower layer; explicit off, empty collections and fontRef none do not.
    /// Direct run formatting and theme/font/color binding are applied downstream.
    pub fn text(
        &self,
        cancelled: &dyn Fn() -> bool,
    ) -> Result<SelectedTableTextStyle<'a>, TableStyleSelectionError> {
        check(cancelled)?;
        let mut out = SelectedTableTextStyle::default();
        for layer in self.iter() {
            check(cancelled)?;
            if let Some(&source_ordinal) = layer.part.retained_ordinals.first() {
                return Err(TableStyleSelectionError::RetainedDeclaration { source_ordinal });
            }
            let Some(tx) = &layer.part.text else { continue };
            for (src, dst) in [(tx.bold, &mut out.bold), (tx.italic, &mut out.italic)] {
                if let Some(TableOnOff::On | TableOnOff::Off) = src {
                    *dst = Some(SelectedTableStyleValue {
                        region: layer.region,
                        source_ordinal: tx.source_ordinal,
                        value: src == Some(TableOnOff::On),
                    });
                }
            }
            if let Some(font) = &tx.font {
                let source_ordinal = match font {
                    SourceTableFontStyle::Collection { source_ordinal, .. }
                    | SourceTableFontStyle::Reference { source_ordinal, .. } => *source_ordinal,
                };
                out.font = Some(SelectedTableStyleValue {
                    region: layer.region,
                    source_ordinal,
                    value: font,
                });
            }
            if let Some(color) = &tx.color {
                out.color = Some(SelectedTableStyleValue {
                    region: layer.region,
                    source_ordinal: color.source_ordinal,
                    value: color,
                });
            }
        }
        Ok(out)
    }
}
