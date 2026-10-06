//! Test-only cross-target driver, not a product SDK or durable host.
mod audio;
mod pcm;
pub use audio::video_audio_conformance;
pub use pcm::video_pcm_conformance;
mod projection;
mod render;
use mo_common::from_json_str_with_path;
use mo_video_edit::*;
use mo_video_model::*;
pub use projection::video_projection_conformance;
pub use render::video_render_conformance;
use serde::Deserialize;
use serde_json::{Value, json};
use wasm_bindgen::prelude::*;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Fixture {
    document: VideoDocument,
    transactions: Vec<Vec<Operation>>,
}

fn run(input: &str) -> std::result::Result<Value, String> {
    if input.len() > 32 * 1024 * 1024 {
        return Err("fixture too large".into());
    }
    let fixture: Fixture = from_json_str_with_path(input).map_err(|e| e.to_string())?;
    if fixture.transactions.len() > 1024 {
        return Err("too many fixture steps".into());
    }
    let mut base = Snapshot::new(fixture.document, ValidationLimits::default(), &|| false)
        .map_err(|e| e.to_string())?;
    let mut steps = vec![];
    for operations in fixture.transactions {
        let request = Transaction {
            contract: EditContract::V1,
            expected_digest: base.digest().clone(),
            operations,
        };
        match prepare(&base, &request, EditLimits::default(), &||false) {
            Ok(result) => {
                let inverse = prepare_inverse(&result.candidate, &result.inverse, EditLimits::default(), &||false).map_err(|e| e.to_string())?;
                if inverse.candidate.digest() != base.digest() { return Err("inverse failed exact roundtrip".into()); }
                steps.push(json!({"status":"prepared","baseDigest":result.base_digest,"requestDigest":result.request_digest,
                    "document":result.candidate.document(),"digest":result.candidate.digest(),"readSet":result.read_set,
                    "writeSet":result.write_set,"inverse":result.inverse,"invalidations":result.invalidations}));
                base = result.candidate;
            }
            Err(error) => steps.push(json!({"status":"rejected","error":error.to_string(),"unchangedDigest":base.digest()})),
        }
    }
    Ok(json!({"status":"complete","steps":steps,"finalDigest":base.digest()}))
}

#[wasm_bindgen]
pub fn video_conformance(input: &str) -> String {
    match run(input) {
        Ok(value) => value.to_string(),
        Err(error) => json!({"status":"invalidFixture","error":error}).to_string(),
    }
}
