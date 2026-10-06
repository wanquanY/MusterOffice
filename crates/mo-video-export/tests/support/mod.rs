#![allow(dead_code)]
use mo_common::{ByteLength, Digest, DocumentId, RationalTime, ResourceId};
use mo_time::ExactRatio as R;
use mo_video_audio::*;
use mo_video_compile::{CompileLimits, CompiledDocument, MediaBinding};
use mo_video_export::*;
use mo_video_model::*;
use mo_video_render::*;
use std::collections::BTreeMap;
pub fn t(n: i64) -> RationalTime {
    RationalTime::new(n, 48000).unwrap()
}
pub fn range(a: i64, b: i64) -> TimeRange {
    TimeRange {
        start: t(a),
        end: t(b),
    }
}
pub fn options() -> ExportOptions {
    ExportOptions {
        profile: mo_media::EncodeProfile::Ffv1PcmMp4,
        video_bit_rate: 2_000_000,
        gop_frames: 25,
        audio: ExportAudio::Stereo { bit_rate: 128000 },
        background_srgb: [24, 24, 24],
        tone_map: None,
        sampling: Sampling::Bilinear,
    }
}
pub fn document() -> VideoDocument {
    let root = SequenceId::new("main").unwrap();
    let mut d = VideoDocument::empty(
        DocumentId::new("export-graph").unwrap(),
        root.clone(),
        Canvas {
            width: 128,
            height: 96,
        },
        FrameRate {
            numerator: 30000,
            denominator: 1001,
        },
    );
    for (name, kind) in [("visual", TrackKind::Graphics), ("audio", TrackKind::Audio)] {
        let id = TrackId::new(name).unwrap();
        d.sequences.get_mut(&root).unwrap().tracks.push(id.clone());
        d.tracks.insert(
            id.clone(),
            Track {
                id,
                name: name.into(),
                kind,
                locked: false,
                muted: false,
                solo: false,
                clips: vec![],
            },
        );
    }
    for (name, start, end, rgba) in [
        ("red", 0, 9600, [255, 0, 0, 255]),
        ("blue", 9600, 19200, [0, 0, 255, 128]),
        ("green", 21600, 24157, [0, 255, 0, 255]),
    ] {
        insert(
            &mut d,
            "visual",
            name,
            range(start, end),
            ClipSource::Solid { rgba },
        );
    }
    let resource = ResourceId::new("tone").unwrap();
    let stream = StreamId::new("mono").unwrap();
    d.resources.insert(
        resource.clone(),
        Resource {
            id: resource.clone(),
            digest: source_digest(),
            byte_length: ByteLength::new(8000 * 4),
            descriptor: ResourceDescriptor::Media {
                streams: BTreeMap::from([(
                    stream.clone(),
                    MediaStream::Audio {
                        available: range(0, 48000),
                        sample_rate: 8000,
                        channels: 1,
                    },
                )]),
            },
        },
    );
    insert(
        &mut d,
        "audio",
        "sound",
        range(0, 24157),
        ClipSource::Media {
            resource,
            stream,
            time_map: TimeMap {
                anchors: vec![
                    TimeAnchor {
                        local: t(0),
                        source: t(0),
                    },
                    TimeAnchor {
                        local: t(24157),
                        source: t(24157),
                    },
                ],
            },
        },
    );
    d.clips
        .get_mut(&ClipId::new("sound").unwrap())
        .unwrap()
        .effects
        .push(Effect::Gain {
            milli_decibels: -6000,
        });
    d
}
pub fn insert(
    d: &mut VideoDocument,
    track: &str,
    name: &str,
    timeline: TimeRange,
    source: ClipSource,
) {
    let id = ClipId::new(name).unwrap();
    d.tracks
        .get_mut(&TrackId::new(track).unwrap())
        .unwrap()
        .clips
        .push(id.clone());
    d.clips.insert(
        id.clone(),
        Clip {
            id,
            name: name.into(),
            timeline,
            source,
            effects: vec![],
            audio: AudioTimePolicy::default(),
            audio_context: None,
        },
    );
}
pub fn compile(d: VideoDocument) -> CompiledDocument {
    CompiledDocument::new(d, CompileLimits::default(), &|| false).unwrap()
}
pub struct Visual;
impl VisualResources for Visual {
    fn media(
        &mut self,
        _: &MediaBinding,
        _: &TimeSample,
        _: &dyn Fn() -> bool,
    ) -> mo_video_render::Result<ResolvedMedia> {
        panic!("no external visual resource in this graph")
    }
}
pub fn source_value(i: i64) -> f32 {
    let x = (i % 64) as f32;
    (if x < 32. { x } else { 64. - x }) / 64. - 0.25
}
pub fn source_digest() -> Digest {
    use sha2::{Digest as Hash, Sha256};
    let bytes: Vec<_> = (0..8000)
        .flat_map(|i| source_value(i).to_le_bytes())
        .collect();
    Digest::from_sha256(Sha256::digest(bytes).into())
}
pub struct Audio {
    pub pending: bool,
    pub fail: bool,
    digest: Digest,
}
impl Default for Audio {
    fn default() -> Self {
        Self {
            pending: false,
            fail: false,
            digest: source_digest(),
        }
    }
}
impl AudioResources for Audio {
    fn describe(&mut self, b: &MediaBinding) -> mo_video_audio::Result<AudioAsset> {
        assert_eq!(b.resource.as_str(), "tone");
        assert_eq!(b.digest, self.digest);
        assert_eq!(b.stream.as_str(), "mono");
        assert_eq!(
            b.descriptor,
            MediaStream::Audio {
                available: range(0, 48000),
                sample_rate: 8000,
                channels: 1
            }
        );
        Ok(AudioAsset {
            sample_rate: 8000,
            layout: ChannelLayout::mono(),
            sample_zero: R::integer(0),
            available: PcmRange::new(0, 8000)?,
        })
    }
    fn read(
        &mut self,
        b: &MediaBinding,
        asset: &AudioAsset,
        range: PcmRange,
        limits: PcmLimits,
        check: &dyn Fn() -> bool,
    ) -> mo_video_audio::Result<PcmBlock> {
        if self.fail {
            return Err(AudioError::Resource("test source failed".into()));
        }
        if self.pending {
            return Err(AudioError::PendingSource {
                resource: b.resource.clone(),
                first_sample: range.first,
                end_sample: range.end,
            });
        }
        let samples = (range.first..range.end).map(source_value).collect();
        PcmBlock::new(
            asset.sample_rate,
            asset.layout.clone(),
            range.first,
            samples,
            limits,
            check,
        )
    }
    fn channel_matrix(
        &mut self,
        input: &ChannelLayout,
        output: &ChannelLayout,
    ) -> mo_video_audio::Result<ChannelMatrix> {
        if input == output {
            Ok(ChannelMatrix::identity(input.clone()))
        } else if input == &ChannelLayout::mono() && output == &ChannelLayout::stereo() {
            Ok(ChannelMatrix::mono_to_stereo())
        } else {
            Err(AudioError::Unsupported("fixture channel map"))
        }
    }
}
