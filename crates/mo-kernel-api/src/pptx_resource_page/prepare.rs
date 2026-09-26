//! Source/font preparation shared by static pages and native timeline sampling.
use super::*;
pub struct PreparedPptxResourceDocument<'a, R: mo_opc::ReaderAt> {
    pub(crate) package: Package<R>,
    pub(crate) index: mo_pptx::source::SourceIndex,
    pub(crate) manifest: Option<PreparedManifest<'a, 'a>>,
    pub(crate) font_manifest_digest: Digest,
}
pub(crate) fn prepare_input<'a>(
    request: &'a PptxResourcePageRequest,
    source: &'a [u8],
    fonts: &'a [u8],
    check: &dyn Fn() -> bool,
) -> Result<PreparedPptxResourceDocument<'a, &'a [u8]>, PptxResourcePageFailure> {
    prepare_pptx_resource_document(request, source, source.len() as u64, fonts, check)
}
/// Prepares immutable document and font identities once for any number of pages.
pub fn prepare_pptx_resource_document<'a, R: mo_opc::ReaderAt>(
    request: &'a PptxResourcePageRequest,
    source: R,
    source_length: u64,
    fonts: &'a [u8],
    check: &dyn Fn() -> bool,
) -> Result<PreparedPptxResourceDocument<'a, R>, PptxResourcePageFailure> {
    prepare_pptx_resource_document_inputs(
        &request.page.expected_source_sha256,
        request.fonts.as_ref(),
        source,
        source_length,
        fonts,
        check,
    )
}

/// Batch callers bind the manifest once, independently of per-page options.
pub fn prepare_pptx_resource_document_inputs<'a, R: mo_opc::ReaderAt>(
    expected_source: &Digest,
    manifest: Option<&'a FontManifest>,
    source: R,
    source_length: u64,
    fonts: &'a [u8],
    check: &dyn Fn() -> bool,
) -> Result<PreparedPptxResourceDocument<'a, R>, PptxResourcePageFailure> {
    if source_length > MAX_INLINE_RESOURCE_BYTES as u64 || fonts.len() > MAX_INLINE_FONT_BYTES {
        return Err(request_failure(
            PptxPageFailureCode::LimitExceeded,
            "resource page input bytes".into(),
        ));
    }
    if manifest.is_none() && !fonts.is_empty() {
        return Err(request_failure(
            PptxPageFailureCode::InputInvalid,
            "font bytes without a manifest".into(),
        ));
    }
    let limits = inline_limits();
    let source_error = |e: PptxError| PptxResourcePageFailure::Source {
        error: pptx_page::failure(SourcePageError::Source(e)),
    };
    let package = Package::open(source, source_length, limits.package, check)
        .map_err(PptxError::from)
        .map_err(source_error)?;
    let index = inspect_source(&package, limits, check).map_err(source_error)?;
    if &index.source_sha256 != expected_source {
        return Err(PptxResourcePageFailure::Source {
            error: pptx_page::failure(SourcePageError::SourceConflict),
        });
    }
    let font_manifest_digest = manifest_digest(manifest)?;
    let manifest = manifest
        .map(|m| PreparedManifest::load(m, fonts, ManifestLimits::default(), check))
        .transpose()
        .map_err(|e| PptxResourcePageFailure::Fonts {
            error: crate::text::shape_failure(e),
        })?;
    Ok(PreparedPptxResourceDocument {
        font_manifest_digest,
        package,
        index,
        manifest,
    })
}

fn manifest_digest(manifest: Option<&FontManifest>) -> Result<Digest, PptxResourcePageFailure> {
    mo_common::digest("musteroffice.preview-font-manifest/1", &manifest).map_err(|_| {
        request_failure(
            PptxPageFailureCode::InputInvalid,
            "font manifest identity".into(),
        )
    })
}
impl<R: mo_opc::ReaderAt> PreparedPptxResourceDocument<'_, R> {
    pub(crate) fn check_request(
        &self,
        request: &PptxResourcePageRequest,
    ) -> Result<(), PptxResourcePageFailure> {
        if self.index.source_sha256 != request.page.expected_source_sha256 {
            return Err(PptxResourcePageFailure::Source {
                error: pptx_page::failure(SourcePageError::SourceConflict),
            });
        }
        if self.font_manifest_digest != manifest_digest(request.fonts.as_ref())? {
            return Err(request_failure(
                PptxPageFailureCode::InputInvalid,
                "document font manifest changed".into(),
            ));
        }
        Ok(())
    }
}
