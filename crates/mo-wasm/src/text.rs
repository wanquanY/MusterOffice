use mo_text::{TextError, backend::TextBackend};
use wasm_bindgen::prelude::*;

#[wasm_bindgen(typescript_custom_section)]
const COMPONENT_INTERFACE: &str = r#"
export interface ShapingComponent {
    shapeBatch(font: Uint8Array, frame: Uint32Array): Uint32Array;
    outlineBatch(font: Uint8Array, frame: Uint32Array): Uint32Array;
    measureBatch(font: Uint8Array, frame: Uint32Array): Uint32Array;
    invalidate(): void;
}
"#;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(typescript_type = "ShapingComponent")]
    pub type ShapingComponent;
    #[wasm_bindgen(method,catch,js_name=shapeBatch)]
    fn invoke(this: &ShapingComponent, font: &[u8], frame: &[u32]) -> Result<Vec<u32>, JsValue>;
    #[wasm_bindgen(method,catch,js_name=measureBatch)]
    fn invoke_metrics(
        this: &ShapingComponent,
        font: &[u8],
        frame: &[u32],
    ) -> Result<Vec<u32>, JsValue>;
    #[wasm_bindgen(method,catch,js_name=outlineBatch)]
    fn invoke_outlines(
        this: &ShapingComponent,
        font: &[u8],
        frame: &[u32],
    ) -> Result<Vec<u32>, JsValue>;
    #[wasm_bindgen(method,catch,js_name=invalidate)]
    fn discard(this: &ShapingComponent) -> Result<(), JsValue>;
}
pub(crate) struct Backend<'a>(pub(crate) &'a ShapingComponent);
impl TextBackend for Backend<'_> {
    fn shape_batch(&mut self, font: &[u8], frame: &[u32]) -> Result<Vec<u32>, TextError> {
        self.0
            .invoke(font, frame)
            .map_err(|_| TextError::Host("WASM component call failed"))
    }
    fn measure_batch(&mut self, font: &[u8], frame: &[u32]) -> Result<Vec<u32>, TextError> {
        self.0
            .invoke_metrics(font, frame)
            .map_err(|_| TextError::Host("WASM metric component call failed"))
    }
    fn outline_batch(&mut self, font: &[u8], frame: &[u32]) -> Result<Vec<u32>, TextError> {
        self.0
            .invoke_outlines(font, frame)
            .map_err(|_| TextError::Host("WASM outline component call failed"))
    }
    fn invalidate(&mut self) {
        let _ = self.0.discard();
    }
}
/// The host owns a separately instantiated shaper and worker lifetime. The Rust
/// engine validates resources, ranges, quantization and every returned glyph.
#[wasm_bindgen]
pub fn shape_text(request: &str, font: &[u8], component: &ShapingComponent) -> String {
    mo_kernel_api::shape_text_json(request, font, &mut Backend(component), &|| false)
}
#[wasm_bindgen]
pub fn shape_cascade(request: &str, bundle: &[u8], component: &ShapingComponent) -> String {
    mo_kernel_api::shape_cascade_json(request, bundle, &mut Backend(component), &|| false)
}

#[wasm_bindgen]
pub fn itemize_paragraph(request: &str) -> String {
    mo_kernel_api::itemize_paragraph_json(request)
}
#[wasm_bindgen]
pub fn shape_paragraph(request: &str, bundle: &[u8], component: &ShapingComponent) -> String {
    mo_kernel_api::shape_paragraph_json(request, bundle, &mut Backend(component), &|| false)
}
#[wasm_bindgen]
pub fn shape_lines(request: &str, bundle: &[u8], component: &ShapingComponent) -> String {
    mo_kernel_api::shape_lines_json(request, bundle, &mut Backend(component), &|| false)
}

#[wasm_bindgen]
pub fn measure_font(request: &str, font: &[u8], component: &ShapingComponent) -> String {
    mo_kernel_api::measure_font_json(request, font, &mut Backend(component), &|| false)
}

#[wasm_bindgen]
pub fn layout_lines(request: &str, bundle: &[u8], component: &ShapingComponent) -> String {
    mo_kernel_api::layout_lines_json(request, bundle, &mut Backend(component), &|| false)
}

#[wasm_bindgen]
pub fn layout_paragraph(request: &str, bundle: &[u8], component: &ShapingComponent) -> String {
    mo_kernel_api::layout_paragraph_json(request, bundle, &mut Backend(component), &|| false)
}

#[wasm_bindgen]
pub fn outline_font(request: &str, font: &[u8], component: &ShapingComponent) -> String {
    mo_kernel_api::outline_font_json(request, font, &mut Backend(component), &|| false)
}

#[wasm_bindgen]
pub fn paragraph_paths(request: &str, bundle: &[u8], component: &ShapingComponent) -> String {
    mo_kernel_api::paragraph_paths_json(request, bundle, &mut Backend(component), &|| false)
}
