//! Source/font preparation shared by static pages and native timeline sampling.
use super::*;
pub(crate) struct PreparedInput<'a> {
    pub package: Package<&'a [u8]>,
    pub index: mo_pptx::source::SourceIndex,
    pub manifest: Option<PreparedManifest<'a, 'a>>,
}
pub(crate) fn prepare_input<'a>(
    request: &'a PptxResourcePageRequest,
    source: &'a [u8],
    fonts: &'a [u8],
    check: &dyn Fn() -> bool,
) -> Result<PreparedInput<'a>, PptxResourcePageFailure> {
    if source.len() > MAX_INLINE_RESOURCE_BYTES || fonts.len() > MAX_INLINE_FONT_BYTES {
        return Err(request_failure(
            PptxPageFailureCode::LimitExceeded,
            "resource page input bytes".into(),
        ));
    }
    if request.fonts.is_none() && !fonts.is_empty() {
        return Err(request_failure(
            PptxPageFailureCode::InputInvalid,
            "font bytes without a manifest".into(),
        ));
    }
    let limits = inline_limits();
    let source_error = |e: PptxError| PptxResourcePageFailure::Source {
        error: pptx_page::failure(SourcePageError::Source(e)),
    };
    let package = Package::open(source, source.len() as u64, limits.package, check)
        .map_err(PptxError::from)
        .map_err(source_error)?;
    let index = inspect_source(&package, limits, check).map_err(source_error)?;
    if index.source_sha256 != request.page.expected_source_sha256 {
        return Err(PptxResourcePageFailure::Source {
            error: pptx_page::failure(SourcePageError::SourceConflict),
        });
    }
    let manifest = request
        .fonts
        .as_ref()
        .map(|m| PreparedManifest::load(m, fonts, ManifestLimits::default(), check))
        .transpose()
        .map_err(|e| PptxResourcePageFailure::Fonts {
            error: crate::text::shape_failure(e),
        })?;
    Ok(PreparedInput {
        package,
        index,
        manifest,
    })
}
