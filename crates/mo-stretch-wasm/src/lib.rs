//! Thin binding to a separately instantiated, host-verified DSP WASM component.
use mo_video_audio::{AudioError, Result, StretchEngine, StretchInfo};
use wasm_bindgen::prelude::*;

#[wasm_bindgen(typescript_custom_section)]
const INTERFACE: &str = r#"
export interface StretchComponent {
  readonly channels: number;
  readonly sampleRate: number;
  readonly inputLatency: number;
  readonly outputLatency: number;
  begin(input: Float32Array, rate: number): void;
  process(input: Float32Array, output: Float32Array): void;
  dispose(): void;
}
"#;
#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(typescript_type = "StretchComponent")]
    pub type StretchComponent;
    #[wasm_bindgen(method, getter, catch, js_name = channels)]
    fn channels(this: &StretchComponent) -> std::result::Result<f64, JsValue>;
    #[wasm_bindgen(method, getter, catch, js_name = sampleRate)]
    fn sample_rate(this: &StretchComponent) -> std::result::Result<f64, JsValue>;
    #[wasm_bindgen(method, getter, catch, js_name = inputLatency)]
    fn input_latency(this: &StretchComponent) -> std::result::Result<f64, JsValue>;
    #[wasm_bindgen(method, getter, catch, js_name = outputLatency)]
    fn output_latency(this: &StretchComponent) -> std::result::Result<f64, JsValue>;
    #[wasm_bindgen(method, catch, js_name = begin)]
    fn invoke_begin(
        this: &StretchComponent,
        input: &[f32],
        rate: f64,
    ) -> std::result::Result<(), JsValue>;
    #[wasm_bindgen(method, catch, js_name = process)]
    fn invoke_process(
        this: &StretchComponent,
        input: &[f32],
        output: &mut [f32],
    ) -> std::result::Result<(), JsValue>;
}
fn host_error(_: JsValue) -> AudioError {
    AudioError::Invalid("WASM stretch component")
}
/// Borrowed serial instance; host retains lifetime and disposes it after the
/// stream finishes/fails. Never reuse this component for a second stream.
pub struct WasmStretch<'a> {
    component: &'a StretchComponent,
    info: StretchInfo,
}
impl<'a> WasmStretch<'a> {
    pub fn new(component: &'a StretchComponent) -> Result<Self> {
        fn integer(value: std::result::Result<f64, JsValue>, max: u32) -> Result<u32> {
            let value = value.map_err(host_error)?;
            if value.is_finite() && value.fract() == 0. && value >= 1. && value <= f64::from(max) {
                Ok(value as u32)
            } else {
                Err(AudioError::Invalid("WASM stretch metadata"))
            }
        }
        let info = StretchInfo {
            channels: integer(component.channels(), 64)? as usize,
            sample_rate: integer(component.sample_rate(), 384000)?,
            input_latency: integer(component.input_latency(), 65536)?,
            output_latency: integer(component.output_latency(), 65536)?,
        };
        Ok(Self { component, info })
    }
}
impl StretchEngine for WasmStretch<'_> {
    fn info(&self) -> StretchInfo {
        self.info
    }
    fn begin(&mut self, input: &[f32], rate: f64) -> Result<()> {
        self.component.invoke_begin(input, rate).map_err(host_error)
    }
    fn process(&mut self, input: &[f32], output: &mut [f32]) -> Result<()> {
        self.component
            .invoke_process(input, output)
            .map_err(host_error)
    }
}
