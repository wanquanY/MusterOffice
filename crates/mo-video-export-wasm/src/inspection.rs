//! Thin borrowed byte capability for the shared export verifier. It neither
//! opens a file nor decides whether the resulting proof may be published.
use super::{ProjectExport, Result, failure, json};
use js_sys::{Function, Uint8Array};
use mo_media_isobmff::{IndexLimits, IsoBmffIndex};
use std::io;
use wasm_bindgen::prelude::*;
#[wasm_bindgen]
impl ProjectExport {
    #[wasm_bindgen(js_name = inspectCandidate)]
    pub fn inspect_candidate(&self, length: u64, read_at: &Function) -> Result<String> {
        if length == 0 || length > 2 << 30 {
            return Err(failure("resource_limit", "candidate byte length"));
        }
        let mut read = |at: u64, destination: &mut [u8]| -> io::Result<usize> {
            if destination.len() > 65536 || at > length || destination.len() as u64 > length - at {
                return Err(io::Error::other("candidate read range"));
            }
            let value = read_at
                .call2(
                    &JsValue::UNDEFINED,
                    &JsValue::from(at),
                    &JsValue::from(destination.len() as u32),
                )
                .map_err(|_| io::Error::other("candidate read"))?;
            let bytes = value
                .dyn_into::<Uint8Array>()
                .map_err(|_| io::Error::other("candidate read type"))?;
            let count = bytes.length() as usize;
            if count == 0 || count > destination.len() {
                return Err(io::Error::other("candidate read size"));
            }
            bytes.copy_to(&mut destination[..count]);
            Ok(count)
        };
        let index = IsoBmffIndex::read(
            &mut read,
            length,
            IndexLimits {
                max_input_bytes: length,
                max_tracks: 2,
                ..IndexLimits::default()
            },
            &|| false,
        )
        .map_err(|error| failure("output_validation_failed", error))?;
        let report = mo_video_export::inspect_browser_candidate(&index, self.config, &|| false)
            .map_err(|error| failure("output_validation_failed", error))?;
        json(&report)
    }
}
