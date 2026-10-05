#![cfg(feature = "native")]
use mo_common::{ByteLength, Digest as ContentDigest, DocumentId, RationalTime, ResourceId};
use mo_media::DecodeLimits;
use mo_native_limits::ProcessLimits;
use mo_native_media::{WorkerBundle, WorkerOptions};
use mo_video_compile::{CompileLimits, CompiledDocument, MediaBinding, QueryLimits, VisualSource};
use mo_video_model::*;
use mo_video_source::{native::NativeVideoDecoder, *};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    fs::{self, File},
    io::{Read, Seek, SeekFrom},
    path::{Path, PathBuf},
    time::Duration,
};

fn time(n: i64) -> RationalTime {
    RationalTime::new(n, 30000).unwrap()
}
fn span() -> TimeRange {
    TimeRange {
        start: time(0),
        end: time(90090),
    }
}
fn make_document(binding: &MediaBinding, length: u64, reverse: bool) -> CompiledDocument {
    let root = SequenceId::new("root").unwrap();
    let track = TrackId::new("v1").unwrap();
    let clip = ClipId::new("clip1").unwrap();
    let mut doc = VideoDocument::empty(
        DocumentId::new("actual-media").unwrap(),
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
    doc.resources.insert(
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
    doc.sequences
        .get_mut(&root)
        .unwrap()
        .tracks
        .push(track.clone());
    doc.tracks.insert(
        track.clone(),
        Track {
            id: track,
            name: "Video".into(),
            kind: TrackKind::Video,
            locked: false,
            muted: false,
            solo: false,
            clips: vec![clip.clone()],
        },
    );
    let (a, b) = if reverse {
        (span().end, span().start)
    } else {
        (span().start, span().end)
    };
    doc.clips.insert(
        clip.clone(),
        Clip {
            id: clip,
            name: "Generated fixture".into(),
            timeline: span(),
            source: ClipSource::Media {
                resource: binding.resource.clone(),
                stream: binding.stream.clone(),
                time_map: TimeMap {
                    anchors: vec![
                        TimeAnchor {
                            local: time(0),
                            source: a,
                        },
                        TimeAnchor {
                            local: span().end,
                            source: b,
                        },
                    ],
                },
            },
            effects: vec![],
            audio: AudioTimePolicy::default(),
        },
    );
    CompiledDocument::new(doc, CompileLimits::default(), &|| false).unwrap()
}
fn options() -> WorkerOptions {
    WorkerOptions {
        decode: DecodeLimits {
            max_input_bytes: 1 << 30,
            max_read_bytes: 1 << 32,
            max_frame_bytes: 1 << 20,
            max_streams: 16,
            max_pixels: 1 << 24,
            max_audio_samples: 262144,
            max_channels: 8,
            max_decode_steps: 10000,
        },
        process: ProcessLimits {
            additional_address_bytes: 512 << 20,
            cpu_seconds: 60,
            open_files: 32,
        },
        open_timeout: Duration::from_secs(15),
        call_timeout: Duration::from_secs(10),
    }
}
fn decoder(bundle: &WorkerBundle, path: &Path) -> NativeVideoDecoder<'static> {
    let mut file = File::open(path).unwrap();
    let length = file.metadata().unwrap().len();
    NativeVideoDecoder::open(
        bundle,
        move |offset, out: &mut [u8]| {
            file.seek(SeekFrom::Start(offset))?;
            file.read(out)
        },
        length,
        options(),
        0,
        &|| false,
    )
    .unwrap()
}
fn record(sample: &TimeSample, result: SampledVideo) -> Value {
    let SourceSelection::Frame(frame) = result.selection else {
        panic!("{:?}", result.selection)
    };
    let mut hash = Sha256::new();
    for plane in &frame.frame.planes {
        hash.update(&plane.bytes);
    }
    json!({"sample":sample,"interval":frame.interval,"pts":frame.frame.info.pts_ticks,"timeBase":frame.frame.info.time_base,
        "generation":frame.frame.info.generation,"sha256":format!("{:x}",hash.finalize()),"decodedFrames":result.stats.decoded_frames,
        "seeks":result.stats.seeks,"cacheHit":result.stats.cache_hit,"retainedBytes":result.stats.retained_bytes})
}
#[test]
#[ignore = "requires a pinned worker bundle, generated VFR fixture and independent runner"]
fn compiled_forward_reverse_and_scrub_select_actual_worker_frames() {
    let fixture = PathBuf::from(std::env::var_os("MO_MEDIA_TEST_FIXTURE").expect("fixture"));
    let output = PathBuf::from(std::env::var_os("MO_VIDEO_SOURCE_TEST_OUTPUT").expect("output"));
    let bundle = WorkerBundle::open(
        PathBuf::from(std::env::var_os("MO_MEDIA_TEST_BUNDLE").expect("bundle")),
        std::env::var("MO_MEDIA_TEST_BUNDLE_SHA256").expect("pin"),
    )
    .unwrap();
    let digest = ContentDigest::from_sha256(Sha256::digest(fs::read(&fixture).unwrap()).into());
    let binding = MediaBinding {
        resource: ResourceId::new("original").unwrap(),
        digest,
        stream: StreamId::new("video0").unwrap(),
        descriptor: MediaStream::Video {
            available: span(),
            dimensions: Canvas {
                width: 128,
                height: 96,
            },
            frame_rate: None,
        },
    };
    let limits = SourceLimits {
        max_frame_bytes: 128 * 96 * 3 / 2,
        max_retained_bytes: 128 * 96 * 3 / 2 * 12,
        max_retained_frames: 12,
        max_decode_calls_per_query: 1000,
        exact_bits: 256,
    };
    let mut output_rows = serde_json::Map::new();
    for (name, reverse) in [("forward", false), ("reverse", true)] {
        let source = decoder(&bundle, &fixture);
        assert_eq!(source.stream().start_ticks, Some(0));
        // This demux estimate differs from the independently checked single edit
        // span (3.003 s). The test runner verifies the container edit, not FPS.
        assert_eq!(source.stream().duration_ticks, Some(89089));
        let doc = make_document(&binding, fs::metadata(&fixture).unwrap().len(), reverse);
        assert_eq!(doc.frame_count().unwrap(), 90);
        let mut sampler = VideoSampler::new(source, binding.clone(), span(), limits).unwrap();
        let mut rows = Vec::new();
        for index in 0..doc.frame_count().unwrap() {
            let plan = doc
                .frame(index, QueryLimits::default(), &|| false)
                .unwrap()
                .value;
            let VisualSource::Media {
                binding: actual,
                mapping,
            } = &plan.layers[0].source
            else {
                panic!("media plan");
            };
            let result = sampler
                .sample(actual, &mapping.sample, SampleMode::Continue, &|| false)
                .unwrap();
            assert!(result.stats.retained_bytes <= limits.max_retained_bytes);
            rows.push(record(&mapping.sample, result));
        }
        output_rows.insert(name.into(), Value::Array(rows));
    }
    let mut sampler =
        VideoSampler::new(decoder(&bundle, &fixture), binding.clone(), span(), limits).unwrap();
    let mut scrubs = Vec::new();
    for (n, side) in [
        (72072, SampleSide::Before),
        (1001, SampleSide::After),
        (89089, SampleSide::After),
        (4004, SampleSide::Before),
        (6006, SampleSide::After),
        (3003, SampleSide::After),
        (0, SampleSide::After),
        (90090, SampleSide::Before),
    ] {
        let sample = TimeSample {
            time: mo_time::ExactRatio::from_time(time(n)),
            side,
        };
        let result = sampler
            .sample(&binding, &sample, SampleMode::Seek, &|| false)
            .unwrap();
        scrubs.push(record(&sample, result));
    }
    output_rows.insert("scrub".into(), Value::Array(scrubs));
    fs::write(output, serde_json::to_vec_pretty(&output_rows).unwrap()).unwrap();
}
