use crate::{image_decode, raster, text};
use wasm_bindgen::prelude::*;
/// Host-owned immutable editor page. free() releases all retained geometry.
#[wasm_bindgen]
#[derive(Default)]
pub struct EditorPageSession {
    inner: mo_kernel_api::EditorPageSession,
}
#[wasm_bindgen]
impl EditorPageSession {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        Self::default()
    }
    pub fn prepare(
        &mut self,
        request: &str,
        material: &[u8],
        fonts: &[u8],
        decoder: &image_decode::ImageDecoderComponent,
        shaping: &text::ShapingComponent,
        raster: &raster::RasterComponent,
    ) -> raster::RenderedRaster {
        let (metadata, pixels) = self.inner.dispatch_json(
            request,
            material,
            fonts,
            Some(mo_kernel_api::EditorPageComponents::All(
                mo_kernel_api::EditorPageBackends {
                    decoder: &mut image_decode::Backend(decoder),
                    text: &mut text::Backend(shaping),
                    raster: &mut raster::Backend(raster),
                },
            )),
            &|| false,
        );
        raster::RenderedRaster { metadata, pixels }
    }
    pub fn command(&mut self, request: &str) -> String {
        self.inner
            .dispatch_json(request, &[], &[], None, &|| false)
            .0
    }
    /// Metadata-only discovery. No component imports or retained-view mutation.
    pub fn inspect(&mut self, request: &str, material: &[u8]) -> String {
        self.inner
            .dispatch_json(request, material, &[], None, &|| false)
            .0
    }
    pub fn pick(&mut self, request: &str, raster: &raster::RasterComponent) -> String {
        self.inner
            .dispatch_json(
                request,
                &[],
                &[],
                Some(mo_kernel_api::EditorPageComponents::Raster(
                    &mut raster::Backend(raster),
                )),
                &|| false,
            )
            .0
    }
}
