use super::*;
pub struct ResourceDocumentInputs<'a> {
    pub images: &'a dyn mo_pptx::source::images::ImageInput,
    pub index: &'a mo_pptx::source::SourceIndex,
    pub manifest: Option<&'a PreparedManifest<'a, 'a>>,
}
pub fn render_resource_document_images<E>(
    requests: &[source_resource_page::ResourcePageRequest],
    inputs: ResourceDocumentInputs<'_>,
    mut backends: PptxResourcePageBackends<'_>,
    check: &dyn Fn() -> bool,
    emit: &mut impl FnMut(
        Result<source_resource_page::SourceResourcePageImage, PptxResourcePageFailure>,
    ) -> Result<(), E>,
) -> Result<(), E> {
    if requests.is_empty() || requests.len() > 256 {
        return emit(Err(request_failure(
            PptxPageFailureCode::LimitExceeded,
            "document batch page count".into(),
        )));
    }
    for request in requests {
        let result = (|| {
            let text = match inputs.manifest {
                Some(manifest) => Some(TextPageContext {
                    manifest,
                    backend: match &mut backends.text {
                        Some(backend) => &mut **backend,
                        None => {
                            return Err(request_failure(
                                PptxPageFailureCode::ResourceRequired,
                                "text component required".into(),
                            ));
                        }
                    },
                }),
                None => None,
            };
            source_resource_page::prepare_input(
                inputs.images,
                inputs.index,
                &request.page,
                backends.decoder,
                text,
                ResourcePageOptions {
                    selection: request.image_source,
                    sampling: request.sampling,
                    text_limits: Default::default(),
                },
                check,
            )
            .and_then(|page| page.render(backends.raster, check))
            .map_err(page_failure)
        })();
        let failed = result.is_err();
        emit(result)?;
        if failed {
            break;
        }
    }
    Ok(())
}

/// Framed transports borrow pixels only while serializing the current page.
pub fn render_resource_document_plan<E>(
    requests: &[source_resource_page::ResourcePageRequest],
    inputs: ResourceDocumentInputs<'_>,
    backends: PptxResourcePageBackends<'_>,
    check: &dyn Fn() -> bool,
    emit: &mut impl FnMut(PptxResourcePageRasterResponse, &[u8]) -> Result<(), E>,
) -> Result<(), E> {
    render_resource_document_images(
        requests,
        inputs,
        backends,
        check,
        &mut |result| match result {
            Ok(image) => emit(
                PptxResourcePageRasterResponse::Rendered {
                    info: Box::new(image.info),
                },
                &image.pixels,
            ),
            Err(error) => emit(
                PptxResourcePageRasterResponse::Error {
                    error: Box::new(error),
                },
                &[],
            ),
        },
    )
}
