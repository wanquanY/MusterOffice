use mo_raster::{BackendReply, RasterBackend, RasterError};
use wasm_bindgen::prelude::*;
#[wasm_bindgen(typescript_custom_section)]
const INTERFACE: &str = r#"
export interface RasterReply { status: number; pixels: Uint8Array; }
export interface RasterComponent {
    raster(frame: Uint32Array): {status: number; pixels: Uint8Array};
    pick?(frame: Uint32Array): {status: number; words: Uint32Array};
    rasterImages?(frame: Uint32Array, images: Uint8Array): {status: number; pixels: Uint8Array};
    invalidate(): void;
}
"#;
#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(typescript_type = "RasterComponent")]
    pub type RasterComponent;
    #[wasm_bindgen(typescript_type = "RasterReply")]
    pub type Reply;
    #[wasm_bindgen(method, catch, js_name = raster)]
    fn invoke(this: &RasterComponent, frame: &[u32]) -> Result<Reply, JsValue>;
    #[wasm_bindgen(method, catch, js_name = pick)]
    fn invoke_pick(this: &RasterComponent, frame: &[u32]) -> Result<Reply, JsValue>;
    #[wasm_bindgen(method, getter, catch, js_name = words)]
    fn words(this: &Reply) -> Result<JsValue, JsValue>;
    #[wasm_bindgen(js_name = Uint32Array)]
    type PickWords;
    #[wasm_bindgen(method, getter, catch, js_name = length)]
    fn word_length(this: &PickWords) -> Result<f64, JsValue>;
    #[wasm_bindgen(js_namespace = Uint32Array, js_name = "prototype.set.call", catch)]
    fn copy_words(destination: &mut [u32], source: &PickWords) -> Result<(), JsValue>;
    #[wasm_bindgen(method, catch, js_name = rasterImages)]
    fn invoke_images(
        this: &RasterComponent,
        frame: &[u32],
        images: &[u8],
    ) -> Result<Reply, JsValue>;
    #[wasm_bindgen(method, getter, catch, js_name = status)]
    fn status(this: &Reply) -> Result<f64, JsValue>;
    #[wasm_bindgen(method, getter, catch, js_name = pixels)]
    fn pixels(this: &Reply) -> Result<JsValue, JsValue>;
    #[wasm_bindgen(method, catch, js_name = invalidate)]
    fn discard(this: &RasterComponent) -> Result<(), JsValue>;
    #[wasm_bindgen(js_name = Uint8Array)]
    type PixelBytes;
    #[wasm_bindgen(method, getter, catch, js_name = length)]
    fn length(this: &PixelBytes) -> Result<f64, JsValue>;
    // Copy into an already bounded Rust allocation. A Vec return value would
    // allocate from the host-reported length before Rust can validate it.
    #[wasm_bindgen(js_namespace = Uint8Array, js_name = "prototype.set.call", catch)]
    fn copy_pixels(destination: &mut [u8], source: &PixelBytes) -> Result<(), JsValue>;
}
pub(crate) struct Backend<'a>(pub(crate) &'a RasterComponent);
impl Backend<'_> {
    fn call(&mut self, frame: &[u32], images: Option<&[u8]>) -> Result<BackendReply, RasterError> {
        let r = match images {
            Some(images) => self.0.invoke_images(frame, images),
            None => self.0.invoke(frame),
        }
        .map_err(|_| RasterError::Host("WASM raster component call failed"))?;
        read_reply(&r, frame)
    }
}
pub(crate) fn read_reply(r: &Reply, frame: &[u32]) -> Result<BackendReply, RasterError> {
    let status = r
        .status()
        .map_err(|_| RasterError::ComponentInvalid("WASM status"))?;
    if !status.is_finite() || status.fract() != 0.0 || !(0.0..=4.0).contains(&status) {
        return Err(RasterError::ComponentInvalid("WASM status"));
    }
    let source = r
        .pixels()
        .map_err(|_| RasterError::ComponentInvalid("WASM pixels"))?
        .dyn_into::<PixelBytes>()
        .map_err(|_| RasterError::ComponentInvalid("WASM pixel type"))?;
    let expected = if status == 0.0 {
        u64::from(frame[2]) * u64::from(frame[3]) * 4
    } else {
        0
    };
    let length = source
        .length()
        .map_err(|_| RasterError::ComponentInvalid("WASM pixel length"))?;
    if expected > mo_raster::MAX_PIXEL_BYTES as u64 || length != expected as f64 {
        return Err(RasterError::ComponentInvalid("WASM pixel length"));
    }
    let mut pixels = Vec::new();
    pixels
        .try_reserve_exact(expected as usize)
        .map_err(|_| RasterError::Host("WASM pixels allocation"))?;
    pixels.resize(expected as usize, 0);
    copy_pixels(&mut pixels, &source)
        .map_err(|_| RasterError::ComponentInvalid("WASM pixel copy"))?;
    Ok(BackendReply {
        status: status as u32,
        pixels,
    })
}

