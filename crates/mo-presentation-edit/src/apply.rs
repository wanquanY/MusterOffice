use crate::{AnchorMap, DeletePolicy, EditError, Operation};
use mo_common::*;
use mo_presentation_model::*;
use std::collections::{BTreeMap, BTreeSet, VecDeque};

pub(crate) fn apply(
    document: &mut Document,
    operation: &Operation,
    anchors: &mut Vec<AnchorMap>,
) -> Result<(), EditError> {
    source_operation(document, operation)?;
    match operation {
        Operation::SetTimeline { slide, timeline } => {
            if !document.slides.contains_key(slide) {
                return Err(EditError::input("timeline slide does not exist"));
            }
            if let Some(timeline) = timeline {
                document.timelines.insert(slide.clone(), timeline.clone());
            } else {
                document.timelines.remove(slide);
            }
        }
        Operation::SetTitle { title } => document.title = title.clone(),
        Operation::InsertSlide { slide, index } => {
            if document.slides.contains_key(&slide.id) {
                return Err(EditError::input("slide already exists"));
            }
            insert(&mut document.slide_order, *index, slide.id.clone())?;
            document.slides.insert(slide.id.clone(), slide.clone());
        }
        Operation::DeleteSlide { slide, policy } => {
            let record = document
                .slides
                .get(slide)
                .ok_or_else(|| EditError::input("slide does not exist"))?;
            if *policy == DeletePolicy::RejectDependencies && !record.objects.is_empty() {
                return Err(EditError::ReferenceConflict(
                    "slide still owns objects".into(),
                ));
            }
            let roots = record.objects.clone();
            delete_objects(document, roots, *policy)?;
            document.slides.remove(slide);
            document.timelines.remove(slide);
            document.slide_order.retain(|id| id != slide);
        }
        Operation::MoveSlide { slide, index } => {
            if !document.slides.contains_key(slide) {
                return Err(EditError::input("slide does not exist"));
            }
            document.slide_order.retain(|id| id != slide);
            insert(&mut document.slide_order, *index, slide.clone())?;
        }
        Operation::SetLayout { slide, layout } => {
            document
                .slides
                .get_mut(slide)
                .ok_or_else(|| EditError::input("slide does not exist"))?
                .layout = layout.clone();
        }
        Operation::PutTheme { theme } => {
            document.themes.insert(theme.id.clone(), theme.clone());
        }
        Operation::PutMaster { master } => {
            document.masters.insert(master.id.clone(), master.clone());
        }
        Operation::PutLayout { layout } => {
            document.layouts.insert(layout.id.clone(), layout.clone());
        }
        Operation::AttachResource { resource } => {
            document
                .resources
                .insert(resource.id.clone(), resource.clone());
        }
        Operation::DetachResource { resource } => {
            if document.resources.remove(resource).is_none() {
                return Err(EditError::input("resource does not exist"));
            }
        }
        Operation::PutFont { font } => {
            document.fonts.insert(font.id.clone(), font.clone());
        }
        Operation::InsertObject { object, index } => {
            if document.objects.contains_key(&object.id) {
                return Err(EditError::input("object already exists"));
            }
            insert(
                container_mut(document, &object.parent)?,
                *index,
                object.id.clone(),
            )?;
            document.objects.insert(object.id.clone(), object.clone());
        }
        Operation::DeleteObject { object, policy } => {
            if !document.objects.contains_key(object) {
                return Err(EditError::input("object does not exist"));
            }
            delete_objects(document, vec![object.clone()], *policy)?;
        }
        Operation::MoveObject {
            object,
            parent,
            index,
            transform,
        } => {
            let old_parent = object_mut(document, object)?.parent.clone();
            container_mut(document, &old_parent)?.retain(|id| id != object);
            insert(container_mut(document, parent)?, *index, object.clone())?;
            let object = object_mut(document, object)?;
            object.parent = parent.clone();
            object.transform = Some(*transform);
        }
        Operation::SetTransform { object, transform } => {
            object_mut(document, object)?.transform = Some(*transform)
        }
        Operation::SetAppearance { object, appearance } => {
            object_mut(document, object)?.appearance = appearance.clone()
        }
        Operation::SetAccessibility {
            object,
            accessibility,
        } => object_mut(document, object)?.accessibility = accessibility.clone(),
        Operation::SetText { object, text } => match &mut object_mut(document, object)?.content {
            ObjectContent::Shape { text: body, .. } => *body = Some(text.clone()),
            _ => return Err(EditError::input("object has no shape text body")),
        },
        Operation::SpliceText {
            object,
            paragraph,
            run,
            start,
            delete,
            insert,
        } => {
            let content = &mut object_mut(document, object)?.content;
            let (text, prefix) = match content {
                ObjectContent::Shape {
                    text: Some(body), ..
                } => {
                    let p = body
                        .paragraphs
                        .iter_mut()
                        .find(|p| &p.id == paragraph)
                        .ok_or_else(|| EditError::input("paragraph does not exist in object"))?;
                    let index = p
                        .runs
                        .iter()
                        .position(|r| &r.id == run)
                        .ok_or_else(|| EditError::input("run does not exist in paragraph"))?;
                    let prefix =
                        scalar_prefix(p.runs[..index].iter().map(|r| r.content.scalar_len()))?;
                    let InlineContent::Text { text } = &mut p.runs[index].content else {
                        return Err(EditError::input("splice requires a text run"));
                    };
                    (text, prefix)
                }
                ObjectContent::RetainedSource { paragraphs, .. } => {
                    let p = paragraphs
                        .iter_mut()
                        .find(|p| &p.id == paragraph)
                        .ok_or_else(|| EditError::input("paragraph does not exist in object"))?;
                    let index = p
                        .runs
                        .iter()
                        .position(|r| &r.id == run)
                        .ok_or_else(|| EditError::input("run does not exist in paragraph"))?;
                    let prefix =
                        scalar_prefix(p.runs[..index].iter().map(RetainedTextRun::scalar_len))?;
                    if p.runs[index].kind != RetainedRunKind::Text {
                        return Err(EditError::input("splice requires a text run"));
                    }
                    (&mut p.runs[index].text, prefix)
                }
                _ => return Err(EditError::input("object has no text body")),
            };
            let end = start
                .checked_add(*delete)
                .ok_or_else(|| EditError::input("splice range overflow"))?;
            let scalar_count = text.chars().count();
            if end as usize > scalar_count {
                return Err(EditError::input("splice range exceeds text run"));
            }
            let from = scalar_to_byte(text, *start as usize);
            let to = scalar_to_byte(text, end as usize);
            let inserted = u32::try_from(insert.chars().count())
                .map_err(|_| EditError::input("insertion exceeds scalar range"))?;
            let start = prefix
                .checked_add(*start)
                .ok_or_else(|| EditError::input("anchor range overflow"))?;
            text.replace_range(from..to, insert);
            anchors.push(AnchorMap {
                paragraph: paragraph.clone(),
                start,
                deleted: *delete,
                inserted,
            });
        }
    }
    Ok(())
}

