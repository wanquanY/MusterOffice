use mo_presentation_compile::source_resource_page::SourceResourcePageImage;
use mo_presentation_delivery::{
    Content, DeliveryError, PreviewFonts, PreviewRenderer, PreviewRequest, RendererIdentity,
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
        source: Content<'_>,
        fonts: PreviewFonts<'_>,
        check: &dyn Fn() -> bool,
        emit: &mut dyn FnMut(usize, SourceResourcePageImage) -> Result<(), DeliveryError>,
    ) -> Result<(), DeliveryError> {
        if requests.is_empty() {
            return Ok(());
        }
        let limits = mo_presentation_delivery::DeliveryLimits::default();
        if requests.len() > limits.max_pages
            || source.byte_length > limits.max_asset_bytes
            || fonts.content.byte_length > limits.max_font_bytes
        {
            return Err(DeliveryError::Limit("native export preview input"));
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
        let prepared = mo_kernel_api::prepare_pptx_resource_document_inputs(
            &requests[0].page.expected_source_sha256,
            fonts.manifest,
            source.reader,
            source.byte_length,
            &font_bytes,
            check,
        )
        .map_err(preview_failure)?;
        let mut text = mo_harfbuzz_sys::NativeShaper::default();
        let mut decoder = mo_skia_sys::NativeRaster;
        let mut raster = mo_skia_sys::NativeRaster;
        for (ordinal, request) in requests.iter().enumerate() {
            if check() {
                return Err(DeliveryError::Cancelled);
            }
            let (response, pixels) = mo_kernel_api::render_pptx_resource_document_page(
                &prepared,
                &request.page,
                request.image_source,
                request.sampling,
                mo_kernel_api::PptxResourcePageBackends {
                    decoder: &mut decoder,
                    text: Some(&mut text),
                    raster: &mut raster,
                },
                check,
            );
            if check() {
                return Err(DeliveryError::Cancelled);
            }
            let image = match response {
                mo_kernel_api::PptxResourcePageRasterResponse::Rendered { info } => {
                    SourceResourcePageImage {
                        info: *info,
                        pixels,
                    }
                }
                mo_kernel_api::PptxResourcePageRasterResponse::Error { error } => {
                    return Err(preview_failure(*error));
                }
            };
            emit(ordinal, image)?;
        }
        Ok(())
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
