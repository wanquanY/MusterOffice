//! Admission for the independent authored portion of a retained document.
//! Whole-document validation still runs atomically after the transaction.
use crate::Operation;
use mo_common::*;
use mo_presentation_model::*;

pub(crate) fn admits(d: &Document, b: &SourceBindings, op: &Operation) -> bool {
    let object = |id: &ObjectId| d.objects.contains_key(id) && !b.objects.contains_key(id);
    let slide = |id: &SlideId| d.slides.contains_key(id) && !b.slides.contains_key(id);
    let position = |index: u32| index as usize >= b.slides.len();
    let parent = |id: &ContainerId| match id {
        ContainerId::Slide(id) => slide(id),
        ContainerId::Group(id) => object(id),
        _ => false,
    };
    match op {
        Operation::InsertSlide { slide, index } => {
            !b.slides.contains_key(&slide.id) && slide.layout.is_none() && position(*index)
        }
        Operation::MoveSlide { slide: id, index } => slide(id) && position(*index),
        Operation::DuplicateSlide { source, index, .. } => slide(source) && position(*index),
        Operation::SetSlideName { slide: id, .. }
        | Operation::SetSlideBackground { slide: id, .. }
        | Operation::DeleteSlide { slide: id, .. }
        | Operation::SetTimeline { slide: id, .. }
        | Operation::SetPresentationSequence { slide: id, .. } => slide(id),
        Operation::SetLayout { slide: id, layout } => slide(id) && layout.is_none(),
        Operation::InsertObject { object, .. } => {
            !b.objects.contains_key(&object.id)
                && parent(&object.parent)
                && !matches!(object.content, ObjectContent::RetainedSource { .. })
        }
        Operation::MoveObject {
            object: id,
            parent: target,
            ..
        } => object(id) && parent(target),
        Operation::DeleteObject { object: id, .. }
        | Operation::SetTransform { object: id, .. }
        | Operation::SetAppearance { object: id, .. }
        | Operation::SetFill { object: id, .. }
        | Operation::SetStroke { object: id, .. }
        | Operation::SetPicture { object: id, .. }
        | Operation::SetPictureCrop { object: id, .. }
        | Operation::SetGeometry { object: id, .. }
        | Operation::SetAccessibility { object: id, .. }
        | Operation::SetText { object: id, .. }
        | Operation::SpliceText { object: id, .. }
        | Operation::EditTable { object: id, .. } => object(id),
        Operation::EnsureResource { resource } | Operation::AttachResource { resource } => {
            resource.id != b.resource
        }
        Operation::DetachResource { resource } => *resource != b.resource,
        Operation::PutFont { font } => font.resource != b.resource,
        _ => false,
    }
}
