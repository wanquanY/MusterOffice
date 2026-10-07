mod support;
use mo_media::{DecodeTimeline, DecodedFrame, MediaKind, ReadAt};
use mo_native_media::*;
use serde_json::json;
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    fs::File,
    io::{Read, Seek, SeekFrom},
    path::Path,
};
fn input(path: &Path) -> impl ReadAt + 'static {
    let mut file = File::open(path).unwrap();
    move |offset, out: &mut [u8]| {
        file.seek(SeekFrom::Start(offset))?;
        let n = out.len().min(997);
        file.read(&mut out[..n])
    }
}
fn digest(frame: &DecodedFrame) -> String {
    let mut hash = Sha256::new();
    for p in &frame.planes {
        hash.update(&p.bytes);
    }
    format!("{:x}", hash.finalize())
}
fn ratio(n: i64, d: u32) -> ExactRatio {
    ExactRatio::new(n.into(), d.into(), 256).unwrap()
}
#[test]
#[ignore = "requires actual ISO BMFF video, pinned worker and evidence output"]
fn actual_raw_decode_mapped_queries_and_seeks() {
    let path = std::env::var("MO_VIDEO_TIME_INPUT").expect("input");
    let path = Path::new(&path);
    let output = std::env::var("MO_VIDEO_TIME_OUTPUT").expect("output");
    let length = path.metadata().unwrap().len();
    let bundle = support::bundle();
    let base = support::options();
    let mut index = NativeIndex::open(
        &bundle,
        input(path),
        length,
        NativeIndexOptions {
            index: IndexLimits::default(),
            max_read_bytes: 64 << 20,
            process: base.process,
            open_timeout: base.open_timeout,
            call_timeout: base.call_timeout,
        },
        &|| false,
    )
    .unwrap();
    let (ordinal, track) = (0..index.summary().tracks)
        .find_map(|i| {
            let IndexPage::Track { track } = index
                .query(IndexQuery::Track { track: i }, &|| false)
                .unwrap()
            else {
                panic!("track")
            };
            (track.handler == *b"vide").then_some((i, track))
        })
        .expect("video track");
    let summary = index.describe_video_time(ordinal, &|| false).unwrap();
    assert_eq!(summary.track_id, track.id);
    assert_eq!(summary.media_timescale, track.timescale);
    let mut samples = Vec::new();
    let mut edits = Vec::new();
    for start in (0..track.samples).step_by(128) {
        let IndexPage::Samples { items, .. } = index
            .query(
                IndexQuery::Samples {
                    track: ordinal,
                    start,
                    count: 128,
                },
                &|| false,
            )
            .unwrap()
        else {
            panic!()
        };
        samples.extend(items);
    }
    for start in (0..track.edits).step_by(128) {
        let IndexPage::Edits { items, .. } = index
            .query(
                IndexQuery::Edits {
                    track: ordinal,
                    start,
                    count: 128,
                },
                &|| false,
            )
            .unwrap()
        else {
            panic!()
        };
        edits.extend(items);
    }
    let container = NativeContainer::open_with_timeline(
        &bundle,
        input(path),
        length,
        base,
        DecodeTimeline::RawIsoBmff,
        &|| false,
    )
    .unwrap();
    assert_eq!(container.timeline(), DecodeTimeline::RawIsoBmff);
    let stream = container
        .streams()
        .iter()
        .find(|s| s.container_track_id == Some(track.id))
        .expect("real tkhd identity")
        .clone();
    assert_eq!(stream.kind, MediaKind::Video);
    let mut decoder = container.select(stream.index, &|| false).unwrap();
    assert_eq!(decoder.timeline(), DecodeTimeline::RawIsoBmff);
    let mut frames = Vec::new();
    let mut by_pts = BTreeMap::new();
    while let Some(frame) = decoder.next_frame(&|| false).unwrap() {
        assert_eq!(frame.info.time_base.numerator, 1);
        assert_eq!(frame.info.time_base.denominator, track.timescale);
        assert_eq!(frame.info.generation, 1);
        let pts = frame.info.pts_ticks.expect("exact PTS");
        let hash = digest(&frame);
        assert!(by_pts.insert(pts, hash.clone()).is_none());
        frames.push(json!({"pts":pts,"sha256":hash,"bytes":frame.planes.iter().map(|p|p.bytes.len()).sum::<usize>()}));
    }
    let mut pts: Vec<_> = samples.iter().map(|s| s.pts).collect();
    pts.sort_unstable();
    assert_eq!(
        by_pts.keys().copied().collect::<Vec<_>>(),
        pts,
        "one decoded picture per indexed access unit in these fixtures"
    );
    let mut points = vec![
        ratio(-1, 1000),
        ExactRatio::integer(0),
        summary.movie_end.clone(),
    ];
    for i in 0..=40 {
        points.push(summary.movie_end.mul(&ratio(i, 40), 256).unwrap());
    }
    for edit in &edits {
        let start =
            ExactRatio::new(edit.movie_start.into(), summary.movie_timescale.into(), 256).unwrap();
        points.push(start.clone());
        let Some(media) = edit.media_time else {
            continue;
        };
        if edit.rate_16_16 <= 0 {
            continue;
        }
        for sample in &samples {
            points.push(
                start
                    .add(
                        &ratio(sample.pts - media, track.timescale)
                            .div(&ratio(i64::from(edit.rate_16_16), 65536), 256)
                            .unwrap(),
                        256,
                    )
                    .unwrap(),
            );
        }
    }
    let mut queries = Vec::new();
    let mut generation = 1;
    let mut seek_checks = 0;
    for (i, time) in points.iter().enumerate() {
        for side in [MediaTimeSide::Before, MediaTimeSide::After] {
            let selection = index
                .select_video_time(ordinal, time, side, &|| false)
                .unwrap();
            let mut frame_hash = None;
            if let VideoTimeSelection::Sample {
                sample,
                decode_start,
                access,
                raw_start,
                ..
            } = &selection
            {
                assert_eq!(samples[*sample as usize].pts, *raw_start);
                assert_eq!(samples[*sample as usize].dts, *decode_start);
                frame_hash = Some(by_pts[raw_start].clone());
                if i % 11 == 0 && side == MediaTimeSide::After {
                    decoder
                        .seek(access.expect("fixture access point").decode_start, &|| {
                            false
                        })
                        .unwrap();
                    generation += 1;
                    let mut found = false;
                    for _ in 0..1000 {
                        let Some(frame) = decoder.next_frame(&|| false).unwrap() else {
                            break;
                        };
                        assert_eq!(frame.info.generation, generation);
                        if frame.info.pts_ticks.unwrap() >= *raw_start {
                            assert_eq!(frame.info.pts_ticks, Some(*raw_start));
                            assert_eq!(digest(&frame), by_pts[raw_start]);
                            found = true;
                            break;
                        }
                    }
                    assert!(found, "indexed picture must remain decodable after seek");
                    seek_checks += 1;
                }
            }
            queries.push(
                json!({"time":time,"side":side,"selection":selection,"frameSha256":frame_hash}),
            );
        }
    }
    assert!(seek_checks > 0);
    let pid = decoder.process_id().unwrap();
    decoder.close(&|| false).unwrap();
    support::assert_reaped(pid);
    let pid = index.process_id().unwrap();
    index.close(&|| false).unwrap();
    support::assert_reaped(pid);
    std::fs::write(output,serde_json::to_vec_pretty(&json!({"summary":summary,"stream":stream,"track":track,
        "samples":samples,"edits":edits,"frames":frames,"queries":queries,"seekChecks":seek_checks})).unwrap()).unwrap();
}
#[test]
#[ignore = "requires explicit pinned worker"]
fn raw_timeline_does_not_fall_back_to_other_containers() {
    let bundle = support::bundle();
    let (bytes, _) = support::wav(4800);
    let result = NativeContainer::open_with_timeline(
        &bundle,
        |at, out: &mut [u8]| support::read(&bytes, at, out, usize::MAX),
        bytes.len() as u64,
        support::options(),
        DecodeTimeline::RawIsoBmff,
        &|| false,
    );
    assert!(result.is_err());
}
