use crate::{CompileError, interval::Interval, source_number};
use mo_common::Digest;
use mo_geometry::{Fixed, Point, Rect};
use mo_presentation_source::{
    PptxError,
    source::{
        SourceIndex, SourceObject, SourceObjectRef,
        drawingml::NativeCoordinate,
        table::{SourceCellAddress, grid::*},
        text::{SourceTextBodyBinding, SourceTextCatalog, bind_body},
    },
};

#[derive(Debug, Clone, Copy)]
pub struct TableGeometryLimits {
    pub grid: NativeTableGridLimits,
    pub max_objects: usize,
    pub max_coordinate_bytes: usize,
}
impl Default for TableGeometryLimits {
    fn default() -> Self {
        Self {
            grid: NativeTableGridLimits::default(),
            max_objects: 65_536,
            max_coordinate_bytes: 4 * 1024 * 1024,
        }
    }
}
#[derive(Debug, thiserror::Error)]
pub enum TableGeometryError {
    #[error(transparent)]
    Source(#[from] PptxError),
    #[error(transparent)]
    Grid(#[from] NativeTableGridError),
    #[error(transparent)]
    Coordinate(#[from] CompileError),
    #[error("negative native table dimension at ordinal {source_ordinal}")]
    NegativeDimension { source_ordinal: u32 },
    #[error("table geometry limit exceeded: {0}")]
    Limit(&'static str),
    #[error("table geometry cancelled")]
    Cancelled,
}
#[derive(Debug, Clone, Copy)]
pub struct TableBoundary {
    pub value: Fixed,
    pub conversion_error_bound: Fixed,
}
#[derive(Debug, Clone, Copy)]
pub struct TableCellGeometry {
    pub address: SourceCellAddress,
    pub source_ordinal: u32,
    pub region: NativeTableRegion,
    /// The physical cell, including one covered by a merge.
    pub physical: Rect,
    /// The whole rectangle owned by the origin. Paint/text uses this once.
    pub merged: Rect,
    /// Maximum error of any edge in the two returned rectangles.
    pub conversion_error_bound: Fixed,
}
/// O(cells + rows + columns) memory. Dimensions are accumulated once at Q96;
/// cells use direct edge lookup and text roots use their native ordinal index.
/// Zero dimensions remain zero, with no hidden minimum-size policy.
pub struct DeclaredTableGeometry<'a> {
    grid: std::sync::Arc<NativeTableGrid<'a>>,
    object: &'a SourceObject,
    catalog: &'a SourceTextCatalog,
    layout: std::sync::Arc<DeclaredTableLayout>,
}
pub(crate) struct DeclaredTableLayout {
    pub(crate) coordinate_bytes: usize,
    columns: Vec<TableBoundary>,
    rows: Vec<TableBoundary>,
    right_to_left: bool,
}
fn conflict() -> PptxError {
    PptxError::SourceConflict("table geometry differs from inspected source".into())
}
fn cancel(check: &dyn Fn() -> bool) -> Result<(), TableGeometryError> {
    if check() {
        Err(TableGeometryError::Cancelled)
    } else {
        Ok(())
    }
}
fn boundaries<'a>(
    mut values: impl ExactSizeIterator<Item = (u32, &'a NativeCoordinate)> + DoubleEndedIterator,
    reversed: bool,
    bytes: &mut usize,
    check: &dyn Fn() -> bool,
) -> Result<Vec<TableBoundary>, TableGeometryError> {
    let count = values.len();
    let mut positions = vec![
        TableBoundary {
            value: Fixed::ZERO,
            conversion_error_bound: Fixed::ZERO
        };
        count + 1
    ];
    let mut total = Interval::integer(0);
    for i in 0..count {
        cancel(check)?;
        // Accumulate from the physical left edge, including for RTL columns.
        // Only one Q96 accumulator is live; no BigInt per-edge array is needed.
        let (source_ordinal, value) = if reversed {
            values.next_back()
        } else {
            values.next()
        }
        .expect("exact native dimension iterator");
        *bytes = bytes
            .checked_sub(value.lexical().len())
            .ok_or(TableGeometryError::Limit("table coordinate bytes"))?;
        let value = source_number::coordinate_interval(value).map_err(|e| match e {
            source_number::PercentageError::LexicalLimit => {
                TableGeometryError::Limit("coordinate lexical bytes")
            }
            source_number::PercentageError::Range => CompileError::Range.into(),
        })?;
        if value.lo < 0.into() {
            return Err(TableGeometryError::NegativeDimension { source_ordinal });
        }
        total = total.add(&value);
        let (value, conversion_error_bound) = total.q32()?;
        positions[if reversed { count - 1 - i } else { i + 1 }] = TableBoundary {
            value,
            conversion_error_bound,
        };
    }
    Ok(positions)
}
impl<'a> DeclaredTableGeometry<'a> {
    pub fn prepare(
        index: &'a SourceIndex,
        expected_source_sha256: &Digest,
        reference: &SourceObjectRef,
        limits: TableGeometryLimits,
        check: &dyn Fn() -> bool,
    ) -> Result<Self, TableGeometryError> {
        cancel(check)?;
        if &index.source_sha256 != expected_source_sha256 {
            return Err(conflict().into());
        }
        let surface = index.surfaces.get(&reference.part).ok_or_else(conflict)?;
        if surface.objects.len() > limits.max_objects {
            return Err(TableGeometryError::Limit("table surface objects"));
        }
        let mut found = None;
        for object in &surface.objects {
            cancel(check)?;
            if object.native_id == reference.native_id && found.replace(object).is_some() {
                return Err(conflict().into());
            }
        }
        let object = found.ok_or_else(conflict)?;
        let table = object.table.as_ref().ok_or_else(conflict)?;
        let grid = NativeTableGrid::compile(table, limits.grid, check)?;
        Self::from_bound(
            object,
            &surface.text,
            std::sync::Arc::new(grid),
            limits,
            check,
        )
    }
    pub fn from_prepared(
        table: &mo_presentation_source::source::prepared::PreparedSourceTable<'a>,
        limits: TableGeometryLimits,
        check: &dyn Fn() -> bool,
    ) -> Result<Self, TableGeometryError> {
        cancel(check)?;
        let cost = table.cost();
        if table.surface().objects.len() > limits.max_objects
            || cost.cells > limits.grid.max_cells
            || cost.steps > limits.grid.max_steps
            || cost.id_bytes > limits.grid.max_id_bytes
        {
            return Err(TableGeometryError::Limit("prepared table geometry"));
        }
        Self::from_bound(
            table.object(),
            &table.surface().text,
            table.shared_grid(),
            limits,
            check,
        )
    }
    fn from_bound(
        object: &'a SourceObject,
        catalog: &'a SourceTextCatalog,
        grid: std::sync::Arc<NativeTableGrid<'a>>,
        limits: TableGeometryLimits,
        check: &dyn Fn() -> bool,
    ) -> Result<Self, TableGeometryError> {
        let table = grid.table();

        // Check the one canonical physical paragraph sequence once. Later cell
        // queries do not scan all preceding cells or duplicate their payloads.
        let mut end = 0usize;
        for (r, row) in table.rows.iter().enumerate() {
            for (c, cell) in row.cells.iter().enumerate() {
                cancel(check)?;
                if cell.paragraph_start as usize != end {
                    return Err(conflict().into());
                }
                let at = SourceCellAddress {
                    row: r as u32,
                    column: c as u32,
                };
                if let Some(binding) = bind_body(object, catalog, Some(at))? {
                    end = binding.range.end;
                }
            }
        }
        if end != object.paragraphs.len() || object.text_body_ordinal.is_some() {
            return Err(conflict().into());
        }
        let right_to_left = table.properties.as_ref().and_then(|p| p.right_to_left) == Some(true);
        let mut bytes = limits.max_coordinate_bytes;
        let columns = boundaries(
            table.columns.iter().map(|c| (c.source_ordinal, &c.width)),
            right_to_left,
            &mut bytes,
            check,
        )?;
        let rows = boundaries(
            table.rows.iter().map(|r| (r.source_ordinal, &r.height)),
            false,
            &mut bytes,
            check,
        )?;
        Ok(Self {
            grid,
            object,
            catalog,
            layout: std::sync::Arc::new(DeclaredTableLayout {
                columns,
                rows,
                right_to_left,
                coordinate_bytes: limits.max_coordinate_bytes - bytes,
            }),
        })
    }
    pub(crate) fn retained_layout(&self) -> std::sync::Arc<DeclaredTableLayout> {
        std::sync::Arc::clone(&self.layout)
    }
    pub(crate) fn is_bound_to(
        &self,
        table: &mo_presentation_source::source::prepared::PreparedSourceTable<'_>,
    ) -> bool {
        std::ptr::eq(self.object, table.object())
            && std::ptr::eq(self.catalog, &table.surface().text)
            && std::ptr::eq(self.grid.table(), table.grid().table())
    }
    pub(crate) fn from_retained(
        table: &mo_presentation_source::source::prepared::PreparedSourceTable<'a>,
        layout: std::sync::Arc<DeclaredTableLayout>,
        limits: TableGeometryLimits,
        check: &dyn Fn() -> bool,
    ) -> Result<Self, TableGeometryError> {
        cancel(check)?;
        let cost = table.cost();
        if table.surface().objects.len() > limits.max_objects
            || cost.cells > limits.grid.max_cells
            || cost.steps > limits.grid.max_steps
            || cost.id_bytes > limits.grid.max_id_bytes
            || layout.coordinate_bytes > limits.max_coordinate_bytes
        {
            return Err(TableGeometryError::Limit("retained table geometry"));
        }
        Ok(Self {
            grid: table.shared_grid(),
            object: table.object(),
            catalog: &table.surface().text,
            layout,
        })
    }
    pub fn shared_grid(&self) -> std::sync::Arc<NativeTableGrid<'a>> {
        self.grid.clone()
    }
    pub fn grid(&self) -> &NativeTableGrid<'a> {
        &self.grid
    }
    /// Physical-order grid edges; x decreases for an RTL table.
    pub fn columns(&self) -> &[TableBoundary] {
        &self.layout.columns
    }
    pub fn rows(&self) -> &[TableBoundary] {
        &self.layout.rows
    }
    pub fn right_to_left(&self) -> bool {
        self.layout.right_to_left
    }
    pub fn body(
        &self,
        at: SourceCellAddress,
    ) -> Result<Option<SourceTextBodyBinding<'a>>, TableGeometryError> {
        Ok(bind_body(self.object, self.catalog, Some(at))?)
    }
    fn rectangle(&self, at: SourceCellAddress, rows: u32, columns: u32) -> (Rect, Fixed) {
        let mut x = [
            self.layout.columns[at.column as usize],
            self.layout.columns[(at.column + columns) as usize],
        ];
        if self.layout.right_to_left {
            x.swap(0, 1);
        }
        let y = [
            self.layout.rows[at.row as usize],
            self.layout.rows[(at.row + rows) as usize],
        ];
        let error = x
            .iter()
            .chain(y.iter())
            .map(|e| e.conversion_error_bound)
            .max()
            .expect("four edges");
        (
            Rect {
                min: Point {
                    x: x[0].value,
                    y: y[0].value,
                },
                max: Point {
                    x: x[1].value,
                    y: y[1].value,
                },
            },
            error,
        )
    }
    pub fn cell(&self, at: SourceCellAddress) -> Option<TableCellGeometry> {
        let cell = self.grid.cell(at)?;
        let region = *self.grid.region(at)?;
        let (physical, p_error) = self.rectangle(at, 1, 1);
        let (merged, m_error) = self.rectangle(region.origin, region.rows, region.columns);
        Some(TableCellGeometry {
            address: at,
            source_ordinal: cell.source_ordinal,
            region,
            physical,
            merged,
            conversion_error_bound: p_error.max(m_error),
        })
    }
}
