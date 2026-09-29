use super::*;
use crate::source::{
    SourceRun, SourceSurface,
    table::*,
    text::{NativeTextAnchor, SourceTextValue},
};
use mo_presentation_model::*;

pub(super) fn build(
    table: &Table,
    owner: u32,
    document: &Document,
    surface: &mut SourceSurface,
    paragraphs: &mut Vec<Vec<SourceRun>>,
    ord: &mut Ordinals,
) -> Result<SourceTable, PptxError> {
    let grid = TableGrid::compile(
        table,
        ValidationLimits::default().max_table_cells,
        ord.check,
    )
    .map_err(|e| match e {
        TableGridError::Cancelled => PptxError::Cancelled,
        e => value_error("table", e.to_string()),
    })?;
    let mut out = SourceTable {
        source_ordinal: ord.next()?,
        properties: Some(SourceTableProperties {
            source_ordinal: ord.next()?,
            ..Default::default()
        }),
        grid_ordinal: ord.next()?,
        columns: vec![],
        rows: vec![],
        retained_ordinals: vec![],
    };
    for column in &table.columns {
        geometry::coordinate(column.width)?;
        out.columns.push(SourceTableColumn {
            source_ordinal: ord.next()?,
            width: native(column.width.get().to_string())?,
        });
    }
    for (r, row) in table.rows.iter().enumerate() {
        cancelled(ord.check)?;
        geometry::coordinate(row.height)?;
        let row_ordinal = ord.next()?;
        let mut cells = Vec::with_capacity(row.cells.len());
        for (c, cell) in row.cells.iter().enumerate() {
            let source_ordinal = ord.next()?;
            let rect = grid.rectangle(&cell.id).expect("validated grid cell");
            let mut margins = SourceTableMargins::default();
            if let Some(body) = &cell.text {
                let inset = |v: mo_common::Emu| {
                    if v.get() > i32::MAX as i64 {
                        return Err(value_error("table margin", "outside native range"));
                    }
                    native(v.get().to_string()).map(Some)
                };
                margins = SourceTableMargins {
                    left: inset(body.insets.left)?,
                    right: inset(body.insets.right)?,
                    top: inset(body.insets.top)?,
                    bottom: inset(body.insets.bottom)?,
                };
            }
            let mut text = text::TextBuilder::new(ord);
            let (root, content) = text.cell_body(
                owner,
                SourceCellAddress {
                    row: r as u32,
                    column: c as u32,
                },
                cell.text.as_ref(),
                document,
            )?;
            let (vertical, horizontal_overflow) = text.catalog.nodes[&root]
                .children
                .iter()
                .find_map(|id| {
                    if let SourceTextValue::Body { attributes } = &text.catalog.nodes[id].value {
                        Some((attributes.vertical, attributes.horizontal_overflow))
                    } else {
                        None
                    }
                })
                .expect("authored body properties");
            let paragraph_start = u32::try_from(paragraphs.len())
                .map_err(|_| PptxError::Limit("table paragraphs"))?;
            let paragraph_count =
                u32::try_from(content.len()).map_err(|_| PptxError::Limit("table paragraphs"))?;
            paragraphs.extend(content);
            surface.text.roots.extend(text.catalog.roots);
            surface.text.nodes.extend(text.catalog.nodes);
            let properties_ordinal = ord.next()?;
            let fill = match &cell.style.fill {
                Inherited::Inherit => None,
                Inherited::Value(fill) => Some(paint::fill(fill, ord)?),
            };
            let mut borders = std::array::from_fn(|_| None);
            for (i, (_, edge)) in cell.style.borders.edges().into_iter().enumerate() {
                if let Inherited::Value(stroke) = edge {
                    borders[i] = Some(paint::line(stroke, ord)?);
                }
            }
            let anchor = match cell.style.vertical_alignment {
                Inherited::Inherit => None,
                Inherited::Value(v) => Some(match v {
                    TableVerticalAlignment::Top => NativeTextAnchor::T,
                    TableVerticalAlignment::Center => NativeTextAnchor::Ctr,
                    TableVerticalAlignment::Bottom => NativeTextAnchor::B,
                    TableVerticalAlignment::Justified => NativeTextAnchor::Just,
                    TableVerticalAlignment::Distributed => NativeTextAnchor::Dist,
                }),
            };
            cells.push(SourceTableCell {
                source_ordinal,
                native_id: None,
                row_span: (r == rect.row && rect.rows > 1).then_some(rect.rows as i32),
                grid_span: (c == rect.column && rect.columns > 1).then_some(rect.columns as i32),
                horizontal_merge: (c != rect.column).then_some(true),
                vertical_merge: (r != rect.row).then_some(true),
                text_body_ordinal: Some(root),
                paragraph_start,
                paragraph_count,
                properties: Some(SourceTableCellProperties {
                    source_ordinal: properties_ordinal,
                    margins,
                    vertical,
                    horizontal_overflow,
                    vertical_alignment: anchor,
                    fill,
                    borders,
                    ..Default::default()
                }),
            });
        }
        out.rows.push(SourceTableRow {
            source_ordinal: row_ordinal,
            height: native(row.height.get().to_string())?,
            cells,
        });
    }
    Ok(out)
}