fn scalar_to_byte(text: &str, offset: usize) -> usize {
    text.char_indices()
        .nth(offset)
        .map_or(text.len(), |(i, _)| i)
}

fn insert<T>(list: &mut Vec<T>, index: u32, value: T) -> Result<(), EditError> {
    let index = index as usize;
    if index > list.len() {
        return Err(EditError::input("insertion index exceeds container length"));
    }
    list.insert(index, value);
    Ok(())
}

fn object_mut<'a>(document: &'a mut Document, id: &ObjectId) -> Result<&'a mut Object, EditError> {
    document
        .objects
        .get_mut(id)
        .ok_or_else(|| EditError::input("object does not exist"))
}

fn container_mut<'a>(
    document: &'a mut Document,
    owner: &ContainerId,
) -> Result<&'a mut Vec<ObjectId>, EditError> {
    match owner {
        ContainerId::Slide(id) => document.slides.get_mut(id).map(|s| &mut s.objects),
        ContainerId::Master(id) => document.masters.get_mut(id).map(|s| &mut s.objects),
        ContainerId::Layout(id) => document.layouts.get_mut(id).map(|s| &mut s.objects),
        ContainerId::Group(id) => document
            .objects
            .get_mut(id)
            .and_then(|o| match &mut o.content {
                ObjectContent::Group { children, .. } => Some(children),
                _ => None,
            }),
    }
    .ok_or_else(|| EditError::input("owning container does not exist or is not a group"))
}

