//! Borrowed text scope, retaining the owning native object and physical cell.
use super::{NativeTextElement, SourceTextCatalog, SourceTextRoot};
use crate::{
    PptxError,
    source::{SourceObject, SourceRun, table::SourceCellAddress},
};
use std::ops::Range;

pub struct SourceTextBodyBinding<'a> {
    pub root: &'a SourceTextRoot,
    /// Indices in the owning object's canonical, physical paragraph sequence.
    pub range: Range<usize>,
    pub paragraphs: &'a [Vec<SourceRun>],
}

/// Select a shape body or a physical cell body without copying text, inventing
/// objects, or scanning every other cell. The source must come from inspection
/// or the author compiler; this does not authenticate caller-supplied indexes.
pub fn bind_body<'a>(
    object: &'a SourceObject,
    catalog: &'a SourceTextCatalog,
    cell: Option<SourceCellAddress>,
) -> Result<Option<SourceTextBodyBinding<'a>>, PptxError> {
    let conflict =
        || PptxError::SourceConflict("native text body scope differs from source".into());
    let (ordinal, range) = if let Some(at) = cell {
        let c = object
            .table
            .as_ref()
            .and_then(|t| t.rows.get(at.row as usize))
            .and_then(|r| r.cells.get(at.column as usize))
            .ok_or_else(conflict)?;
        let start = c.paragraph_start as usize;
        let end = start
            .checked_add(c.paragraph_count as usize)
            .ok_or_else(conflict)?;
        (c.text_body_ordinal, start..end)
    } else {
        (object.text_body_ordinal, 0..object.paragraphs.len())
    };
    let paragraphs = object.paragraphs.get(range.clone()).ok_or_else(conflict)?;
    let Some(ordinal) = ordinal else {
        // Tables intentionally have no ordinary shape text body.
        if (cell.is_some() || object.table.is_none()) && !range.is_empty() {
            return Err(conflict());
        }
        return Ok(None);
    };
    let root = catalog.root(ordinal).ok_or_else(conflict)?;
    let node = catalog.nodes.get(&ordinal).ok_or_else(conflict)?;
    if root.owner != Some(object.native_id)
        || root.cell != cell
        || node.element != NativeTextElement::TxBody
        || node.parent.is_some()
    {
        return Err(conflict());
    }
    Ok(Some(SourceTextBodyBinding {
        root,
        range,
        paragraphs,
    }))
}
