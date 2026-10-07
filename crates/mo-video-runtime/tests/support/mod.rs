#![allow(dead_code)]
use mo_video_runtime::{
    authoring::model::*,
    common::*,
    compile::*,
    media::*,
    source::{indexed::*, *},
};
use mo_video_runtime::{
    isobmff::{ByteRange, Edit, Sample, TrackIndex},
    visual_source::*,
};
use std::{
    cell::{Cell, RefCell},
    collections::BTreeMap,
    rc::Rc,
};
pub fn t(n: i64) -> RationalTime {
    RationalTime::new(n, 1).unwrap()
}
pub fn binding() -> MediaBinding {
    MediaBinding {
        resource: ResourceId::new("video").unwrap(),
        digest: Digest::from_sha256([1; 32]),
        stream: StreamId::new("v").unwrap(),
        descriptor: MediaStream::Video {
            available: TimeRange {
                start: t(0),
                end: t(4),
            },
            dimensions: Canvas {
                width: 2,
                height: 2,
            },
            frame_rate: None,
        },
    }
}
pub fn track() -> TrackIndex {
    TrackIndex {
        id: 7,
        flags: 1,
        handler: *b"vide",
        declared_track_duration: Some(4000),
        timescale: 1_000_003,
        declared_media_duration: Some(1_500_004),
        decode_duration: 1_500_004,
        display_matrix: [0; 9],
        display_size_16_16: [2 * 65536; 2],
        descriptions: vec![],
        auxiliary: vec![],
        edits: vec![
            Edit {
                movie_start: 0,
                movie_duration: 1000,
                media_time: None,
                rate_16_16: 65536,
            },
            Edit {
                movie_start: 1000,
                movie_duration: 1000,
                media_time: Some(0),
                rate_16_16: 98305,
            },
            Edit {
                movie_start: 2000,
                movie_duration: 1000,
                media_time: Some(500_001),
                rate_16_16: 0,
            },
            Edit {
                movie_start: 3000,
                movie_duration: 1000,
                media_time: Some(0),
                rate_16_16: 65536,
            },
        ],
        samples: [(0, 500_001), (500_001, 500_002), (1_000_003, 500_001)]
            .into_iter()
            .enumerate()
            .map(|(i, (pts, duration))| Sample {
                bytes: ByteRange {
                    offset: i as u64,
                    length: 1,
                },
                dts: pts,
                pts,
                decode_duration: duration,
                description: 0,
                sync_sample: true,
            })
            .collect(),
    }
}
#[derive(Default)]
pub struct Observed {
    pub decoder_drops: Cell<usize>,
    pub timeline_drops: Cell<usize>,
    pub timeline_calls: Cell<usize>,
    pub reads: Cell<usize>,
    pub seeks: RefCell<Vec<i64>>,
}
pub struct Decoder {
    pub stream: StreamInfo,
    pub observed: Rc<Observed>,
    pub broken_color: bool,
    pub broken_pixel: bool,
    generation: u64,
    cursor: usize,
}
impl Decoder {
    pub fn new(observed: Rc<Observed>) -> Self {
        Self {
            stream: StreamInfo {
                index: 0,
                container_track_id: Some(7),
                kind: MediaKind::Video,
                codec: "rawvideo".into(),
                time_base: Some(TimeBase {
                    numerator: 1,
                    denominator: 1_000_003,
                }),
                start_ticks: Some(0),
                duration_ticks: Some(1_500_004),
                width: Some(2),
                height: Some(2),
                sample_rate: None,
                channels: None,
                nominal_frame_rate: None,
                sample_aspect_ratio: Some(TimeBase {
                    numerator: 1,
                    denominator: 1,
                }),
                colors: colors(),
                channel_layout: String::new(),
                display_matrix: None,
            },
            observed,
            broken_color: false,
            broken_pixel: false,
            generation: 1,
            cursor: 0,
        }
    }
}
fn colors() -> CodecColorTags {
    CodecColorTags {
        primaries: 1,
        transfer: 13,
        matrix: 0,
        range: 2,
        chroma_location: 0,
    }
}
impl Drop for Decoder {
    fn drop(&mut self) {
        self.observed
            .decoder_drops
            .set(self.observed.decoder_drops.get() + 1);
    }
}
impl VideoDecoder for Decoder {
    type Error = std::io::Error;
    fn stream(&self) -> &StreamInfo {
        &self.stream
    }
    fn timeline(&self) -> DecodeTimeline {
        DecodeTimeline::RawIsoBmff
    }
    fn generation(&self) -> u64 {
        self.generation
    }
    fn seek(&mut self, ticks: i64, _: &dyn Fn() -> bool) -> std::io::Result<()> {
        self.observed.seeks.borrow_mut().push(ticks);
        self.generation += 1;
        self.cursor = [0, 500_001, 1_000_003].partition_point(|p| *p < ticks);
        Ok(())
    }
    fn next(&mut self, _: &dyn Fn() -> bool) -> std::io::Result<Option<DecodedFrame>> {
        self.observed.reads.set(self.observed.reads.get() + 1);
        if self.cursor >= 3 {
            return Ok(None);
        }
        let i = self.cursor;
        self.cursor += 1;
        let mut tags = colors();
        if self.broken_color {
            tags.primaries = 0;
        }
        let pixel = [[255, 0, 0, 255], [0, 255, 0, 128], [0, 0, 255, 255]][i];
        Ok(Some(DecodedFrame {
            info: FrameInfo {
                generation: self.generation,
                kind: MediaKind::Video,
                time_base: self.stream.time_base.unwrap(),
                pts_ticks: Some([0, 500_001, 1_000_003][i]),
                duration_ticks: Some([500_001, 500_002, 500_001][i]),
                width: Some(2),
                height: Some(2),
                sample_rate: None,
                channels: None,
                samples: None,
                format: if self.broken_pixel {
                    "unsupported"
                } else {
                    "rgba"
                }
                .into(),
                channel_layout: String::new(),
                colors: tags,
                sample_aspect_ratio: None,
                alpha_mode: 2,
                key_frame: true,
                interlaced: false,
                top_field_first: false,
            },
            planes: vec![FramePlane {
                row_bytes: 8,
                rows: 2,
                bytes: pixel.repeat(4),
            }],
        }))
    }
}
impl IndexedVideoDecoder for Decoder {
    fn seek_sample(
        &mut self,
        target: DecodeTarget,
        cancel: &dyn Fn() -> bool,
    ) -> std::io::Result<()> {
        self.seek(target.access_start, cancel)
    }
}
pub struct Timeline {
    pub inner: VideoTiming,
    pub observed: Rc<Observed>,
    pub fail: Rc<Cell<bool>>,
}
impl VideoTimeline for Timeline {
    type Error = TimingError;
    fn summary(&self) -> &VideoTimingSummary {
        self.inner.summary()
    }
    fn select(
        &mut self,
        time: &ExactRatio,
        side: Side,
        check: &dyn Fn() -> bool,
    ) -> std::result::Result<VideoTimeSelection, TimingError> {
        self.observed
            .timeline_calls
            .set(self.observed.timeline_calls.get() + 1);
        if self.fail.get() {
            return Err(TimingError::Invalid("timeline revoked"));
        }
        self.inner.select(time, side, check)
    }
}
impl Drop for Timeline {
    fn drop(&mut self) {
        self.observed
            .timeline_drops
            .set(self.observed.timeline_drops.get() + 1);
    }
}
pub type Source = IndexedVisualSource<Decoder, Timeline>;
pub fn source_with(decoder: Decoder, limits: VisualSourceLimits) -> (Source, Rc<Cell<bool>>) {
    let timeline = Timeline {
        inner: VideoTiming::build(&track(), 1000, Default::default(), &|| false).unwrap(),
        observed: decoder.observed.clone(),
        fail: Rc::default(),
    };
    let fail = timeline.fail.clone();
    let sampler = IndexedVideoSampler::new(
        decoder,
        timeline,
        binding(),
        SourceLimits {
            max_frame_bytes: 16,
            max_retained_bytes: 48,
            max_retained_frames: 3,
            ..Default::default()
        },
    )
    .unwrap();
    (
        IndexedVisualSource::new(sampler, ColorInterpretation::Declared { hlg: None }, limits)
            .unwrap(),
        fail,
    )
}
pub fn source() -> (Source, Rc<Observed>, Rc<Cell<bool>>) {
    let observed: Rc<Observed> = Rc::default();
    let (source, fail) = source_with(
        Decoder::new(observed.clone()),
        VisualSourceLimits {
            max_retained_bytes: 128,
            max_retained_frames: 2,
            ..Default::default()
        },
    );
    (source, observed, fail)
}
pub fn sample(n: i64, d: i64, side: SampleSide) -> TimeSample {
    TimeSample {
        time: ExactRatio::new(n.into(), d.into(), 256).unwrap(),
        side,
    }
}
pub fn document(binding: &MediaBinding, length: u64, reverse: bool) -> CompiledDocument {
    let root = SequenceId::new("main").unwrap();
    let mut d = VideoDocument::empty(
        DocumentId::new("indexed-composition").unwrap(),
        root.clone(),
        Canvas {
            width: 2,
            height: 2,
        },
        FrameRate {
            numerator: 24,
            denominator: 1,
        },
    );
    d.resources.insert(
        binding.resource.clone(),
        Resource {
            id: binding.resource.clone(),
            digest: binding.digest.clone(),
            byte_length: ByteLength::new(length),
            descriptor: ResourceDescriptor::Media {
                streams: BTreeMap::from([(binding.stream.clone(), binding.descriptor.clone())]),
            },
        },
    );
    let MediaStream::Video {
        available,
        dimensions,
        ..
    } = binding.descriptor
    else {
        panic!()
    };
    d.canvas = dimensions;
    for (name, kind, source) in [
        (
            "background",
            TrackKind::Graphics,
            ClipSource::Solid {
                rgba: [0, 0, 255, 255],
            },
        ),
        (
            "video",
            TrackKind::Video,
            ClipSource::Media {
                resource: binding.resource.clone(),
                stream: binding.stream.clone(),
                time_map: TimeMap {
                    anchors: vec![
                        TimeAnchor {
                            local: t(0),
                            source: if reverse {
                                available.end
                            } else {
                                available.start
                            },
                        },
                        TimeAnchor {
                            local: available.end,
                            source: if reverse {
                                available.start
                            } else {
                                available.end
                            },
                        },
                    ],
                },
            },
        ),
    ] {
        let id = TrackId::new(name).unwrap();
        let clip = ClipId::new(name).unwrap();
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
                clips: vec![clip.clone()],
            },
        );
        d.clips.insert(
            clip.clone(),
            Clip {
                id: clip,
                name: name.into(),
                timeline: available,
                source,
                effects: vec![],
                audio: Default::default(),
                audio_context: None,
            },
        );
    }
    CompiledDocument::new(d, Default::default(), &|| false).unwrap()
}