fn delete_objects(
    document: &mut Document,
    roots: Vec<ObjectId>,
    policy: DeletePolicy,
) -> Result<(), EditError> {
    let mut dependents: BTreeMap<ObjectId, Vec<ObjectId>> = BTreeMap::new();
    for (id, object) in &document.objects {
        if let ObjectContent::Group { children, .. } = &object.content {
            dependents
                .entry(id.clone())
                .or_default()
                .extend(children.iter().cloned());
        }
        if let ObjectContent::Connector { start, end } = &object.content {
            for endpoint in [start, end] {
                if let ConnectorEndpoint::Attached { object, .. } = endpoint {
                    dependents
                        .entry(object.clone())
                        .or_default()
                        .push(id.clone());
                }
            }
        }
    }
    let mut remove: BTreeSet<_> = roots.iter().cloned().collect();
    let mut pending = VecDeque::from(roots);
    while let Some(id) = pending.pop_front() {
        if let Some(refs) = dependents.get(&id) {
            for dependent in refs {
                if remove.contains(dependent) {
                    continue;
                }
                if policy == DeletePolicy::RejectDependencies {
                    return Err(EditError::ReferenceConflict(format!(
                        "{dependent} depends on {id}"
                    )));
                }
                remove.insert(dependent.clone());
                pending.push_back(dependent.clone());
            }
        }
    }
    crate::timing::delete_references(document, &remove, policy)?;
    for id in &remove {
        document.objects.remove(id);
    }
    for slide in document.slides.values_mut() {
        slide.objects.retain(|id| !remove.contains(id));
    }
    for master in document.masters.values_mut() {
        master.objects.retain(|id| !remove.contains(id));
    }
    for layout in document.layouts.values_mut() {
        layout.objects.retain(|id| !remove.contains(id));
    }
    for object in document.objects.values_mut() {
        if let ObjectContent::Group { children, .. } = &mut object.content {
            children.retain(|id| !remove.contains(id));
        }
    }
    Ok(())
}

fn scalar_prefix(mut lengths: impl Iterator<Item = usize>) -> Result<u32, EditError> {
    lengths.try_fold(0_u32, |n, length| {
        n.checked_add(u32::try_from(length).map_err(|_| EditError::input("run is too long"))?)
            .ok_or_else(|| EditError::input("paragraph is too long"))
    })
}

// Source-backed documents share the transaction engine. Unsupported changes
// cannot cross the immutable provenance boundary and silently discard content.
fn source_operation(d: &Document, operation: &Operation) -> Result<(), EditError> {
    let Some(bindings) = &d.source_bindings else {
        return Ok(());
    };
    let constraint = match operation {
        Operation::SetTransform { object, .. } => {
            bindings
                .objects
                .get(object)
                .ok_or_else(|| EditError::input("source object binding missing"))?
                .transform_constraint
        }
        Operation::SpliceText { object, run, .. } => {
            bindings
                .objects
                .get(object)
                .and_then(|o| o.runs.get(run))
                .ok_or_else(|| EditError::input("source run binding missing"))?
                .constraint
        }
        _ => {
            return Err(EditError::input(
                "operation requires coordinated native preservation; source bindings are immutable",
            ));
        }
    };
    if let Some(c) = constraint {
        return Err(EditError::input(format!(
            "native edit requires coordinated preservation: {c:?}"
        )));
    }
    Ok(())
}
