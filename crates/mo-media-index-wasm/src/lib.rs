//! Thin synchronous Worker boundary over the exact native index/time semantics.
//! Host admits immutable bytes, supplies bounded ReadAt and owns hard termination.
use js_sys::{Function, Uint8Array};
use mo_media::MediaError;
use mo_media_isobmff::{
    IndexLimits,
    portable::{self, PortableIndex},
};
use std::io;
use wasm_bindgen::prelude::*;
fn failure(error: MediaError) -> JsValue {
    JsValue::from_str(&serde_json::to_string(&error).expect("typed media failure"))
}
#[wasm_bindgen]
pub struct MediaIndex {
    inner: PortableIndex,
    byte_length: u64,
}
#[wasm_bindgen]
impl MediaIndex {
    /// readAt(offset: bigint, count: number) returns a Uint8Array no larger than
    /// count (<=64 KiB). It is called only during open, never retained by Rust.
    pub fn open(
        byte_length: u64,
        read_at: &Function,
        options: &str,
    ) -> Result<MediaIndex, JsValue> {
        let limits: IndexLimits = if options.is_empty() {
            IndexLimits::default()
        } else {
            portable::decode(options).map_err(failure)?
        };
        let mut reader = reader(byte_length, read_at);
        Ok(Self {
            inner: PortableIndex::open(&mut reader, byte_length, limits, &|| false)
                .map_err(failure)?,
            byte_length,
        })
    }
    pub fn summary(&self) -> Result<String, JsValue> {
        self.inner.summary().map_err(failure)
    }
    pub fn query(&self, request: &str) -> Result<String, JsValue> {
        self.inner.query(request).map_err(failure)
    }
    #[wasm_bindgen(js_name=videoTiming)]
    pub fn video_timing(&mut self, track: u32) -> Result<String, JsValue> {
        self.inner.video_timing(track, &|| false).map_err(failure)
    }
    #[wasm_bindgen(js_name=selectVideo)]
    pub fn select_video(&mut self, request: &str) -> Result<String, JsValue> {
        self.inner.select_video(request, &|| false).map_err(failure)
    }
    pub fn movie(&self, request: &str) -> Result<String, JsValue> {
        self.inner.movie(request, &|| false).map_err(failure)
    }
    /// Same immutable byte capability as open; no callback is retained. A host
    /// yields between bounded batches and owns cancellation/deadline termination.
    #[wasm_bindgen(js_name=scanAvc)]
    pub fn scan_avc(&mut self, request: &str, read_at: &Function) -> Result<String, JsValue> {
        self.inner
            .scan_avc(request, &mut reader(self.byte_length, read_at), &|| false)
            .map_err(failure)
    }
    #[wasm_bindgen(js_name=avcSample)]
    pub fn avc_sample(&self, request: &str) -> Result<String, JsValue> {
        self.inner.avc_sample(request).map_err(failure)
    }
    #[wasm_bindgen(js_name=avcBootstrap)]
    pub fn avc_bootstrap(&self, request: &str) -> Result<Vec<u8>, JsValue> {
        self.inner.avc_bootstrap(request).map_err(failure)
    }
}
fn reader(
    byte_length: u64,
    read_at: &Function,
) -> impl FnMut(u64, &mut [u8]) -> io::Result<usize> + '_ {
    move |offset, destination| {
        if destination.len() > 65536
            || offset > byte_length
            || destination.len() as u64 > byte_length - offset
        {
            return Err(io::Error::other("source read range"));
        }
        let value = read_at
            .call2(
                &JsValue::UNDEFINED,
                &JsValue::from(offset),
                &JsValue::from(destination.len() as u32),
            )
            .map_err(|_| io::Error::other("host read failed"))?;
        let bytes = value
            .dyn_into::<Uint8Array>()
            .map_err(|_| io::Error::other("host read must return Uint8Array"))?;
        let length = bytes.length() as usize;
        if length == 0 || length > destination.len() {
            return Err(io::Error::other("host read size"));
        }
        bytes.copy_to(&mut destination[..length]);
        Ok(length)
    }
}
#[wasm_bindgen]
pub fn media_index_profile() -> String {
    portable::PROFILE.into()
}
// The Worker cannot process cancel messages while synchronous Rust is running.
// Cancellation belongs to the host's terminate/deadline boundary, not a fake flag.