impl RasterBackend for Backend<'_> {
    fn pick(&mut self, frame: &[u32]) -> Result<mo_raster::picking::PickingReply, RasterError> {
        let r = self
            .0
            .invoke_pick(frame)
            .map_err(|_| RasterError::Host("WASM picking component call"))?;
        let status = r
            .status()
            .map_err(|_| RasterError::ComponentInvalid("WASM picking status"))?;
        if !status.is_finite() || status.fract() != 0.0 || !(0.0..=4.0).contains(&status) {
            return Err(RasterError::ComponentInvalid("WASM picking status"));
        }
        let source = r
            .words()
            .map_err(|_| RasterError::ComponentInvalid("WASM picking words"))?
            .dyn_into::<PickWords>()
            .map_err(|_| RasterError::ComponentInvalid("WASM picking word type"))?;
        let count = source
            .word_length()
            .map_err(|_| RasterError::ComponentInvalid("WASM picking length"))?;
        let expected = if status == 0.0 {
            7 + frame[9] as usize
                * ((frame[8] as usize).div_ceil(32) * 2 + (frame[7] as usize + 1).div_ceil(32))
        } else {
            0
        };
        if expected > mo_raster::picking::MAX_PICK_REPLY_WORDS || count != expected as f64 {
            return Err(RasterError::ComponentInvalid("WASM picking length"));
        }
        let mut words = Vec::new();
        words
            .try_reserve_exact(expected)
            .map_err(|_| RasterError::Host("WASM picking allocation"))?;
        words.resize(expected, 0);
        copy_words(&mut words, &source)
            .map_err(|_| RasterError::ComponentInvalid("WASM picking copy"))?;
        Ok(mo_raster::picking::PickingReply {
            status: status as u32,
            words,
        })
    }
    fn raster(&mut self, frame: &[u32]) -> Result<BackendReply, RasterError> {
        self.call(frame, None)
    }
    fn raster_images(&mut self, frame: &[u32], images: &[u8]) -> Result<BackendReply, RasterError> {
        self.call(frame, Some(images))
    }
    fn invalidate(&mut self) {
        let _ = self.0.discard();
    }
}
#[wasm_bindgen]
pub struct RenderedRaster {
    pub(crate) metadata: String,
    pub(crate) pixels: Vec<u8>,
}
#[wasm_bindgen]
impl RenderedRaster {
    #[wasm_bindgen(getter)]
    pub fn metadata(&self) -> String {
        self.metadata.clone()
    }
    pub fn take_pixels(self) -> Vec<u8> {
        self.pixels
    }
}
#[wasm_bindgen]
pub fn render_paths(request: &str, component: &RasterComponent) -> RenderedRaster {
    let (metadata, pixels) =
        mo_kernel_api::render_paths_json(request, &mut Backend(component), &|| false);
    RenderedRaster { metadata, pixels }
}
#[wasm_bindgen]
pub fn render_scene(request: &str, component: &RasterComponent) -> RenderedRaster {
    let (metadata, pixels) =
        mo_kernel_api::render_scene_json(request, &mut Backend(component), &|| false);
    RenderedRaster { metadata, pixels }
}

