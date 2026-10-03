use mo_text::{TextError, backend::TextBackend};
use wasm_bindgen::prelude::*;

#[wasm_bindgen(typescript_custom_section)]
const COMPONENT_INTERFACE: &str = r#"
export interface ShapingComponent {
    shapeBatch(font: Uint8Array, frame: Uint32Array): Uint32Array;
    outlineBatch(font: Uint8Array, frame: Uint32Array): Uint32Array;
    measureBatch(font: Uint8Array, frame: Uint32Array): Uint32Array;
    caretBatch(font: Uint8Array, frame: Uint32Array): Uint32Array;
    registerFont(font: Uint8Array): number;
    unregisterFont(handle: number): void;
    shapeRegistered(handle: number, frame: Uint32Array): Uint32Array;
    measureRegistered(handle: number, frame: Uint32Array): Uint32Array;
    outlineRegistered(handle: number, frame: Uint32Array): Uint32Array;
    caretRegistered(handle: number, frame: Uint32Array): Uint32Array;
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
    #[wasm_bindgen(method,catch,js_name=caretBatch)]
    fn invoke_carets(
        this: &ShapingComponent,
        font: &[u8],
        frame: &[u32],
    ) -> Result<Vec<u32>, JsValue>;
    #[wasm_bindgen(method,catch,js_name=registerFont)]
    fn register(this: &ShapingComponent, font: &[u8]) -> Result<u32, JsValue>;
    #[wasm_bindgen(method,catch,js_name=unregisterFont)]
    fn unregister(this: &ShapingComponent, handle: u32) -> Result<(), JsValue>;
    #[wasm_bindgen(method,catch,js_name=shapeRegistered)]
    fn registered_shape(
        this: &ShapingComponent,
        handle: u32,
        frame: &[u32],
    ) -> Result<Vec<u32>, JsValue>;
    #[wasm_bindgen(method,catch,js_name=measureRegistered)]
    fn registered_metrics(
        this: &ShapingComponent,
        handle: u32,
        frame: &[u32],
    ) -> Result<Vec<u32>, JsValue>;
    #[wasm_bindgen(method,catch,js_name=outlineRegistered)]
    fn registered_outlines(
        this: &ShapingComponent,
        handle: u32,
        frame: &[u32],
    ) -> Result<Vec<u32>, JsValue>;
    #[wasm_bindgen(method,catch,js_name=caretRegistered)]
    fn registered_carets(
        this: &ShapingComponent,
        handle: u32,
        frame: &[u32],
    ) -> Result<Vec<u32>, JsValue>;
    #[wasm_bindgen(method,catch,js_name=invalidate)]
    fn discard(this: &ShapingComponent) -> Result<(), JsValue>;
}
pub(crate) struct Backend<'a>(pub(crate) &'a ShapingComponent);
impl TextBackend for Backend<'_> {
    fn supports_font_residency(&self) -> bool {
        true
    }
    fn register_font(&mut self, font: &[u8]) -> Result<u32, TextError> {
        self.0
            .register(font)
            .map_err(|_| TextError::Host("WASM font registration failed"))
    }
    fn unregister_font(&mut self, handle: u32) -> Result<(), TextError> {
        self.0
            .unregister(handle)
            .map_err(|_| TextError::Host("WASM font release failed"))
    }
    fn shape_registered(&mut self, handle: u32, frame: &[u32]) -> Result<Vec<u32>, TextError> {
        self.0
            .registered_shape(handle, frame)
            .map_err(|_| TextError::Host("WASM registered shaping failed"))
    }
    fn measure_registered(&mut self, handle: u32, frame: &[u32]) -> Result<Vec<u32>, TextError> {
        self.0
            .registered_metrics(handle, frame)
            .map_err(|_| TextError::Host("WASM registered metrics failed"))
    }
    fn outline_registered(&mut self, handle: u32, frame: &[u32]) -> Result<Vec<u32>, TextError> {
        self.0
            .registered_outlines(handle, frame)
            .map_err(|_| TextError::Host("WASM registered outlines failed"))
    }
    fn caret_registered(&mut self, handle: u32, frame: &[u32]) -> Result<Vec<u32>, TextError> {
        self.0
            .registered_carets(handle, frame)
            .map_err(|_| TextError::Host("WASM registered carets failed"))
    }
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
    fn caret_batch(&mut self, font: &[u8], frame: &[u32]) -> Result<Vec<u32>, TextError> {
        self.0
            .invoke_carets(font, frame)
            .map_err(|_| TextError::Host("WASM caret component call failed"))
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
pub fn font_carets(request: &str, font: &[u8], component: &ShapingComponent) -> String {
    mo_kernel_api::font_carets_json(request, font, &mut Backend(component), &|| false)
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
