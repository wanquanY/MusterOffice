use super::*;
use js_sys::{Function, Uint8Array};
use mo_common::Ticks;
use mo_media_isobmff::{
    Edit,
    movie::{MovieClock, MovieTimeline},
    timing::TimingLimits,
};
use mo_video_audio::{AudioAsset, AudioResources, PcmLimits, PcmRange};
use mo_video_compile::MediaBinding;
use mo_video_pcm::{
    CanonicalAudioResource, PcmReadLimits, PcmStreamRecord, PcmTileRef, PcmTileStore,
    audio_movie::{MovieAudioLimits, MovieAudioPlan, MovieAudioResource},
};

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Clock {
    track_id: u32,
    movie_timescale: u32,
    media_timescale: u32,
    declared_track_duration: Option<MovieTicks>,
    edits: Vec<ClockEdit>,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct ClockEdit {
    movie_start: MovieTicks,
    movie_duration: MovieTicks,
    media_time: Option<Ticks>,
    rate_16_16: i32,
}
/// ISO BMFF movie durations are unsigned 64-bit ticks. Keep their wire form
/// exact without reusing byte-length or signed document-time value types.
#[derive(Deserialize)]
#[serde(try_from = "String")]
struct MovieTicks(u64);
impl TryFrom<String> for MovieTicks {
    type Error = &'static str;
    fn try_from(text: String) -> std::result::Result<Self, Self::Error> {
        if text.is_empty()
            || text.len() > 20
            || !text.bytes().all(|b| b.is_ascii_digit())
            || (text.len() > 1 && text.starts_with('0'))
        {
            return Err("canonical unsigned movie ticks required");
        }
        text.parse().map(Self).map_err(|_| "movie tick overflow")
    }
}
struct Store {
    lookup: Function,
    read: Function,
}
impl PcmTileStore for Store {
    fn lookup(
        &mut self,
        source: &Digest,
        index: u32,
        _: &dyn Fn() -> bool,
    ) -> mo_video_pcm::Result<Option<PcmTileRef>> {
        let value = self
            .lookup
            .call2(
                &JsValue::UNDEFINED,
                &JsValue::from_str(source.as_str()),
                &JsValue::from(index),
            )
            .map_err(|_| PcmError::Store("host lookup failed".into()))?;
        if value.is_null() || value.is_undefined() {
            return Ok(None);
        }
        let text = value
            .as_string()
            .ok_or(PcmError::Invalid("tile reference type"))?;
        if text.len() > 4096 {
            return Err(PcmError::Budget);
        }
        mo_common::from_json_str_with_path(&text)
            .map(Some)
            .map_err(|_| PcmError::Invalid("tile reference"))
    }
    fn read(
        &mut self,
        reference: &PcmTileRef,
        max_bytes: usize,
        _: &dyn Fn() -> bool,
    ) -> mo_video_pcm::Result<Vec<u8>> {
        let text =
            serde_json::to_string(reference).map_err(|_| PcmError::Invalid("tile reference"))?;
        let value = self
            .read
            .call2(
                &JsValue::UNDEFINED,
                &JsValue::from_str(&text),
                &JsValue::from(max_bytes as u32),
            )
            .map_err(|_| PcmError::Store("host read failed".into()))?;
        let bytes = value
            .dyn_into::<Uint8Array>()
            .map_err(|_| PcmError::Invalid("tile payload type"))?;
        if bytes.length() as usize > max_bytes {
            return Err(PcmError::Budget);
        }
        Ok(bytes.to_vec())
    }
}

/// A bounded read capability over HOST-authorized, atomically committed tiles.
/// This does not open files, persist data, fetch a model or trust arbitrary JSON
/// as proof of authorization. Missing/corrupt PCM fails; only empty edits silence.
#[wasm_bindgen]
pub struct MoviePcm {
    source: MovieAudioResource<CanonicalAudioResource<Store>>,
    binding: MediaBinding,
    asset: AudioAsset,
}
#[wasm_bindgen]
impl MoviePcm {
    pub fn open(
        options: &str,
        record: &str,
        clock: &str,
        lookup: Function,
        read: Function,
    ) -> Result<Self> {
        let options = Options::read(options)?;
        let record: PcmStreamRecord = parse(record, 16384)?;
        let clock: Clock = parse(clock, 1 << 20)?;
        if clock.track_id != options.track_id || clock.edits.len() > 4096 {
            return Err(invalid());
        }
        let edits: Vec<_> = clock
            .edits
            .into_iter()
            .map(|edit| Edit {
                movie_start: edit.movie_start.0,
                movie_duration: edit.movie_duration.0,
                media_time: edit.media_time.map(|v| v.get()),
                rate_16_16: edit.rate_16_16,
            })
            .collect();
        let (decoded, source) = record
            .restore(&options.origin()?, limits())
            .map_err(failure)?;
        let raw = source.binding().ok_or_else(invalid)?.clone();
        let timeline = MovieTimeline::from_clock(
            MovieClock {
                track_id: clock.track_id,
                movie_timescale: clock.movie_timescale,
                media_timescale: clock.media_timescale,
                declared_track_duration: clock.declared_track_duration.map(|v| v.0),
                edits: &edits,
            },
            &ExactRatio::from_time(decoded.decoded_range(256).map_err(failure)?.end),
            TimingLimits {
                max_edits: 4096,
                ..Default::default()
            },
            &|| false,
        )
        .map_err(|_| invalid())?;
        let plan = MovieAudioPlan::new(
            &timeline,
            raw.clone(),
            &decoded,
            MovieAudioLimits {
                max_working_bytes: 8 << 20,
                ..Default::default()
            },
            &|| false,
        )
        .map_err(|e| failure(PcmError::Audio(e)))?;
        let asset = plan.asset().clone();
        let binding = MediaBinding {
            descriptor: plan.descriptor().clone(),
            ..raw
        };
        let canonical = CanonicalAudioResource::new(
            source,
            Store { lookup, read },
            PcmReadLimits {
                max_working_bytes: 4 << 20,
                max_cached_bytes: 1 << 20,
                max_cached_tiles: 32,
                ..Default::default()
            },
        )
        .map_err(failure)?;
        let source = MovieAudioResource::new(plan, binding.clone(), canonical)
            .map_err(|e| failure(PcmError::Audio(e)))?;
        Ok(Self {
            source,
            binding,
            asset,
        })
    }
    pub fn info(&self) -> Result<String> {
        json(
            &serde_json::json!({"profile":mo_video_pcm::audio_movie::PROFILE,"descriptor":self.binding.descriptor,
            "sampleRate":self.asset.sample_rate,"channels":self.asset.layout.len(),"frames":Ticks::new(self.asset.available.end)}),
        )
    }
    /// Output is owned f32le. No previous JS result remains owned by this object.
    pub fn read(&mut self, first: i64, frames: u32) -> Result<Uint8Array> {
        if frames > 8192 {
            return Err(failure(PcmError::Budget));
        }
        let end = first.checked_add(i64::from(frames)).ok_or_else(invalid)?;
        let range = PcmRange::new(first, end).map_err(|e| failure(PcmError::Audio(e)))?;
        let pcm = self
            .source
            .read(
                &self.binding,
                &self.asset,
                range,
                PcmLimits {
                    max_frames: 8192,
                    max_bytes: 8192 * 2 * 4,
                },
                &|| false,
            )
            .map_err(|e| failure(PcmError::Audio(e)))?;
        let bytes: Vec<u8> = pcm.samples().iter().flat_map(|v| v.to_le_bytes()).collect();
        Ok(Uint8Array::from(bytes.as_slice()))
    }
    #[wasm_bindgen(js_name=retainedBytes)]
    pub fn retained_bytes(&mut self) -> usize {
        self.source.retained_filter_bytes() + self.source.raw_mut().retained_bytes()
    }
    pub fn clear(&mut self) {
        self.source.clear();
        self.source.raw_mut().clear();
    }
}
