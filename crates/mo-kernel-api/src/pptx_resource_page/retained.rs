use super::*;
use mo_pptx::source::{document::SourcePlan, images::SourceImages};

pub fn render_retained_resource_document<E>(
    request: &RetainedResourceDocumentRequest,
    source: &[u8],
    fonts: &[u8],
    backends: PptxResourcePageBackends<'_>,
    check: &dyn Fn() -> bool,
    emit: &mut impl FnMut(PptxResourcePageRasterResponse, &[u8]) -> Result<(), E>,
) -> Result<(), E> {
    let prepared = (|| {
        if request.pages.is_empty()
            || request.pages.len() > 256
            || source.len() > MAX_INLINE_RESOURCE_BYTES
            || fonts.len() > MAX_INLINE_FONT_BYTES
        {
            return Err(request_failure(
                PptxPageFailureCode::LimitExceeded,
                "retained document batch budget".into(),
            ));
        }
        if request.fonts.is_none() && !fonts.is_empty() {
            return Err(request_failure(
                PptxPageFailureCode::InputInvalid,
                "font bytes without a manifest".into(),
            ));
        }
        let package = Package::open(source, source.len() as u64, inline_limits().package, check)
            .map_err(|e| page_failure(SourcePageError::Source(e.into())))?;
        let plan = SourcePlan::new(&request.document, &package, inline_limits(), check)
            .map_err(|e| page_failure(SourcePageError::Source(e)))?;
        if request
            .pages
            .iter()
            .any(|p| &p.page.expected_source_sha256 != plan.identity())
        {
            return Err(page_failure(SourcePageError::SourceConflict));
        }
        let manifest = request
            .fonts
            .as_ref()
            .map(|m| PreparedManifest::load(m, fonts, ManifestLimits::default(), check))
            .transpose()
            .map_err(|e| PptxResourcePageFailure::Fonts {
                error: crate::text::shape_failure(e),
            })?;
        Ok((package, plan, manifest))
    })();
    let (package, plan, manifest) = match prepared {
        Ok(p) => p,
        Err(error) => {
            return emit(
                PptxResourcePageRasterResponse::Error {
                    error: Box::new(error),
                },
                &[],
            );
        }
    };
    let images = match SourceImages::new(&plan, &package) {
        Ok(images) => images,
        Err(e) => {
            return emit(
                PptxResourcePageRasterResponse::Error {
                    error: Box::new(page_failure(SourcePageError::Source(e))),
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
