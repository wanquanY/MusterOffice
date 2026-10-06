mod support;
use mo_media_isobmff::{IndexLimits, IsoBmffIndex};
use mo_native_media::{IndexPage, IndexQuery, NativeIndex, NativeIndexOptions, WorkerError};
use mo_native_worker::SessionError;
use serde_json::json;
use sha2::{Digest, Sha256};
use std::{
    cell::Cell,
    io::{Read, Seek, SeekFrom},
    time::Duration,
};

fn options() -> NativeIndexOptions {
    let base = support::options();
    NativeIndexOptions {
        index: IndexLimits::default(),
        max_read_bytes: 64 << 20,
        process: base.process,
        open_timeout: base.open_timeout,
        call_timeout: base.call_timeout,
    }
}
fn input() -> Vec<u8> {
    std::fs::read(std::env::var("MO_BMFF_INPUT").expect("real container input")).unwrap()
}
#[test]
#[ignore = "requires explicit worker and real media input/output"]
fn actual_index_pages_preserve_all_raw_metadata_and_sample_bytes() {
    let path = std::env::var("MO_BMFF_INPUT").expect("real input");
    let output = std::env::var("MO_BMFF_OUTPUT").expect("evidence output");
    let mut source = std::fs::File::open(&path).unwrap();
    let length = source.metadata().unwrap().len();
    let calls = Cell::new(0);
    let read_bytes = Cell::new(0u64);
    let mut index = NativeIndex::open(
        &support::bundle(),
        |offset, out: &mut [u8]| {
            source.seek(SeekFrom::Start(offset))?;
            let n = out.len().min(13);
            let n = source.read(&mut out[..n])?;
            calls.set(calls.get() + 1);
            read_bytes.set(read_bytes.get() + n as u64);
            Ok(n)
        },
        length,
        options(),
        &|| false,
    )
    .unwrap();
    let pid = index.process_id().unwrap();
    assert_ne!(pid, std::process::id());
    assert_eq!(index.applied_limits().cpu_seconds, 60);
    let summary = index.summary();
    assert_eq!(summary.bytes_read, read_bytes.get());
    let queries_after = calls.get();
    // Test reference uses the pure parser against independent file I/O; this
    // checks transport completeness. Python separately compares packet semantics
    // and hashes against a different demux implementation.
    let bytes = input();
    let expected = IsoBmffIndex::read(
        &mut |offset, out: &mut [u8]| support::read(&bytes, offset, out, usize::MAX),
        length,
        options().index,
        &|| false,
    )
    .unwrap();
    assert!(
        index
            .query(
                IndexQuery::Samples {
                    track: 0,
                    start: 0,
                    count: 129
                },
                &|| false
            )
            .is_err()
    );
    assert!(!index.is_stopped());
    let mut tracks = Vec::new();
    let mut packets = std::fs::File::open(&path).unwrap();
    for (t, reference) in expected.tracks.iter().enumerate() {
        let t = t as u32;
        let IndexPage::Track { track } = index
            .query(IndexQuery::Track { track: t }, &|| false)
            .unwrap()
        else {
            panic!("track")
        };
        assert_eq!(track, reference.into());
        let mut samples = Vec::new();
        let mut raw = Vec::new();
        for start in (0..track.samples).step_by(37) {
            let IndexPage::Samples { total, items } = index
                .query(
                    IndexQuery::Samples {
                        track: t,
                        start,
                        count: 37,
                    },
                    &|| false,
                )
                .unwrap()
            else {
                panic!("samples")
            };
            assert_eq!(total, track.samples);
            raw.extend(items);
        }
        assert_eq!(raw, reference.samples);
        for sample in raw {
            packets.seek(SeekFrom::Start(sample.bytes.offset)).unwrap();
            let mut data = vec![0; sample.bytes.length as usize];
            packets.read_exact(&mut data).unwrap();
            samples.push(json!({"dts":sample.dts,"pts":sample.pts,"duration":sample.decode_duration,
                "offset":sample.bytes.offset,"byteLength":sample.bytes.length,"sync":sample.sync_sample,
                "description":sample.description,"sha256":format!("{:x}",Sha256::digest(&data))}));
        }
        let mut edits = Vec::new();
        for start in 0..track.edits {
            let IndexPage::Edits { items, .. } = index
                .query(
                    IndexQuery::Edits {
                        track: t,
                        start,
                        count: 1,
                    },
                    &|| false,
                )
                .unwrap()
            else {
                panic!("edits")
            };
            edits.extend(items);
        }
        assert_eq!(edits, reference.edits);
        for auxiliary in [false, true] {
            let count = if auxiliary {
                track.auxiliary
            } else {
                track.descriptions
            };
            for b in 0..count {
                let (kind, want) = if auxiliary {
                    let e = &reference.auxiliary[b as usize];
                    (e.kind, &e.payload)
                } else {
                    let e = &reference.descriptions[b as usize];
                    (e.format, &e.payload)
                };
                let mut actual = Vec::new();
                loop {
                    let offset = actual.len() as u32;
                    let query = if auxiliary {
                        IndexQuery::Auxiliary {
                            track: t,
                            index: b,
                            offset,
                            length: 7,
                        }
                    } else {
                        IndexQuery::Description {
                            track: t,
                            index: b,
                            offset,
                            length: 7,
                        }
                    };
                    let IndexPage::Blob {
                        kind: k,
                        total,
                        bytes,
                    } = index.query(query, &|| false).unwrap()
                    else {
                        panic!("blob")
                    };
                    assert_eq!(k, kind);
                    actual.extend(bytes);
                    if actual.len() == total as usize {
                        break;
                    }
                }
                assert_eq!(&actual, want);
            }
        }
        tracks.push(json!({"id":track.id,"handler":String::from_utf8_lossy(&track.handler),
            "timescale":track.timescale,"declaredDuration":track.declared_media_duration,"decodeDuration":track.decode_duration,
            "flags":track.flags,"trackDuration":track.declared_track_duration,"displayMatrix":track.display_matrix,"displaySize16_16":track.display_size_16_16,
            "edits":edits.iter().map(|e|json!({"movieStart":e.movie_start,"movieDuration":e.movie_duration,"mediaTime":e.media_time,"rate16_16":e.rate_16_16})).collect::<Vec<_>>(),"samples":samples}));
    }
    let mut ranges = Vec::new();
    for start in 0..summary.media_data {
        let IndexPage::MediaData { items, .. } = index
            .query(IndexQuery::MediaData { start, count: 1 }, &|| false)
            .unwrap()
        else {
            panic!("mdat")
        };
        ranges.extend(items);
    }
    assert_eq!(ranges, expected.media_data);
    assert_eq!(
        calls.get(),
        queries_after,
        "queries must not restart parsing or re-read source"
    );
    index.close(&|| false).unwrap();
    support::assert_reaped(pid);
    let value = json!({"movieTimescale":summary.movie_timescale,"declaredMovieDuration":summary.declared_movie_duration,
        "bytesRead":summary.bytes_read,"fileLength":length,"tracks":tracks});
    std::fs::write(output, serde_json::to_vec_pretty(&value).unwrap()).unwrap();
}
#[test]
#[ignore = "requires explicit worker and real media input"]
fn indexing_budget_faults_cancellation_deadline_and_drop_are_failures() {
    let bytes = input();
    let bundle = support::bundle();
    let open = |options| {
        NativeIndex::open(
            &bundle,
            |offset, out: &mut [u8]| support::read(&bytes, offset, out, usize::MAX),
            bytes.len() as u64,
            options,
            &|| false,
        )
    };
    let mut small = options();
    small.index.max_samples = 1;
    assert!(matches!(
        open(small),
        Err(WorkerError::Media(mo_media::MediaError {
            kind: mo_media::ErrorKind::Limit,
            ..
        }))
    ));
    small = options();
    small.max_read_bytes = 1;
    assert!(matches!(open(small), Err(WorkerError::Invalid(_))));
    for mode in 0..4 {
        let cancel = Cell::new(false);
        let mut opts = options();
        opts.open_timeout = Duration::from_millis(500);
        let result = NativeIndex::open(
            &bundle,
            |offset, out: &mut [u8]| {
                match mode {
                    0 => panic!("injected read callback panic"),
                    1 => return Ok(out.len() + 1),
                    2 => cancel.set(true),
                    _ => std::thread::sleep(Duration::from_millis(600)),
                }
                support::read(&bytes, offset, out, 13)
            },
            bytes.len() as u64,
            opts,
            &|| cancel.get(),
        );
        assert!(matches!(
            (mode, result),
            (0, Err(WorkerError::CallbackPanic))
                | (1, Err(WorkerError::Invalid(_)))
                | (2, Err(WorkerError::Session(SessionError::Cancelled)))
                | (3, Err(WorkerError::Session(SessionError::Deadline)))
        ));
    }
    let mut index = open(options()).unwrap();
    let pid = index.process_id().unwrap();
    assert!(matches!(
        index.query(IndexQuery::Track { track: 0 }, &|| true),
        Err(WorkerError::Session(SessionError::Cancelled))
    ));
    assert!(index.is_stopped());
    support::assert_reaped(pid);
    assert!(matches!(
        index.query(IndexQuery::Track { track: 0 }, &|| false),
        Err(WorkerError::Session(SessionError::Stopped))
    ));
    let index = open(options()).unwrap();
    let pid = index.process_id().unwrap();
    drop(index);
    support::assert_reaped(pid);
    let mut index = open(options()).unwrap();
    let pid = index.process_id().unwrap();
    assert!(
        index
            .query(
                IndexQuery::Samples {
                    track: 0,
                    start: u32::MAX,
                    count: 1
                },
                &|| false
            )
            .is_err()
    );
    assert!(index.is_stopped());
    support::assert_reaped(pid);
}
