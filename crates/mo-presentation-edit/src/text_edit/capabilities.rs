use super::*;

fn unavailable(reason: TextEditRestriction) -> TextEditAvailability {
    TextEditAvailability::Unavailable { reason }
}
fn assess(
    result: Result<retained::NativeRange<'_>, EditError>,
) -> Result<TextEditAvailability, EditError> {
    match result {
        Ok(_) => Ok(TextEditAvailability::Available),
        Err(EditError::TextRestricted(reason)) => Ok(unavailable(*reason)),
        Err(error) => Err(error),
    }
}

/// Cheap selection/model preflight using the same target and native range
/// validators as execution. No candidate snapshot clone, revision or font work.
pub fn text_capabilities(
    snapshot: &Snapshot,
    query: &TextCapabilitiesQuery,
    check: &dyn Fn() -> bool,
) -> Result<TextEditingCapabilities, EditError> {
    cancelled(check)?;
    let unavailable_all = |reason: TextEditRestriction| TextEditingCapabilities {
        document_id: snapshot.document().id.clone(),
        revision: snapshot.revision().clone(),
        object: query.object.clone(),
        cell: query.cell.clone(),
        initialize: unavailable(reason.clone()),
        replace: unavailable(reason.clone()),
        delete: unavailable(reason.clone()),
        character_style: unavailable(reason),
        replacement_policy: None,
    };
    let result = match target::resolve(
        snapshot.document(),
        &query.object,
        query.cell.as_ref(),
        check,
    )? {
        target::Target::Unavailable(reason) => return Ok(unavailable_all(reason)),
        target::Target::Authored(None) => {
            let mut result = unavailable_all(TextEditRestriction::MissingTextBody);
            result.initialize = TextEditAvailability::Available;
            result.replacement_policy = Some(TextReplacementPolicy::AuthoredBody);
            result
        }
        target::Target::Authored(Some(body)) => {
            let mut result = unavailable_all(TextEditRestriction::SelectionRequired);
            result.initialize = unavailable(TextEditRestriction::TextBodyExists);
            result.replacement_policy = Some(TextReplacementPolicy::AuthoredBody);
            if let Some(selection) = &query.selection {
                let range = ordered(body, selection, check)?;
                result.replace = TextEditAvailability::Available;
                result.delete = TextEditAvailability::Available;
                result.character_style = if range.first == range.last
                    && range.start.scalar_offset == range.end.scalar_offset
                {
                    unavailable(TextEditRestriction::NonemptySelectionRequired)
                } else {
                    TextEditAvailability::Available
                };
            }
            result
        }
        target::Target::Retained(paragraphs) => {
            let mut result = unavailable_all(TextEditRestriction::SelectionRequired);
            result.initialize = unavailable(TextEditRestriction::NativeStructureRequired);
            result.character_style = unavailable(TextEditRestriction::NativeStructureRequired);
            result.replacement_policy = Some(TextReplacementPolicy::RetainedTextLeaves);
            if let Some(selection) = &query.selection {
                result.replace = assess(retained::range(
                    snapshot.document(),
                    &query.object,
                    paragraphs,
                    selection,
                    true,
                    check,
                ))?;
                result.delete = assess(retained::range(
                    snapshot.document(),
                    &query.object,
                    paragraphs,
                    selection,
                    false,
                    check,
                ))?;
            }
            result
        }
    };
    cancelled(check)?;
    Ok(result)
}
