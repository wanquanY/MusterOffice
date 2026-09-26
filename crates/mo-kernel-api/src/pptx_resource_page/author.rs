//! One semantic author plan, resource binding and font preparation per batch.
//! No OPC writer or source XML reader participates in this rendering path.
use super::*;
use mo_common::ResourceId;
use mo_pptx::{AuthorPlan, ResourceData, Resources, source::images::AuthorImages};
use std::collections::BTreeMap;
#[cfg(test)]
mod tests;

struct Bundle<'a>(BTreeMap<ResourceId, &'a [u8]>);
impl Resources for Bundle<'_> {
    fn open(&self, id: &ResourceId) -> Result<ResourceData<'_>, PptxError> {
        let bytes = self
            .0
            .get(id)
            .ok_or_else(|| PptxError::ResourceRequired(id.clone()))?;
        Ok(ResourceData {
            reader: bytes,
            byte_length: bytes.len() as u64,
        })
    }
}
fn invalid(message: &'static str) -> PptxResourcePageFailure {
    request_failure(PptxPageFailureCode::InputInvalid, message.into())
}
fn bundle<'a>(
    request: &AuthorResourceDocumentRequest,
    plan: &AuthorPlan<'_>,
    bytes: &'a [u8],
    check: &dyn Fn() -> bool,
) -> Result<Bundle<'a>, PptxResourcePageFailure> {
    let mut out = BTreeMap::new();
    let mut offset = 0u64;
    if request.resources.len() != plan.bindings().images.len() {
        return Err(invalid("author image coverage"));
    }
    for range in &request.resources {
        if check() {
            return Err(page_failure(SourcePageError::Source(PptxError::Cancelled)));
        }
        let end = offset
            .checked_add(range.byte_length.get())
            .filter(|n| *n <= bytes.len() as u64)
            .ok_or_else(|| invalid("author image range"))?;
        if range.offset.get() != offset
            || !plan.bindings().images.contains_key(&range.id)
            || out
                .insert(range.id.clone(), &bytes[offset as usize..end as usize])
                .is_some()
        {
            return Err(invalid("author image range binding"));
        }
        offset = end;
    }
    if offset != bytes.len() as u64 {
        return Err(invalid("unbound author image bytes"));
    }
    Ok(Bundle(out))
}

/// Streams successful pages in order; a domain failure emits one structured
/// error frame and ends the batch. Callback errors stop computation immediately.
/// The host still owns process termination, output storage and publication.
pub fn render_author_resource_document<E>(
    request: &AuthorResourceDocumentRequest,
    resources: &[u8],
    fonts: &[u8],
    backends: PptxResourcePageBackends<'_>,
    check: &dyn Fn() -> bool,
    emit: &mut impl FnMut(PptxResourcePageRasterResponse, &[u8]) -> Result<(), E>,
) -> Result<(), E> {
    let prepared = (|| {
        if request.pages.is_empty()
            || request.pages.len() > 256
            || resources.len() > MAX_INLINE_RESOURCE_BYTES
            || fonts.len() > MAX_INLINE_FONT_BYTES
        {
            return Err(request_failure(
                PptxPageFailureCode::LimitExceeded,
                "author document batch budget".into(),
            ));
        }
        if request.fonts.is_none() && !fonts.is_empty() {
            return Err(invalid("font bytes without a manifest"));
        }
        let plan = AuthorPlan::new(
            &request.document,
            &request.defaults,
            Default::default(),
            check,
        )
        .map_err(|e| page_failure(SourcePageError::Source(e)))?;
        if request
            .pages
            .iter()
            .any(|p| &p.page.expected_source_sha256 != plan.identity())
        {
            return Err(page_failure(SourcePageError::SourceConflict));
        }
        let bundle = bundle(request, &plan, resources, check)?;
        let manifest = request
            .fonts
            .as_ref()
            .map(|m| PreparedManifest::load(m, fonts, ManifestLimits::default(), check))
            .transpose()
            .map_err(|e| PptxResourcePageFailure::Fonts {
                error: crate::text::shape_failure(e),
            })?;
        Ok((plan, bundle, manifest))
    })();
    let (plan, bundle, manifest) = match prepared {
        Ok(prepared) => prepared,
        Err(error) => {
            return emit(
                PptxResourcePageRasterResponse::Error {
                    error: Box::new(error),
                },
                &[],
            );
        }
    };
    let images = match AuthorImages::new(&plan, &bundle, check) {
        Ok(images) => images,
        Err(error) => {
            return emit(
                PptxResourcePageRasterResponse::Error {
                    error: Box::new(page_failure(SourcePageError::Source(error))),
                },
                &[],
            );
        }
    };
    super::batch::render_resource_document_plan(
        &request.pages,
        super::batch::ResourceDocumentInputs {
            images: &images,
            index: plan.declarations(),
            manifest: manifest.as_ref(),
        },
        backends,
        check,
        emit,
    )
}
