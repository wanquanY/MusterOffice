#![cfg(feature = "native")]
#[path = "audio_movie/stream_store.rs"]
mod stream_store;
use mo_video_runtime::{
    audio::*, audio_movie::*, isobmff::*, media::*, native::*, pcm::*, time::ExactRatio as R,
};
use sha2::{Digest as _, Sha256};
use std::{
    fs::{self, File},
    io::{Read, Seek, SeekFrom},
    path::Path,
};

fn reader(path: &Path) -> impl ReadAt + 'static {
    let mut file = File::open(path).unwrap();
    move |offset, out: &mut [u8]| {
        file.seek(SeekFrom::Start(offset))?;
        file.read(out)
    }
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
        process: mo_video_runtime::process_limits::ProcessLimits {
            additional_address_bytes: 512 << 20,
            cpu_seconds: 60,
            open_files: 32,
        },
        open_timeout: std::time::Duration::from_secs(15),
        call_timeout: std::time::Duration::from_secs(15),
    }
}
fn decode(
    bundle: &WorkerBundle,
    path: &Path,
    mode: DecodeTimeline,
) -> (InspectedPcm, Vec<f32>, u32) {
    let length = fs::metadata(path).unwrap().len();
    let container =
        NativeContainer::open_with_timeline(bundle, reader(path), length, options(), mode, &|| {
            false
        })
        .unwrap();
    let stream = container
        .streams()
        .iter()
        .find(|s| s.kind == MediaKind::Audio)
        .unwrap()
        .clone();
    let mut decoder = container.select(stream.index, &|| false).unwrap();
    let mut scan = PcmInspection::new(1, PcmInspectLimits::default()).unwrap();
    let mut samples = vec![];
    while let Some(frame) = decoder.next_frame(&|| false).unwrap() {
        let pcm = scan
            .observe_pcm(&frame, ByteOrder::Little, &|| false)
            .unwrap();
        assert_eq!(pcm.layout(), &ChannelLayout::mono());
        assert_eq!(pcm.first_frame(), samples.len() as i64);
        samples.extend_from_slice(pcm.samples());
    }
    decoder.close(&|| false).unwrap();
    (
        scan.finish_at_eof(&|| false).unwrap(),
        samples,
        stream.container_track_id.unwrap(),
    )
}
#[test]
#[ignore = "requires pinned worker, actual VFR/AAC fixture and output path"]
fn actual_raw_aac_movie_mapping_equals_fresh_demuxer_pcm() {
    let fixture =
        std::path::PathBuf::from(std::env::var_os("MO_AUDIO_MOVIE_FIXTURE").expect("fixture"));
    let output =
        std::path::PathBuf::from(std::env::var_os("MO_AUDIO_MOVIE_REPORT").expect("report"));
    assert!(!output.exists());
    let bundle = WorkerBundle::open(
        std::env::var_os("MO_MEDIA_TEST_BUNDLE")
            .expect("bundle")
            .into(),
        std::env::var("MO_MEDIA_TEST_BUNDLE_SHA256").expect("pin"),
    )
    .unwrap();
    let (raw, canonical, id) =
        stream_store::decode(&bundle, &fixture, &output.with_extension("tiles"));
    let raw_binding = canonical.source().binding().unwrap().clone();
    let tile_count = canonical.source().tile_count();
    let (demuxed, expected, other_id) = decode(&bundle, &fixture, DecodeTimeline::Demuxer);
    assert_eq!(id, other_id);
    let options = options();
    let mut index = NativeIndex::open(
        &bundle,
        reader(&fixture),
        fs::metadata(&fixture).unwrap().len(),
        NativeIndexOptions {
            index: IndexLimits::default(),
            max_read_bytes: options.decode.max_read_bytes,
            process: options.process,
            open_timeout: options.open_timeout,
            call_timeout: options.call_timeout,
        },
        &|| false,
    )
    .unwrap();
    let mut selected = None;
    for ordinal in 0..index.summary().tracks {
        if let IndexPage::Track { track } = index
            .query(IndexQuery::Track { track: ordinal }, &|| false)
            .unwrap()
            && track.id == id
        {
            assert_eq!(track.handler, *b"soun");
            selected = Some(ordinal);
        }
    }
    let ordinal = selected.expect("indexed raw audio stream");
    let timeline = index
        .movie_timeline(
            ordinal,
            &R::from_time(raw.decoded_range(256).unwrap().end),
            presentation::TimingLimits::default(),
            &|| false,
        )
        .unwrap();
    assert_eq!(timeline.track_id(), id);
    assert!(
        index
            .movie_timeline(
                ordinal,
                &R::integer(0),
                presentation::TimingLimits {
                    max_edits: 0,
                    ..Default::default()
                },
                &|| false
            )
            .is_err()
    );
    let edits = timeline
        .segments()
        .iter()
        .map(|s| serde_json::json!({"movie":s.movie(),"media":s.media_origin(),"rate":s.rate()}))
        .collect::<Vec<_>>();
    index.close(&|| false).unwrap();
    let plan = MovieAudioPlan::new(
        &timeline,
        raw_binding.clone(),
        &raw,
        MovieAudioLimits::default(),
        &|| false,
    )
    .unwrap();
    let mut movie_binding = raw_binding.clone();
    movie_binding.descriptor = plan.descriptor().clone();
    let asset = plan.asset().clone();
    let mut source = MovieAudioResource::new(plan, movie_binding.clone(), canonical).unwrap();
    let mut actual = vec![];
    let mut first = 0;
    while first < asset.available.end {
        let end = (first + 997).min(asset.available.end);
        let pcm = source
            .read(
                &movie_binding,
                &asset,
                PcmRange::new(first, end).unwrap(),
                PcmLimits::default(),
                &|| false,
            )
            .unwrap();
        actual.extend_from_slice(pcm.samples());
        first = end;
    }
    assert_eq!(asset.available.end, 144000);
    assert_eq!(demuxed.asset().available.end, 144000);
    assert_eq!(actual.len(), expected.len());
    for (index, (actual, expected)) in actual.iter().zip(&expected).enumerate() {
        assert_eq!(
            actual.to_bits(),
            expected.to_bits(),
            "mapped PCM bits differ at {index}"
        );
    }
    for first in [143999, 1000, 60000, 0, 80000] {
        let end = (first + 719).min(asset.available.end);
        let block = source
            .read(
                &movie_binding,
                &asset,
                PcmRange::new(first, end).unwrap(),
                PcmLimits::default(),
                &|| false,
            )
            .unwrap();
        assert_eq!(block.frames(), (end - first) as usize);
        assert!(
            block
                .samples()
                .iter()
                .zip(&expected[first as usize..end as usize])
                .all(|(a, b)| a.to_bits() == b.to_bits())
        );
    }
    let bytes: Vec<u8> = actual.iter().flat_map(|v| v.to_le_bytes()).collect();
    let report = serde_json::json!({
        "format":"musteroffice.audio-movie-native/1","status":"passed",
        "sourceSha256":movie_binding.digest,"workerManifestSha256":bundle.manifest_sha256(),
        "rawDecodeFrames":raw.decode_frames(),"rawPcmFrames":raw.asset().available.end,
        "rawRange":raw.decoded_range(256).unwrap(),"moviePcmFrames":asset.available.end,
        "movieEnd":timeline.end(),"edits":edits,"pcmSha256":format!("{:x}",Sha256::digest(bytes)),
        "reference":"fresh pinned worker demuxer decode, equal f32 bits",
        "filterBytes":source.retained_filter_bytes(),"randomWindows":5,
        "canonicalTiles":tile_count,"tileReads":source.raw_mut().store_mut().reads,
        "rawDecodePasses":1,"completionRecordRestored":true,
        "scope":"actual decoder, EOF-sealed file-backed PCM, saved-record restore and exact movie mapping; fixture host, not product persistence/playback"
    });
    fs::write(output, serde_json::to_vec_pretty(&report).unwrap()).unwrap();
}
