//! Test-only PCM capability binding. Not a decoder or production resource host.
use mo_common::{ResourceId, Ticks, from_json_str_with_path};
use mo_time::ExactRatio as R;
use mo_video_audio::*;
use mo_video_compile::{CompileLimits, CompiledDocument, MediaBinding, QueryLimits};
use mo_video_model::{MediaStream, StreamId, VideoDocument};
use serde::Deserialize;
use serde_json::{Value, json};
use wasm_bindgen::prelude::*;
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct PcmFixture {
    resource: ResourceId,
    stream: StreamId,
    sample_zero: R,
    first_sample: Ticks,
    samples: Vec<f32>,
}
struct Host {
    values: Vec<PcmFixture>,
}
impl AudioResources for Host {
    fn describe(&mut self, binding: &MediaBinding) -> Result<AudioAsset> {
        let value = self
            .values
            .iter()
            .find(|v| v.resource == binding.resource && v.stream == binding.stream)
            .ok_or_else(|| AudioError::Resource("fixture missing".into()))?;
        let MediaStream::Audio {
            sample_rate,
            channels: 1,
            ..
        } = binding.descriptor
        else {
            return Err(AudioError::Unsupported("fixture mono PCM"));
        };
        Ok(AudioAsset {
            sample_rate,
            layout: ChannelLayout::mono(),
            sample_zero: value.sample_zero.clone(),
            available: PcmRange::new(
                value.first_sample.get(),
                value.first_sample.get() + value.samples.len() as i64,
            )?,
        })
    }
    fn read(
        &mut self,
        binding: &MediaBinding,
        asset: &AudioAsset,
        range: PcmRange,
        limits: PcmLimits,
        check: &dyn Fn() -> bool,
    ) -> Result<PcmBlock> {
        let value = self
            .values
            .iter()
            .find(|v| v.resource == binding.resource && v.stream == binding.stream)
            .ok_or_else(|| AudioError::Resource("fixture missing".into()))?;
        let start = usize::try_from(range.first - value.first_sample.get())
            .map_err(|_| AudioError::Invalid("fixture offset"))?;
        let end = usize::try_from(range.end - value.first_sample.get())
            .map_err(|_| AudioError::Invalid("fixture end"))?;
        let samples = value
            .samples
            .get(start..end)
            .ok_or(AudioError::Invalid("fixture PCM window"))?;
        PcmBlock::new(
            asset.sample_rate,
            asset.layout.clone(),
            range.first,
            samples.to_vec(),
            limits,
            check,
        )
    }
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Query {
    first: Ticks,
    frames: u32,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Fixture {
    document: VideoDocument,
    pcm: Vec<PcmFixture>,
    queries: Vec<Query>,
}
fn run(input: &str) -> std::result::Result<Value, String> {
    if input.len() > 4 << 20 {
        return Err("fixture too large".into());
    }
    let fixture: Fixture = from_json_str_with_path(input).map_err(|e| e.to_string())?;
    if fixture.queries.len() > 32
        || fixture.pcm.len() > 16
        || fixture.pcm.iter().any(|p| {
            p.samples.len() > 65536
                || p.first_sample
                    .get()
                    .checked_add(p.samples.len() as i64)
                    .is_none()
        })
    {
        return Err("fixture limits".into());
    }
    let c = CompiledDocument::new(fixture.document, CompileLimits::default(), &|| false)
        .map_err(|e| e.to_string())?;
    let mut mixer = AudioMixer::new(&c, ChannelLayout::mono(), MixLimits::default())
        .map_err(|e| e.to_string())?;
    let mut host = Host {
        values: fixture.pcm,
    };
    let mut descriptions = vec![];
    for value in &host.values {
        let resource = c
            .document()
            .resources
            .get(&value.resource)
            .ok_or("fixture resource")?;
        let mo_video_model::ResourceDescriptor::Media { streams } = &resource.descriptor else {
            return Err("fixture resource type".into());
        };
        let binding = MediaBinding {
            resource: resource.id.clone(),
            digest: resource.digest.clone(),
            stream: value.stream.clone(),
            descriptor: streams.get(&value.stream).ok_or("fixture stream")?.clone(),
        };
        let MediaStream::Audio {
            sample_rate,
            channels: 1,
            ..
        } = binding.descriptor
        else {
            return Err("fixture layout".into());
        };
        let asset = AudioAsset {
            sample_rate,
            layout: ChannelLayout::mono(),
            sample_zero: value.sample_zero.clone(),
            available: PcmRange::new(
                value.first_sample.get(),
                value.first_sample.get() + value.samples.len() as i64,
            )
            .map_err(|e| e.to_string())?,
        };
        descriptions
            .push(AudioResourceDescription::new(binding, &asset, 256).map_err(|e| e.to_string())?);
    }
    let mut results = vec![];
    for query in fixture.queries {
        if query.frames > 4096 {
            return Err("fixture output limit".into());
        }
        match mixer.render(&mut host, query.first.get(), query.frames, &|| false) {
            Ok(block) => {
                let plan = c
                    .audio_block(
                        query.first.get(),
                        query.frames,
                        QueryLimits::default(),
                        &|| false,
                    )
                    .map_err(|e| e.to_string())?
                    .value;
                results.push(json!({"status":"mixed","bits":block.pcm.samples().iter().map(|v|v.to_bits()).collect::<Vec<_>>(),"plan":plan,
                    "stats":{"voices":block.stats.voices,"visited":block.stats.visited,"multiplyAdds":block.stats.multiply_adds,"sourceFrames":block.stats.source_frames,"peakPcmBytes":block.stats.peak_pcm_bytes,"kernelBytes":block.stats.retained_kernel_bytes}}));
            }
            Err(error) => results.push(json!({"status":"rejected","error":error.to_string()})),
        }
    }
    Ok(
        json!({"status":"complete","profile":PROFILE,"documentDigest":c.digest(),"descriptions":descriptions,"results":results}),
    )
}
#[wasm_bindgen]
pub fn video_audio_conformance(input: &str) -> String {
    match run(input) {
        Ok(value) => value.to_string(),
        Err(error) => json!({"status":"invalidFixture","error":error}).to_string(),
    }
}
