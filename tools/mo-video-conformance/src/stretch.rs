//! Test-only identical Rust scheduling around Native/WASM DSP implementations.
use mo_video_audio::*;
use serde::Deserialize;
use serde_json::{Value, json};
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Fixture {
    sample_rate: u32,
    channels: usize,
    samples: Vec<f32>,
    output_frames: u64,
    #[serde(default)]
    retry_reads: bool,
}
struct Input {
    samples: Vec<f32>,
    channels: usize,
    fail: bool,
    retry_reads: bool,
}
impl StretchInput for Input {
    fn read(&mut self, first: u64, samples: &mut [f32], _: &dyn Fn() -> bool) -> Result<()> {
        if self.fail {
            self.fail = false;
            return Err(AudioError::Resource("fixture pending".into()));
        }
        self.fail = self.retry_reads;
        let offset = first as usize * self.channels;
        samples.copy_from_slice(&self.samples[offset..offset + samples.len()]);
        Ok(())
    }
}
fn parse(input: &str) -> std::result::Result<Fixture, String> {
    if input.len() > 16 << 20 {
        return Err("fixture too large".into());
    }
    let value: Fixture = serde_json::from_str(input).map_err(|e| e.to_string())?;
    if value.channels == 0
        || value.channels > 8
        || value.samples.is_empty()
        || !value.samples.len().is_multiple_of(value.channels)
        || value.samples.len() > 1_000_000
        || value.output_frames > 480000
    {
        return Err("fixture PCM limits".into());
    }
    Ok(value)
}
fn run<E: StretchEngine>(fixture: Fixture, engine: E) -> std::result::Result<Value, String> {
    let layout = match fixture.channels {
        1 => ChannelLayout::mono(),
        2 => ChannelLayout::stereo(),
        _ => return Err("fixture layout".into()),
    };
    let mut stream = StretchStream::new(
        engine,
        fixture.sample_rate,
        layout,
        (fixture.samples.len() / fixture.channels) as u64,
        fixture.output_frames,
        StretchLimits::default(),
    )
    .map_err(|e| e.to_string())?;
    let mut source = Input {
        samples: fixture.samples,
        channels: fixture.channels,
        fail: fixture.retry_reads,
        retry_reads: fixture.retry_reads,
    };
    let mut output = vec![];
    let mut chunks = vec![];
    let mut retries = 0;
    loop {
        match stream.next(&mut source, &|| false) {
            Ok(Some(block)) => {
                if block.first_frame() != (output.len() / fixture.channels) as i64 {
                    return Err("non-contiguous output".into());
                }
                chunks.push(block.frames());
                output.extend_from_slice(block.samples());
            }
            Ok(None) => break,
            Err(AudioError::Resource(message)) if message == "fixture pending" => {
                retries += 1;
            }
            Err(e) => return Err(e.to_string()),
        }
    }
    Ok(
        json!({"status":"complete", "samples":output, "chunks":chunks, "retries":retries,
        "inputLatency":stream.engine().info().input_latency, "outputLatency":stream.engine().info().output_latency,
        "bufferBytes":stream.buffer_bytes()}),
    )
}
fn reply(result: std::result::Result<Value, String>) -> String {
    result
        .unwrap_or_else(|error| json!({"status":"invalidFixture","error":error}))
        .to_string()
}
#[cfg(not(target_arch = "wasm32"))]
pub fn video_stretch_conformance(input: &str) -> String {
    reply(parse(input).and_then(|fixture| {
        let engine =
            mo_stretch_sys::NativeStretch::new(fixture.sample_rate, fixture.channels, 32 << 20)
                .map_err(|e| e.to_string())?;
        run(fixture, engine)
    }))
}
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub fn video_stretch_conformance(
    input: &str,
    component: &mo_stretch_wasm::StretchComponent,
) -> String {
    reply(parse(input).and_then(|fixture| {
        let engine = mo_stretch_wasm::WasmStretch::new(component).map_err(|e| e.to_string())?;
        run(fixture, engine)
    }))
}
