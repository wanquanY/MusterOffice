//! Compact append expands into one ordinary, revision-fenced transaction.
use super::*;
use crate::{Failure, FailureCode};
use mo_common::{Digest, OperationId, RequestId};
use mo_presentation_edit::{Operation, OperationEntry, Transaction};
use mo_presentation_model::{Document, Resource};

pub(crate) fn transaction(
    base: &Document,
    revision: Digest,
    request_id: RequestId,
    slides: &[SlideContent],
    resources: &[Resource],
    check: &dyn Fn() -> bool,
) -> Result<Transaction, Failure> {
    let fragment = PresentationContent {
        id: base.id.clone(),
        title: base.title.clone(),
        page_size: base.page_size,
        slides: slides.to_vec(),
        resources: resources.to_vec(),
    }
    .to_document(check)?;
    let mut operations = vec![];
    let mut add = |operation| -> Result<(), Failure> {
        if check() {
            return Err(Failure::new(FailureCode::Cancelled, "append cancelled"));
        }
        operations.push(OperationEntry {
            operation_id: OperationId::new(format!("append:{}", operations.len()))
                .expect("bounded generated ID"),
            operation,
        });
        Ok(())
    };
    for resource in fragment.resources.values() {
        if let Some(existing) = base.resources.get(&resource.id) {
            if existing != resource {
                return Err(Failure::new(
                    FailureCode::InputInvalid,
                    "append cannot replace an existing resource",
                ));
            }
        } else {
            add(Operation::AttachResource {
                resource: resource.clone(),
            })?;
        }
    }
    for (offset, id) in fragment.slide_order.iter().enumerate() {
        let mut slide = fragment.slides[id].clone();
        let objects = std::mem::take(&mut slide.objects);
        add(Operation::InsertSlide {
            slide,
            index: (base.slide_order.len() + offset)
                .try_into()
                .map_err(|_| Failure::new(FailureCode::LimitExceeded, "slide index"))?,
        })?;
        for (index, id) in objects.iter().enumerate() {
            add(Operation::InsertObject {
                object: fragment.objects[id].clone(),
                index: index as u32,
            })?;
        }
    }
    Ok(Transaction {
        document_id: base.id.clone(),
        request_id,
        base_revision: revision,
        operations,
    })
}
