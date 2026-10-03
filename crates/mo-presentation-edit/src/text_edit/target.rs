//! An interactive text target is a shape or a visible authored cell. Covered
//! cells retain independent storage for splitting, but are not editing targets.
use super::*;

pub(super) fn authored_body<'a>(
    document: &'a Document,
    command: &TextEditCommand,
    check: &dyn Fn() -> bool,
) -> Result<Option<&'a TextBody>, EditError> {
    let object = document
        .objects
        .get(&command.object)
        .ok_or_else(|| EditError::input("text object does not exist"))?;
    match (&object.content, &command.cell) {
        (ObjectContent::Shape { text, .. }, None) => Ok(text.as_ref()),
        (ObjectContent::Table { table }, Some(id)) => {
            for cell in table.rows.iter().flat_map(|r| &r.cells) {
                cancelled(check)?;
                if &cell.id == id {
                    if matches!(cell.merge, TableCellMerge::Covered { .. }) {
                        return Err(EditError::input(
                            "covered table cell is not a text editing target",
                        ));
                    }
                    return Ok(cell.text.as_ref());
                }
            }
            Err(EditError::input("text table cell does not exist"))
        }
        (ObjectContent::Table { .. }, None) => {
            Err(EditError::input("table text requires a cell identity"))
        }
        (_, Some(_)) => Err(EditError::input("cell identity requires an authored table")),
        (ObjectContent::RetainedSource { .. }, None) => {
            Err(EditError::input("text action requires native preservation"))
        }
        _ => Err(EditError::input("object cannot own an authored text body")),
    }
}

pub(super) fn initialize(
    text: &str,
    setup: &TextBodySetup,
    command: &TextEditCommand,
    ids: &mut Ids,
    check: &dyn Fn() -> bool,
) -> Result<(TextBody, TextSelection), EditError> {
    let paragraph = ids.paragraph();
    let mut body = TextBody {
        paragraphs: vec![Paragraph {
            id: paragraph.clone(),
            style: setup.paragraph_style.clone(),
            default_run_style: setup.default_run_style.clone(),
            runs: vec![],
        }],
        style: setup.style.clone(),
        insets: setup.insets,
        wrap: setup.wrap,
        overflow: setup.overflow,
    };
    let anchor = TextAnchor {
        paragraph,
        scalar_offset: 0,
        affinity: Affinity::After,
    };
    let range = Range {
        first: 0,
        last: 0,
        start: anchor.clone(),
        end: anchor,
    };
    let (selection, _) = replace(&mut body, &range, text, &command.object, ids, check)?;
    // There was no old paragraph: do not emit a range map from a fabricated ID.
    Ok((body, selection))
}
