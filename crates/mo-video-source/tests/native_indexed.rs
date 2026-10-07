#![cfg(feature = "native")]
#[path = "native_indexed/support.rs"]
mod support;
use mo_common::{Digest as ContentDigest, RationalTime, ResourceId};
use mo_native_media::{
    IndexLimits, IndexPage, IndexQuery, NativeIndex, NativeIndexOptions, WorkerBundle,
};
use mo_video_compile::{MediaBinding, QueryLimits, VisualSource};
use mo_video_model::{Canvas, MediaStream, SampleSide, StreamId, TimeRange, TimeSample};
use mo_video_source::{
    indexed::*,
    native::{NativeVideoDecoder, NativeVideoTimeline},
    *,
};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    fs,
    path::{Path, PathBuf},
};

type Sampler = IndexedVideoSampler<NativeVideoDecoder<'static>, NativeVideoTimeline<'static>>;
fn source(bundle: &WorkerBundle, path: &Path) -> (Sampler, MediaBinding, [u32; 2]) {
    let options = support::options();
    let length = path.metadata().unwrap().len();
    let mut index = NativeIndex::open(
        bundle,
        support::reader(path),
        length,
        NativeIndexOptions {
            index: IndexLimits::default(),
            max_read_bytes: 64 << 20,
            process: options.process,
            open_timeout: options.open_timeout,
            call_timeout: options.call_timeout,
        },
        &|| false,
    )
    .unwrap();
    let ordinal = (0..index.summary().tracks)
        .find(|&i| {
            let IndexPage::Track { track } = index
                .query(IndexQuery::Track { track: i }, &|| false)
                .unwrap()
            else {
                panic!()
            };
            track.handler == *b"vide"
        })
        .unwrap();
    let index_pid = index.process_id().unwrap();
    let timeline = NativeVideoTimeline::new(index, ordinal, &|| false).unwrap();
    let summary = timeline.summary().clone();
    let decoder = NativeVideoDecoder::open_raw_track(
        bundle,
        support::reader(path),
        length,
        options,
        summary.track_id,
        &|| false,
    )
    .unwrap();
    let decoder_pid = decoder.process_id().unwrap();
    let dimensions = Canvas {
        width: decoder.stream().width.unwrap(),
        height: decoder.stream().height.unwrap(),
    };
    let binding = MediaBinding {
        resource: ResourceId::new("actual-source").unwrap(),
        digest: ContentDigest::from_sha256(Sha256::digest(fs::read(path).unwrap()).into()),
        stream: StreamId::new(format!("track-{}", summary.track_id)).unwrap(),
        descriptor: MediaStream::Video {
            available: TimeRange {
                start: RationalTime::new(0, 1).unwrap(),
                end: summary.movie_end.to_time_exact().unwrap(),
            },
            dimensions,
            frame_rate: None,
        },
    };
    let frame_bytes = dimensions.width as usize * dimensions.height as usize * 3 / 2;
    let limits = SourceLimits {
        exact_bits: 256,
        max_frame_bytes: frame_bytes,
        max_retained_bytes: frame_bytes * 4,
        max_retained_frames: 4,
        max_decode_calls_per_query: 1000,
    };
    (
        IndexedVideoSampler::new(decoder, timeline, binding.clone(), limits).unwrap(),
        binding,
        [index_pid, decoder_pid],
    )
}
fn record(sample: &TimeSample, value: IndexedSampledVideo) -> Value {
    let picture=value.frame().map(|frame| {
        let mut hash=Sha256::new(); for plane in &frame.planes { hash.update(&plane.bytes); }
        json!({"pts":frame.info.pts_ticks,"timeBase":frame.info.time_base,"generation":frame.info.generation,
            "sha256":format!("{:x}",hash.finalize()),"bytes":frame.planes.iter().map(|p|p.bytes.len()).sum::<usize>()})
    });
    json!({"time":sample.time,"side":sample.side,"selection":value.selection(),"frameSha256":picture.as_ref().map(|p|&p["sha256"]),"picture":picture,
        "stats":{"decodedFrames":value.stats.decoded_frames,"seeks":value.stats.seeks,"cacheHit":value.stats.cache_hit,"retainedBytes":value.stats.retained_bytes}})
}
#[test]
#[ignore = "requires actual media, independent timing evidence, pinned worker and output"]
fn actual_indexed_source_executes_compiled_edits_reverse_and_boundary_scrubbing() {
    let input = PathBuf::from(std::env::var_os("MO_INDEXED_INPUT").unwrap());
    let evidence: Value =
        serde_json::from_slice(&fs::read(std::env::var_os("MO_INDEXED_TIMING").unwrap()).unwrap())
            .unwrap();
    let bundle = WorkerBundle::open(
        PathBuf::from(std::env::var_os("MO_MEDIA_TEST_BUNDLE").unwrap()),
        std::env::var("MO_MEDIA_TEST_BUNDLE_SHA256").unwrap(),
    )
    .unwrap();
    let (mut sampler, binding, pids) = source(&bundle, &input);
    let mut rows = Vec::new();
    // These points cover every mapped edit/sample boundary, including gaps.
    for row in evidence["queries"].as_array().unwrap().iter().rev() {
        let sample = TimeSample {
            time: serde_json::from_value(row["time"].clone()).unwrap(),
            side: serde_json::from_value(row["side"].clone()).unwrap(),
        };
        let result = sampler
            .sample(&binding, &sample, SampleMode::Seek, &|| false)
            .unwrap_or_else(|error| {
                panic!(
                    "sample {sample:?}, indexed expectation {}: {error}",
                    row["selection"]
                )
            });
        rows.push(record(&sample, result));
    }
    let mut plans = Vec::new();
    for reverse in [false, true] {
        let doc = support::document(&binding, input.metadata().unwrap().len(), reverse);
        for number in 0..doc.frame_count().unwrap() {
            let plan = doc
                .frame(number, QueryLimits::default(), &|| false)
                .unwrap()
                .value;
            let VisualSource::Media {
                binding: actual,
                mapping,
            } = &plan.layers[0].source
            else {
                panic!("compiled media")
            };
            let result = sampler
                .sample(actual, &mapping.sample, SampleMode::Continue, &|| false)
                .unwrap();
            let mut row = record(&mapping.sample, result);
            row["reverse"] = json!(reverse);
            row["frameNumber"] = json!(number);
            plans.push(row);
        }
    }
    let retained = rows
        .iter()
        .map(|row| row["stats"]["retainedBytes"].as_u64().unwrap())
        .max()
        .unwrap();
    assert!(retained > 0);
    drop(sampler);
    for pid in pids {
        support::reaped(pid);
    }
    // Exercise actual owner cleanup after cancellation, including a cached frame.
    let (mut sampler, _, pids) = source(&bundle, &input);
    let sample = plans.iter().find(|r| !r["picture"].is_null()).unwrap();
    let sample = TimeSample {
        time: serde_json::from_value(sample["time"].clone()).unwrap(),
        side: SampleSide::After,
    };
    sampler
        .sample(&binding, &sample, SampleMode::Seek, &|| false)
        .unwrap();
    assert!(
        sampler
            .sample(&binding, &sample, SampleMode::Continue, &|| true)
            .is_err()
    );
    assert!(sampler.is_stopped());
    assert_eq!(sampler.retained_bytes(), 0);
    for pid in pids {
        support::reaped(pid);
    }
    fs::write(
        std::env::var_os("MO_INDEXED_OUTPUT").unwrap(),
        serde_json::to_vec_pretty(&json!({"binding":binding,"scrub":rows,"plans":plans})).unwrap(),
    )
    .unwrap();
}
