#![allow(dead_code)]
use mo_common::{ByteLength, Digest, DocumentId, RationalTime, ResourceId};
use mo_time::ExactRatio as R;
use mo_video_audio::*;
use mo_video_compile::MediaBinding;
use mo_video_model::*;
use std::collections::BTreeMap;
pub fn t(n: i64) -> RationalTime {
    RationalTime::new(n, 8000).unwrap()
}
pub fn range(a: i64, b: i64) -> TimeRange {
    TimeRange {
        start: t(a),
        end: t(b),
    }
}
pub fn sid(id: &str) -> SequenceId {
    SequenceId::new(id).unwrap()
}
pub fn tid(id: &str) -> TrackId {
    TrackId::new(id).unwrap()
}
pub fn cid(id: &str) -> ClipId {
    ClipId::new(id).unwrap()
}
pub fn mapping(a: i64, b: i64, n: i64) -> TimeMap {
    TimeMap {
        anchors: vec![
            TimeAnchor {
                local: t(0),
                source: t(a),
            },
            TimeAnchor {
                local: t(n),
                source: t(b),
            },
        ],
    }
}
pub fn document(samples: Vec<f32>, sample_rate: u32) -> (VideoDocument, MemoryAudio) {
    let mut d = VideoDocument::empty(
        DocumentId::new("sound").unwrap(),
        sid("main"),
        Canvas {
            width: 100,
            height: 100,
        },
        FrameRate {
            numerator: 25,
            denominator: 1,
        },
    );
    d.audio_sample_rate = 8000;
    let id = ResourceId::new("source").unwrap();
    let stream = StreamId::new("audio").unwrap();
    let descriptor = MediaStream::Audio {
        available: TimeRange {
            start: t(0),
            end: RationalTime::new(samples.len() as i64, sample_rate).unwrap(),
        },
        sample_rate,
        channels: 1,
    };
    let digest = Digest::from_sha256([1; 32]);
    d.resources.insert(
        id.clone(),
        Resource {
            id: id.clone(),
            digest: digest.clone(),
            byte_length: ByteLength::new(100),
            descriptor: ResourceDescriptor::Media {
                streams: BTreeMap::from([(stream.clone(), descriptor.clone())]),
            },
        },
    );
    let asset = AudioAsset {
        sample_rate,
        layout: ChannelLayout::mono(),
        sample_zero: R::integer(0),
        available: PcmRange::new(0, samples.len() as i64).unwrap(),
    };
    (
        d,
        MemoryAudio {
            samples,
            asset,
            binding: MediaBinding {
                resource: id,
                digest,
                stream,
                descriptor,
            },
            reads: 0,
        },
    )
}
pub fn track(d: &mut VideoDocument, sequence: &str, id: &str) {
    d.sequences
        .get_mut(&sid(sequence))
        .unwrap()
        .tracks
        .push(tid(id));
    d.tracks.insert(
        tid(id),
        Track {
            id: tid(id),
            name: id.into(),
            kind: TrackKind::Audio,
            locked: false,
            muted: false,
            solo: false,
            clips: vec![],
        },
    );
}
pub fn media(host: &MemoryAudio, map: TimeMap) -> ClipSource {
    ClipSource::Media {
        resource: host.binding.resource.clone(),
        stream: host.binding.stream.clone(),
        time_map: map,
    }
}
pub fn clip(d: &mut VideoDocument, track: &str, id: &str, range: TimeRange, source: ClipSource) {
    d.tracks.get_mut(&tid(track)).unwrap().clips.push(cid(id));
    d.clips.insert(
        cid(id),
        Clip {
            audio_context: None,
            id: cid(id),
            name: id.into(),
            timeline: range,
            source,
            effects: vec![],
            audio: AudioTimePolicy {
                rate: AudioRatePolicy::Resample,
                reverse: ReverseAudioPolicy::Reverse,
                hold: HoldAudioPolicy::Silence,
            },
        },
    );
}
pub struct MemoryAudio {
    pub samples: Vec<f32>,
    pub asset: AudioAsset,
    pub binding: MediaBinding,
    pub reads: usize,
}
impl AudioResources for MemoryAudio {
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
        self.reads += 1;
        PcmBlock::new(
            asset.sample_rate,
            asset.layout.clone(),
            range.first,
            self.samples[range.first as usize..range.end as usize].to_vec(),
            limits,
            check,
        )
    }
    fn channel_matrix(
        &mut self,
        input: &ChannelLayout,
        output: &ChannelLayout,
    ) -> Result<ChannelMatrix> {
        if input == &ChannelLayout::mono() && output == &ChannelLayout::stereo() {
            Ok(ChannelMatrix::mono_to_stereo())
        } else {
            Err(AudioError::Unsupported("test matrix"))
        }
    }
}
