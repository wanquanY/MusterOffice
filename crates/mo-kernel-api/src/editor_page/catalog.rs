//! Page-local identities come from admitted native declarations, never from
//! host guesses or paint binding ordinals. Ancestors preserve group selection.
use super::*;
use mo_common::ObjectId;
use mo_pptx::source::{SourceIndex, SourceObjectKind, SourceObjectRef};
use std::collections::{BTreeMap, BTreeSet};

pub(super) fn build(
    page: &SourceEditorPage,
    source: &SourceIndex,
    model_id: impl Fn(&SourceObjectRef) -> Result<Option<ObjectId>, PptxResourcePageFailure>,
    check: &dyn Fn() -> bool,
) -> Result<Vec<EditorPageObjectInfo>, PptxResourcePageFailure> {
    let invalid = || {
        failure(
            PptxPageFailureCode::InputInvalid,
            "editor object ancestry binding",
        )
    };
    let mut result = Vec::<EditorPageObjectInfo>::new();
    let mut covered = 0;
    for layer in &page.page().page.info.layers {
        cancelled(check)?;
        let surface = source.surfaces.get(&layer.part).ok_or_else(invalid)?;
        let mut needed: BTreeSet<_> = page
            .objects()
            .iter()
            .filter(|o| o.part == layer.part)
            .map(|o| o.native_id)
            .collect();
        covered += needed.len();
        if needed.is_empty() {
            continue;
        }
        // Source objects are preorder. A single reverse pass retains every
        // needed ancestor without multiplying work by hierarchy depth.
        for object in surface.objects.iter().rev() {
            cancelled(check)?;
            if needed.contains(&object.native_id)
                && let Some(parent) = object.parent_group
            {
                needed.insert(parent);
            }
        }
        let mut indices = BTreeMap::new();
        for object in &surface.objects {
            cancelled(check)?;
            if !needed.contains(&object.native_id) {
                continue;
            }
            if result.len() == 8192 {
                return Err(failure(
                    PptxPageFailureCode::LimitExceeded,
                    "editor object catalog",
                ));
            }
            let parent = object
                .parent_group
                .map(|id| {
                    let index: u32 = *indices.get(&id).ok_or_else(invalid)?;
                    if result[index as usize].kind != SourceObjectKind::Group {
                        return Err(invalid());
                    }
                    Ok(index)
                })
                .transpose()?;
            let reference = SourceObjectRef {
                part: layer.part.clone(),
                native_id: object.native_id,
            };
            if indices
                .insert(object.native_id, result.len() as u32)
                .is_some()
            {
                return Err(invalid());
            }
            result.push(EditorPageObjectInfo {
                object_id: model_id(&reference)?,
                object: reference,
                name: object.name.clone(),
                kind: object.kind,
                surface: surface.kind,
                parent,
            });
        }
        if indices.len() != needed.len() {
            return Err(invalid());
        }
    }
    if covered != page.objects().len() {
        return Err(invalid());
    }
    Ok(result)
}
