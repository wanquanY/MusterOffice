use mo_presentation_compile::source_resource_page::SourceResourcePageImage;
use mo_presentation_delivery::{
    DeliveryError, PreviewFonts, PreviewInput, PreviewRenderer, PreviewRequest, RendererIdentity,
};

pub struct Renderer {
    pub identity: RendererIdentity,
}
impl PreviewRenderer for Renderer {
    fn identity(&self) -> RendererIdentity {
        self.identity.clone()
    }
    fn render_pages(
        &mut self,
        requests: &[PreviewRequest],
        input: PreviewInput<'_>,
        fonts: PreviewFonts<'_>,
        check: &dyn Fn() -> bool,
        emit: &mut dyn FnMut(usize, SourceResourcePageImage) -> Result<(), DeliveryError>,
    ) -> Result<(), DeliveryError> {
        if requests.is_empty() {
            return Ok(());
        }
        let limits = mo_presentation_delivery::DeliveryLimits::default();
        if requests.len() > limits.max_pages || fonts.content.byte_length > limits.max_font_bytes {
            return Err(DeliveryError::Limit("native export preview input"));
        }
        if fonts.manifest.is_none() && fonts.content.byte_length != 0 {
            return Err(DeliveryError::Invalid("font bytes without a manifest"));
        }
        if match &input {
            PreviewInput::Source(s) | PreviewInput::Retained { source: s, .. } => {
                s.byte_length > limits.max_asset_bytes
            }
            _ => false,
        } {
            return Err(DeliveryError::Limit("source preview input"));
        }
        let mut font_bytes = vec![0; fonts.content.byte_length as usize];
        for (i, chunk) in font_bytes.chunks_mut(65536).enumerate() {
            if check() {
                return Err(DeliveryError::Cancelled);
            }
            fonts
                .content
                .reader
                .read_exact_at(chunk, (i * 65536) as u64)?;
        }
        let manifest = fonts
            .manifest
            .map(|m| {
                mo_text::manifest::PreparedManifest::load(m, &font_bytes, Default::default(), check)
            })
            .transpose()
            .map_err(|error| {
                preview_failure(mo_kernel_api::PptxResourcePageFailure::Fonts {
                    error: error.into(),
                })
            })?;
        use mo_pptx::source::images::{AuthorImages, ImageInput, PackageImages, SourceImages};
        let mut render = |images: &dyn ImageInput, index: &mo_pptx::source::SourceIndex| {
            let mut ordinal = 0;
            mo_kernel_api::render_resource_document_images(
                requests,
                mo_kernel_api::ResourceDocumentInputs {
                    images,
                    index,
                    manifest: manifest.as_ref(),
                },
                mo_kernel_api::PptxResourcePageBackends {
                    decoder: &mut mo_skia_sys::NativeRaster,
                    text: Some(&mut mo_harfbuzz_sys::NativeShaper::default()),
                    raster: &mut mo_skia_sys::NativeRaster,
                },
                check,
                &mut |result| {
                    emit(ordinal, result.map_err(preview_failure)?)?;
                    ordinal += 1;
                    Ok(())
                },
            )
        };
        match input {
            PreviewInput::Author { plan, resources } => render(
                &AuthorImages::new(plan, resources, check)?,
                plan.declarations(),
            ),
            PreviewInput::Retained { plan, source } => {
                let package = mo_opc::Package::open(
                    source.reader,
                    source.byte_length,
                    Default::default(),
                    check,
                )
                .map_err(mo_pptx::PptxError::from)?;
                render(&SourceImages::new(plan, &package)?, plan.declarations())
            }
            PreviewInput::Source(source) => {
                let package = mo_opc::Package::open(
                    source.reader,
                    source.byte_length,
                    Default::default(),
                    check,
                )
                .map_err(mo_pptx::PptxError::from)?;
                let index = mo_pptx::source::inspect_source(&package, Default::default(), check)?;
                render(&PackageImages(&package), &index)
            }
        }
    }
}

fn preview_failure(error: mo_kernel_api::PptxResourcePageFailure) -> DeliveryError {
    let diagnostic = match serde_json::to_value(error) {
        Ok(value) => value,
        Err(_) => return DeliveryError::Serialization,
    };
    DeliveryError::Preview {
        message: "native page rendering failed".into(),
        diagnostic: Some(Box::new(diagnostic)),
    }
}
