//! Discover the exact plan accepted by prepare without rasterizing or changing
//! the current view. Author resource payloads are verified later at preparation.
use super::*;
use mo_common::SlideId;
use mo_opc::Package;
use mo_pptx::{
    AuthorPlan, PptxError,
    source::{SourceIndex, SurfaceKind, document::SourcePlan, inspect_source},
};
use mo_presentation_model::Document;
use std::collections::BTreeMap;
#[cfg(test)]
mod tests;

pub(super) fn inspect(
    input: &EditorDocumentInput,
    material: &[u8],
    check: &dyn Fn() -> bool,
) -> Result<EditorDocumentInfo, PptxResourcePageFailure> {
    match input {
        EditorDocumentInput::Author { document, defaults } => {
            if !material.is_empty() {
                return Err(failure(
                    PptxPageFailureCode::InputInvalid,
                    "material supplied to author document inspection",
                ));
            }
            let plan = AuthorPlan::new(document, defaults, Default::default(), check)
                .map_err(source_error)?;
            let ids = plan
                .bindings()
                .slides
                .iter()
                .map(|s| (s.part.as_str(), &s.id))
                .collect();
            catalog(plan.declarations(), Some(document), ids, check)
        }
        EditorDocumentInput::Retained { document } => {
            let package = package(material, check)?;
            let plan = SourcePlan::new(
                document,
                &package,
                crate::pptx_source::inline_limits(),
                check,
            )
            .map_err(source_error)?;
            let ids = document
                .source_bindings
                .as_ref()
                .expect("validated source plan")
                .slides
                .iter()
                .map(|(id, part)| (part.as_str(), id))
                .collect();
            catalog(plan.declarations(), Some(document), ids, check)
        }
        EditorDocumentInput::Pptx {} => {
            let package = package(material, check)?;
            let index = inspect_source(&package, crate::pptx_source::inline_limits(), check)
                .map_err(source_error)?;
            catalog(&index, None, BTreeMap::new(), check)
        }
    }
}
fn package<'a>(
    material: &'a [u8],
    check: &dyn Fn() -> bool,
) -> Result<Package<&'a [u8]>, PptxResourcePageFailure> {
    Package::open(
        material,
        material.len() as u64,
        crate::pptx_source::inline_limits().package,
        check,
    )
    .map_err(PptxError::from)
    .map_err(source_error)
}
fn catalog(
    index: &SourceIndex,
    document: Option<&Document>,
    ids: BTreeMap<&str, &SlideId>,
    check: &dyn Fn() -> bool,
) -> Result<EditorDocumentInfo, PptxResourcePageFailure> {
    let invalid = || failure(PptxPageFailureCode::InputInvalid, "editor slide identity");
    if document.is_some_and(|d| {
        d.slide_order.len() != index.slides.len() || ids.len() != index.slides.len()
    }) {
        return Err(invalid());
    }
    let mut slides = Vec::with_capacity(index.slides.len());
    for (i, s) in index.slides.iter().enumerate() {
        cancelled(check)?;
        let surface = index.surfaces.get(&s.part).ok_or_else(invalid)?;
        if surface.kind != SurfaceKind::Slide {
            return Err(invalid());
        }
        let slide_id = document
            .map(|d| {
                let id = *ids.get(s.part.as_str()).ok_or_else(invalid)?;
                if id != &d.slide_order[i] {
                    return Err(invalid());
                }
                Ok(id.clone())
            })
            .transpose()?;
        slides.push(EditorDocumentSlide {
            slide: s.part.clone(),
            native_id: s.native_id,
            slide_id,
            name: surface.name.clone(),
            hidden: surface.hidden,
        });
    }
    let model = document
        .map(|d| -> Result<_, PptxResourcePageFailure> {
            Ok(EditorDocumentIdentity {
                id: d.id.clone(),
                semantic_digest: d.semantic_digest().map_err(|_| invalid())?,
            })
        })
        .transpose()?;
    cancelled(check)?;
    Ok(EditorDocumentInfo {
        source_sha256: index.source_sha256.clone(),
        model,
        page_size: index.page_size,
        slides,
    })
}
