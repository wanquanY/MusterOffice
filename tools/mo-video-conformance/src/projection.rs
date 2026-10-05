use mo_common::{Ticks, check_json_size, from_json_str_with_path};
use mo_video_compile::*;
use mo_video_model::{TimeSample, VideoDocument};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use wasm_bindgen::prelude::*;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Fixture {
    document: VideoDocument,
    queries: Vec<Query>,
}
#[derive(Deserialize)]
#[serde(tag = "type", rename_all = "camelCase", deny_unknown_fields)]
enum Query {
    Frame {
        index: Ticks,
    },
    AudioSample {
        index: Ticks,
    },
    VisualAt {
        sample: TimeSample,
    },
    AudioAt {
        sample: TimeSample,
    },
    Presentation {
        binding: Box<MediaBinding>,
        window: PresentationWindow,
        sample: TimeSample,
    },
}
fn value<T: Serialize>(result: Result<Computed<T>>) -> std::result::Result<Value, String> {
    let computed = result.map_err(|e| e.to_string())?;
    Ok(
        json!({"status":"computed","value":computed.value,"visits":computed.stats.visited,"selectedNodes":computed.stats.selected}),
    )
}
fn run(input: &str) -> std::result::Result<Value, String> {
    if input.len() > 32 * 1024 * 1024 {
        return Err("projection fixture too large".into());
    }
    let fixture: Fixture = from_json_str_with_path(input).map_err(|e| e.to_string())?;
    if fixture.queries.len() > 128 {
        return Err("too many projection queries".into());
    }
    let compiled = CompiledDocument::new(fixture.document, CompileLimits::default(), &|| false)
        .map_err(|e| e.to_string())?;
    let mut results = vec![];
    let limits = QueryLimits {
        max_output_bytes: 1024 * 1024,
        ..Default::default()
    };
    let mut remaining = 32 * 1024 * 1024;
    for query in fixture.queries {
        let result = match query {
            Query::Frame { index } => value(compiled.frame(index.get(), limits, &|| false)),
            Query::AudioSample { index } => {
                value(compiled.audio_sample(index.get(), limits, &|| false))
            }
            Query::VisualAt { sample } => value(compiled.visual_at(&sample, limits, &|| false)),
            Query::AudioAt { sample } => value(compiled.audio_at(&sample, limits, &|| false)),
            Query::Presentation {
                binding,
                window,
                sample,
            } => PresentationIndex::new(window, PresentationLimits::default(), &|| false)
                .and_then(|i| i.select(&binding, &sample, &|| false))
                .map(|v| json!({"status":"computed","value":v}))
                .map_err(|e| e.to_string()),
        };
        let result = result.unwrap_or_else(|error| json!({"status":"rejected","error":error}));
        remaining -= check_json_size(&result, remaining, &|| false).map_err(|e| e.to_string())?;
        results.push(result);
    }
    Ok(
        json!({"status":"complete","digest":compiled.digest(),"duration":compiled.duration(),
        "frameCount":compiled.frame_count().map_err(|e|e.to_string())?.to_string(),
        "audioSampleCount":compiled.audio_sample_count().map_err(|e|e.to_string())?.to_string(),"results":results}),
    )
}
#[wasm_bindgen]
pub fn video_projection_conformance(input: &str) -> String {
    match run(input) {
        Ok(value) => value.to_string(),
        Err(error) => json!({"status":"invalidFixture","error":error}).to_string(),
    }
}