#[wasm_bindgen]
pub fn render_page(request: &str, component: &RasterComponent) -> RenderedRaster {
    let (metadata, pixels) =
        mo_kernel_api::render_page_json(request, &mut Backend(component), &|| false);
    RenderedRaster { metadata, pixels }
}

#[wasm_bindgen]
pub fn render_pptx_page(
    request: &str,
    source: &[u8],
    component: &RasterComponent,
) -> RenderedRaster {
    let (metadata, pixels) =
        mo_kernel_api::render_pptx_page_json(request, source, &mut Backend(component), &|| false);
    RenderedRaster { metadata, pixels }
}

/// The host supplies independent component instances and explicit font bytes.
/// Both wrappers delegate to the same Rust text/page computation as Native.
#[wasm_bindgen]
pub fn render_pptx_text_page(
    request: &str,
    source: &[u8],
    fonts: &[u8],
    shaping: &crate::text::ShapingComponent,
    raster: &RasterComponent,
) -> RenderedRaster {
    let (metadata, pixels) = mo_kernel_api::render_pptx_text_page_json(
        request,
        source,
        fonts,
        &mut crate::text::Backend(shaping),
        &mut Backend(raster),
        &|| false,
    );
    RenderedRaster { metadata, pixels }
}

#[wasm_bindgen]
pub fn render_image_paths(
    request: &str,
    images: &[u8],
    component: &RasterComponent,
) -> RenderedRaster {
    let (metadata, pixels) =
        mo_kernel_api::render_image_paths_json(request, images, &mut Backend(component), &|| false);
    RenderedRaster { metadata, pixels }
}

#[wasm_bindgen]
pub fn render_image_scene(
    request: &str,
    images: &[u8],
    component: &RasterComponent,
) -> RenderedRaster {
    let (metadata, pixels) =
        mo_kernel_api::render_image_scene_json(request, images, &mut Backend(component), &|| false);
    RenderedRaster { metadata, pixels }
}

/// Decode, shape and render a native page through the common Rust boundary.
#[wasm_bindgen]
pub fn render_pptx_resource_page(
    request: &str,
    source: &[u8],
    fonts: &[u8],
    decoder: &crate::image_decode::ImageDecoderComponent,
    shaping: &crate::text::ShapingComponent,
    raster: &RasterComponent,
) -> RenderedRaster {
    let (metadata, pixels) = mo_kernel_api::render_pptx_resource_page_json(
        request,
        source,
        fonts,
        mo_kernel_api::PptxResourcePageBackends {
            decoder: &mut crate::image_decode::Backend(decoder),
            text: Some(&mut crate::text::Backend(shaping)),
            raster: &mut Backend(raster),
        },
        &|| false,
    );
    RenderedRaster { metadata, pixels }
}

#[wasm_bindgen]
pub fn render_playback_page(request: &str, component: &RasterComponent) -> RenderedRaster {
    let (metadata, pixels) =
        mo_kernel_api::render_playback_page_json(request, &mut Backend(component), &|| false);
    RenderedRaster { metadata, pixels }
}

#[wasm_bindgen]
pub fn render_pptx_playback_page(
    request: &str,
    source: &[u8],
    fonts: &[u8],
    decoder: &crate::image_decode::ImageDecoderComponent,
    shaping: &crate::text::ShapingComponent,
    raster: &RasterComponent,
) -> RenderedRaster {
    let (metadata, pixels) = mo_kernel_api::render_pptx_playback_page_json(
        request,
        source,
        fonts,
        mo_kernel_api::PptxResourcePageBackends {
            decoder: &mut crate::image_decode::Backend(decoder),
            text: Some(&mut crate::text::Backend(shaping)),
            raster: &mut Backend(raster),
        },
        &|| false,
    );
    RenderedRaster { metadata, pixels }
}
