use super::*;
use crate::source::{SourceRun, table::*, text::SourceTextCatalog};

pub(crate) fn table(
    x: &mut Xml,
    value: &SourceTable,
    catalog: &SourceTextCatalog,
    paragraphs: &[Vec<SourceRun>],
    check: &dyn Fn() -> bool,
) -> Result<(), PptxError> {
    if !value.retained_ordinals.is_empty() {
        return Err(unexpected());
    }
    x.raw("<a:graphic><a:graphicData uri=\"http://schemas.openxmlformats.org/drawingml/2006/table\"><a:tbl>")?;
    if let Some(p) = &value.properties {
        // The author projection does not yet produce effects or inline styles.
        // Source-bound export preserves their XML through the source plan.
        if p.effects.is_some() || p.inline_style.is_some() {
            return Err(unexpected());
        }
        x.raw("<a:tblPr")?;
        for (name, flag) in [
            ("rtl", p.right_to_left),
            ("firstRow", p.first_row),
            ("firstCol", p.first_column),
            ("lastRow", p.last_row),
            ("lastCol", p.last_column),
            ("bandRow", p.band_rows),
            ("bandCol", p.band_columns),
        ] {
            attribute(x, name, &flag)?;
        }
        x.raw(">")?;
        if let Some(f) = &p.fill {
            fill(x, f)?;
        }
        if let Some(id) = &p.style_id {
            x.raw("<a:tableStyleId>")?;
            x.text(id)?;
            x.raw("</a:tableStyleId>")?;
        }
        x.raw("</a:tblPr>")?;
    }
    x.raw("<a:tblGrid>")?;
    for column in &value.columns {
        crate::cancelled(check)?;
        x.raw("<a:gridCol")?;
        x.attr("w", lexical(&column.width)?)?;
        x.raw("/>")?;
    }
    x.raw("</a:tblGrid>")?;
    for row in &value.rows {
        crate::cancelled(check)?;
        x.raw("<a:tr")?;
        x.attr("h", lexical(&row.height)?)?;
        x.raw(">")?;
        for cell in &row.cells {
            crate::cancelled(check)?;
            x.raw("<a:tc")?;
            attribute(x, "id", &cell.native_id)?;
            attribute(x, "rowSpan", &cell.row_span)?;
            attribute(x, "gridSpan", &cell.grid_span)?;
            attribute(x, "hMerge", &cell.horizontal_merge)?;
            attribute(x, "vMerge", &cell.vertical_merge)?;
            x.raw(">")?;
            if let Some(root) = cell.text_body_ordinal {
                let start = cell.paragraph_start as usize;
                let end = start
                    .checked_add(cell.paragraph_count as usize)
                    .ok_or_else(unexpected)?;
                text::cell_text(
                    x,
                    catalog,
                    root,
                    paragraphs.get(start..end).ok_or_else(unexpected)?,
                )?;
            } else if cell.paragraph_count != 0 {
                return Err(unexpected());
            }
            if let Some(p) = &cell.properties {
                cell_properties(x, p)?;
            }
            x.raw("</a:tc>")?;
        }
        x.raw("</a:tr>")?;
    }
    x.raw("</a:tbl></a:graphicData></a:graphic>")
}
fn cell_properties(x: &mut Xml, p: &SourceTableCellProperties) -> Result<(), PptxError> {
    x.raw("<a:tcPr")?;
    for (name, v) in [
        ("marL", &p.margins.left),
        ("marR", &p.margins.right),
        ("marT", &p.margins.top),
        ("marB", &p.margins.bottom),
    ] {
        attribute(x, name, v)?;
    }
    attribute(x, "anchor", &p.vertical_alignment)?;
    attribute(x, "anchorCtr", &p.center_anchor)?;
    attribute(x, "vert", &p.vertical)?;
    attribute(x, "horzOverflow", &p.horizontal_overflow)?;
    x.raw(">")?;
    for (tag, line) in ["lnL", "lnR", "lnT", "lnB", "lnTlToBr", "lnBlToTr"]
        .into_iter()
        .zip(&p.borders)
    {
        if let Some(line) = line {
            paint::line_tag(x, line, tag)?;
        }
    }
    if let Some(f) = &p.fill {
        fill(x, f)?;
    }
    x.raw("</a:tcPr>")
}
