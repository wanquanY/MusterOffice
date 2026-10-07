#![allow(dead_code)]
use mo_video_runtime::audio::Result;
use mo_video_runtime::{
    audio::*, audio_movie::*, authoring::model::*, common::*, compile::MediaBinding, isobmff::*,
    media::*, pcm::*, time::ExactRatio as R,
};
pub fn t(n: i64, d: u32) -> RationalTime {
    RationalTime::new(n, d).unwrap()
}
pub fn r(n: i64, d: u32) -> R {
    R::from_time(t(n, d)).normalized()
}
pub fn frame(first: i64, samples: &[f32], sample_rate: u32) -> DecodedFrame {
    DecodedFrame {
        info:serde_json::from_value(serde_json::json!({
            "generation":1,"kind":"audio","timeBase":{"numerator":1,"denominator":sample_rate},
            "ptsTicks":first,"durationTicks":999999,"width":null,"height":null,"sampleRate":sample_rate,
            "channels":1,"samples":samples.len(),"format":"flt","channelLayout":"mono",
            "colors":{"primaries":2,"transfer":2,"matrix":2,"range":0,"chromaLocation":0},
            "sampleAspectRatio":null,"alphaMode":0,"keyFrame":true,"interlaced":false,"topFieldFirst":false
        })).unwrap(),
        planes:vec![FramePlane{row_bytes:samples.len()*4,rows:1,bytes:samples.iter().flat_map(|s|s.to_le_bytes()).collect()}]
    }
}
pub fn inspect(samples: &[f32], sr: u32) -> InspectedPcm {
    let mut scan = PcmInspection::new(1, PcmInspectLimits::default()).unwrap();
    scan.observe(&frame(0, samples, sr), ByteOrder::Little, &|| false)
        .unwrap();
    scan.finish_at_eof(&|| false).unwrap()
}
pub fn binding(decoded: &InspectedPcm) -> MediaBinding {
    MediaBinding {
        resource: ResourceId::new("audio").unwrap(),
        digest: Digest::from_sha256([7; 32]),
        stream: StreamId::new("track-2").unwrap(),
        descriptor: MediaStream::Audio {
            available: decoded.decoded_range(256).unwrap(),
            sample_rate: decoded.asset().sample_rate,
            channels: decoded.asset().layout.len() as u16,
        },
    }
}
pub struct Memory {
    pub binding: MediaBinding,
    pub asset: AudioAsset,
    pub samples: Vec<f32>,
    pub requests: Vec<PcmRange>,
    pub broken: bool,
}
impl AudioResources for Memory {
    fn describe(&mut self, binding: &MediaBinding) -> Result<AudioAsset> {
        assert_eq!(binding, &self.binding);
        Ok(self.asset.clone())
    }
    fn read(
        &mut self,
        binding: &MediaBinding,
        asset: &AudioAsset,
        range: PcmRange,
        limits: PcmLimits,
        check: &dyn Fn() -> bool,
    ) -> Result<PcmBlock> {
        assert_eq!(binding, &self.binding);
        assert_eq!(asset, &self.asset);
        self.requests.push(range);
        if self.broken {
            return Err(AudioError::Resource("fixture unavailable".into()));
        }
        assert!(range.first >= 0 && range.end <= asset.available.end);
        PcmBlock::new(
            asset.sample_rate,
            asset.layout.clone(),
            range.first,
            self.samples[range.first as usize * asset.layout.len()
                ..range.end as usize * asset.layout.len()]
                .to_vec(),
            limits,
            check,
        )
    }
}
pub fn track(edits: Vec<Edit>, timescale: u32) -> TrackIndex {
    TrackIndex {
        id: 2,
        flags: 3,
        handler: *b"soun",
        declared_track_duration: None,
        timescale,
        declared_media_duration: None,
        decode_duration: 0,
        display_matrix: [0; 9],
        display_size_16_16: [0; 2],
        descriptions: vec![],
        samples: vec![],
        edits,
        auxiliary: vec![],
    }
}
pub fn edit(start: u64, duration: u64, media: Option<i64>, rate: i32) -> Edit {
    Edit {
        movie_start: start,
        movie_duration: duration,
        media_time: media,
        rate_16_16: rate,
    }
}
pub fn resource(
    samples: Vec<f32>,
    edits: Vec<Edit>,
    scale: u32,
    limits: MovieAudioLimits,
) -> (MovieAudioResource<Memory>, MediaBinding, AudioAsset) {
    let decoded = inspect(&samples, 8000);
    let raw = binding(&decoded);
    let timeline = movie::MovieTimeline::build(
        &track(edits, 8000),
        scale,
        &r(samples.len() as i64, 8000),
        presentation::TimingLimits::default(),
        &|| false,
    )
    .unwrap();
    let plan = MovieAudioPlan::new(&timeline, raw.clone(), &decoded, limits, &|| false).unwrap();
    let mut output = raw.clone();
    output.descriptor = plan.descriptor().clone();
    let asset = plan.asset().clone();
    let memory = Memory {
        binding: raw,
        asset: decoded.asset().clone(),
        samples,
        requests: vec![],
        broken: false,
    };
    (
        MovieAudioResource::new(plan, output.clone(), memory).unwrap(),
        output,
        asset,
    )
}
pub fn read(
    source: &mut MovieAudioResource<Memory>,
    binding: &MediaBinding,
    asset: &AudioAsset,
    first: i64,
    end: i64,
) -> Vec<f32> {
    source
        .read(
            binding,
            asset,
            PcmRange::new(first, end).unwrap(),
            PcmLimits::default(),
            &|| false,
        )
        .unwrap()
        .samples()
        .to_vec()
}
