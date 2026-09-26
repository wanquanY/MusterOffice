use mo_presentation_compile::source_resource_page::{
    self, ResourcePageOptions, SourceResourcePageImage, TextPageContext,
};
use mo_presentation_delivery::{
    Content, DeliveryError, PreviewRenderer, PreviewRequest, RendererIdentity,
};
use mo_text::manifest::PreparedManifest;

pub struct Renderer {
    pub identity: RendererIdentity,
}
impl PreviewRenderer for Renderer {
    fn identity(&self) -> RendererIdentity {
        self.identity.clone()
    }
    fn render(
        &mut self,
        request: &PreviewRequest,
        source: Content<'_>,
        fonts: Content<'_>,
        check: &dyn Fn() -> bool,
    ) -> Result<SourceResourcePageImage, DeliveryError> {
        let limits = mo_presentation_delivery::DeliveryLimits::default();
        if source.byte_length > limits.max_asset_bytes || fonts.byte_length > limits.max_font_bytes
        {
            return Err(DeliveryError::Limit("native export preview input"));
        }
        if request.fonts.is_none() && fonts.byte_length != 0 {
            return Err(DeliveryError::Invalid("font bytes without manifest"));
        }
        let package =
            mo_opc::Package::open(source.reader, source.byte_length, Default::default(), check)
                .map_err(mo_pptx::PptxError::from)?;
        let index = mo_pptx::source::inspect_source(&package, Default::default(), check)?;
        if package.sha256() != &request.page.expected_source_sha256 {
            return Err(DeliveryError::Invalid("preview source digest"));
        }
        let mut font_bytes = vec![0; fonts.byte_length as usize];
        for (i, chunk) in font_bytes.chunks_mut(65536).enumerate() {
            if check() {
                return Err(DeliveryError::Cancelled);
            }
            fonts.reader.read_exact_at(chunk, (i * 65536) as u64)?;
        }
        let manifest = request
            .fonts
            .as_ref()
            .map(|m| PreparedManifest::load(m, &font_bytes, Default::default(), check))
            .transpose()
            .map_err(|error| {
                preview_failure(mo_kernel_api::PptxResourcePageFailure::Fonts {
                    error: error.into(),
                })
            })?;
        let mut text = mo_harfbuzz_sys::NativeShaper::default();
        let mut decoder = mo_skia_sys::NativeRaster;
        let mut raster = mo_skia_sys::NativeRaster;
        source_resource_page::prepare(
            &package,
            &index,
            &request.page,
            &mut decoder,
            manifest.as_ref().map(|manifest| TextPageContext {
                manifest,
                backend: &mut text,
            }),
            ResourcePageOptions {
                selection: request.image_source,
                sampling: request.sampling,
                text_limits: Default::default(),
            },
            check,
        )
        .and_then(|page| page.render(&mut raster, check))
        .map_err(|error| preview_failure(error.into()))
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
