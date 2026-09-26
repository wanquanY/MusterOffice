use mo_image::{DecoderReply, ImageDecoder, ImageError};
use wasm_bindgen::prelude::*;
#[wasm_bindgen(typescript_custom_section)]
const INTERFACE: &str = r#"
export interface ImageDecoderComponent {
    decodeImage(encoded: Uint8Array): {status: number; words: Uint32Array; pixels: Uint8Array};
    invalidate(): void;
}
"#;
#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(typescript_type = "ImageDecoderComponent")]
    pub type ImageDecoderComponent;
    type Reply;
    #[wasm_bindgen(method, catch, js_name = decodeImage)]
    fn invoke(this: &ImageDecoderComponent, encoded: &[u8]) -> Result<Reply, JsValue>;
    #[wasm_bindgen(method, catch, js_name = invalidate)]
    fn discard(this: &ImageDecoderComponent) -> Result<(), JsValue>;
    #[wasm_bindgen(method, getter, catch, js_name = status)]
    fn status(this: &Reply) -> Result<f64, JsValue>;
    #[wasm_bindgen(method, getter, catch, js_name = words)]
    fn words(this: &Reply) -> Result<JsValue, JsValue>;
    #[wasm_bindgen(method, getter, catch, js_name = pixels)]
    fn pixels(this: &Reply) -> Result<JsValue, JsValue>;
    #[wasm_bindgen(js_name = Uint32Array)]
    type Words;
    #[wasm_bindgen(js_name = Uint8Array)]
    type Pixels;
    #[wasm_bindgen(method, getter, catch, js_name = length)]
    fn word_length(this: &Words) -> Result<f64, JsValue>;
    #[wasm_bindgen(method, getter, catch, js_name = length)]
    fn pixel_length(this: &Pixels) -> Result<f64, JsValue>;
    #[wasm_bindgen(js_namespace = Uint32Array, js_name = "prototype.set.call", catch)]
    fn copy_words(destination: &mut [u32], source: &Words) -> Result<(), JsValue>;
    #[wasm_bindgen(js_namespace = Uint8Array, js_name = "prototype.set.call", catch)]
    fn copy_pixels(destination: &mut [u8], source: &Pixels) -> Result<(), JsValue>;
}
pub(crate) struct Backend<'a>(pub &'a ImageDecoderComponent);
impl ImageDecoder for Backend<'_> {
    fn decode(&mut self, encoded: &[u8]) -> Result<DecoderReply, ImageError> {
        let result = self
            .0
            .invoke(encoded)
            .map_err(|_| ImageError::Host("WASM decoder call"))?;
        let invalid = |_| ImageError::ComponentInvalid("WASM decode reply");
        let status = result.status().map_err(invalid)?;
        if !status.is_finite()
            || status.fract() != 0.0
            || !(0.0..=5.0).contains(&status)
            || status == 4.0
        {
            return Err(ImageError::ComponentInvalid("WASM decode status"));
        }
        let source = result
            .words()
            .map_err(invalid)?
            .dyn_into::<Words>()
            .map_err(invalid)?;
        if source.word_length().map_err(invalid)? != 9.0 {
            return Err(ImageError::ComponentInvalid("WASM decode metadata length"));
        }
        let mut words = [0; 9];
        copy_words(&mut words, &source).map_err(invalid)?;
        let source = result
            .pixels()
            .map_err(invalid)?
            .dyn_into::<Pixels>()
            .map_err(invalid)?;
        let expected = if status == 0.0 { words[8] as usize } else { 0 };
        if expected > mo_image::MAX_PIXEL_BYTES
            || source.pixel_length().map_err(invalid)? != expected as f64
        {
            return Err(ImageError::ComponentInvalid("WASM decode pixel length"));
        }
        let mut pixels = Vec::new();
        pixels
            .try_reserve_exact(expected)
            .map_err(|_| ImageError::Host("WASM decode allocation"))?;
        pixels.resize(expected, 0);
        copy_pixels(&mut pixels, &source).map_err(invalid)?;
        Ok(DecoderReply {
            status: status as u32,
            words,
            pixels,
        })
    }
    fn invalidate(&mut self) {
        let _ = self.0.discard();
    }
}
#[wasm_bindgen]
pub struct DecodedImage {
    metadata: String,
    pixels: Vec<u8>,
}
#[wasm_bindgen]
impl DecodedImage {
    #[wasm_bindgen(getter)]
    pub fn metadata(&self) -> String {
        self.metadata.clone()
    }
    pub fn take_pixels(self) -> Vec<u8> {
        self.pixels
    }
}
#[wasm_bindgen]
pub fn decode_image(
    request: &str,
    encoded: &[u8],
    component: &ImageDecoderComponent,
) -> DecodedImage {
    let (metadata, pixels) =
        mo_kernel_api::decode_image_json(request, encoded, &mut Backend(component), &|| false);
    DecodedImage { metadata, pixels }
}
