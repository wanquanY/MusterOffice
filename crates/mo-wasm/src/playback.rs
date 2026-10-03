use crate::raster::{Backend, RasterComponent, RenderedRaster};
use wasm_bindgen::prelude::*;
mod prepared;

/// Explicitly owned sampler. Run in the host's worker; free() releases the plan.
/// No global registry, clocks, filesystem or implicit component allocation.
#[wasm_bindgen]
#[derive(Default)]
pub struct PlaybackSession {
    inner: mo_kernel_api::PlaybackSession,
}

/// Source/font bytes and shaping/decoder components are consumed only by
/// prepare; resize re-admits source images without fonts. Host owns scheduling, result fencing, worker termination and free().
#[wasm_bindgen]
#[derive(Default)]
pub struct PptxPlaybackSession {
    inner: mo_kernel_api::PptxPlaybackSession,
}
#[wasm_bindgen]
impl PptxPlaybackSession {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        Self::default()
    }
    pub fn prepare(
        &mut self,
        request: &str,
        source: &[u8],
        fonts: &[u8],
        decoder: &crate::image_decode::ImageDecoderComponent,
        shaping: &crate::text::ShapingComponent,
    ) -> String {
        self.inner
            .dispatch_json(
                request,
                source,
                fonts,
                Some(mo_kernel_api::PptxPlaybackResources {
                    decoder: &mut crate::image_decode::Backend(decoder),
                    text: Some(&mut crate::text::Backend(shaping)),
                }),
                None,
                &|| false,
            )
            .0
    }
    pub fn resize(
        &mut self,
        request: &str,
        source: &[u8],
        decoder: &crate::image_decode::ImageDecoderComponent,
    ) -> String {
        self.inner
            .dispatch_json(
                request,
                source,
                &[],
                Some(mo_kernel_api::PptxPlaybackResources {
                    decoder: &mut crate::image_decode::Backend(decoder),
                    text: None,
                }),
                None,
                &|| false,
            )
            .0
    }
    pub fn command(&mut self, request: &str) -> String {
        self.inner
            .dispatch_json(request, &[], &[], None, None, &|| false)
            .0
    }
    pub fn render(&mut self, request: &str, component: &RasterComponent) -> RenderedRaster {
        let (metadata, pixels) = self.inner.dispatch_json(
            request,
            &[],
            &[],
            None,
            Some(&mut Backend(component)),
            &|| false,
        );
        RenderedRaster { metadata, pixels }
    }
}
#[wasm_bindgen]
impl PlaybackSession {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        Self::default()
    }
    pub fn command(&mut self, request: &str) -> String {
        self.inner.dispatch_json(request, None, &|| false).0
    }
    pub fn render(&mut self, request: &str, component: &RasterComponent) -> RenderedRaster {
        let (metadata, pixels) =
            self.inner
                .dispatch_json(request, Some(&mut Backend(component)), &|| false);
        RenderedRaster { metadata, pixels }
    }
}
