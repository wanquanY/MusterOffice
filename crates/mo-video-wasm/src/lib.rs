//! Independent authoring component. Host owns worker lifetime, storage and media.
//! This does not link PPT, codecs, file/network I/O or product state.
use mo_video_operations as api;
use wasm_bindgen::prelude::*;
fn failure(error: api::Failure) -> JsValue {
    JsValue::from_str(&serde_json::to_string(&error).expect("typed failure"))
}
#[wasm_bindgen]
pub struct VideoProject {
    inner: api::VideoProject,
}
#[wasm_bindgen]
impl VideoProject {
    #[wasm_bindgen(js_name=create)]
    pub fn create(input: &str) -> Result<VideoProject, JsValue> {
        let limits = api::Limits::default();
        let input = api::decode(input, 65536, &|| false).map_err(failure)?;
        Ok(Self {
            inner: api::VideoProject::create(input, limits, &|| false).map_err(failure)?,
        })
    }
    #[wasm_bindgen(js_name=open)]
    pub fn open(input: &str) -> Result<VideoProject, JsValue> {
        let limits = api::Limits::default();
        let record = api::decode(input, limits.max_snapshot_bytes, &|| false).map_err(failure)?;
        Ok(Self {
            inner: api::VideoProject::open(record, limits, &|| false).map_err(failure)?,
        })
    }
    pub fn snapshot(&self) -> Result<String, JsValue> {
        api::encode(
            &self.inner.snapshot(),
            api::Limits::default().max_snapshot_bytes,
            &|| false,
        )
        .map_err(failure)
    }
    pub fn summary(&self) -> Result<String, JsValue> {
        let v = self.inner.summary(&|| false).map_err(failure)?;
        api::encode(&v, 65536, &|| false).map_err(failure)
    }
    pub fn list(&self, input: &str) -> Result<String, JsValue> {
        let request = api::decode(input, 65536, &|| false).map_err(failure)?;
        let page = self.inner.list(request, &|| false).map_err(failure)?;
        api::encode(
            &page,
            api::Limits::default().query.max_output_bytes,
            &|| false,
        )
        .map_err(failure)
    }
    pub fn prepare(&self, input: &str) -> Result<String, JsValue> {
        let limits = api::Limits::default();
        let request =
            api::decode(input, limits.edit.max_request_bytes, &|| false).map_err(failure)?;
        let candidate = self.inner.prepare(&request, &|| false).map_err(failure)?;
        api::encode(&candidate, limits.max_output_bytes, &|| false).map_err(failure)
    }
    #[wasm_bindgen(js_name=prepareInverse)]
    pub fn prepare_inverse(&self, input: &str) -> Result<String, JsValue> {
        let limits = api::Limits::default();
        let request =
            api::decode(input, limits.edit.max_request_bytes, &|| false).map_err(failure)?;
        let candidate = self
            .inner
            .prepare_inverse(&request, &|| false)
            .map_err(failure)?;
        api::encode(&candidate, limits.max_output_bytes, &|| false).map_err(failure)
    }
    pub fn frame(&self, input: &str) -> Result<String, JsValue> {
        let request = api::decode(input, 65536, &|| false).map_err(failure)?;
        let plan = self.inner.frame(request, &|| false).map_err(failure)?;
        api::encode(
            &plan,
            api::Limits::default().query.max_output_bytes,
            &|| false,
        )
        .map_err(failure)
    }
    pub fn resources(&self) -> Result<String, JsValue> {
        let closure = self.inner.resources(&|| false).map_err(failure)?;
        api::encode(&closure, api::Limits::default().max_output_bytes, &|| false).map_err(failure)
    }
}
#[wasm_bindgen]
pub fn video_profile() -> String {
    api::PROFILE.into()
}
