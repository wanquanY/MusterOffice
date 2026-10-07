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
        let mut reader = |offset: u64, destination: &mut [u8]| -> io::Result<usize> {
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
        };
        Ok(Self {
            inner: PortableIndex::open(&mut reader, byte_length, limits, &|| false)
                .map_err(failure)?,
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
}
#[wasm_bindgen]
pub fn media_index_profile() -> String {
    portable::PROFILE.into()
}
// The Worker cannot process cancel messages while synchronous Rust is running.
// Cancellation belongs to the host's terminate/deadline boundary, not a fake flag.
