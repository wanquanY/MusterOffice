mod support;
use mo_common::Digest;
use mo_video_audio::*;
use mo_video_pcm::*;
use std::{cell::Cell, sync::Arc};
use support::*;
#[test]
fn arbitrary_decode_partitions_produce_identical_canonical_tiles_and_checksums() {
    let source = source(-37, 8197, 256);
    let a = materialize(Arc::clone(&source), &[8192, 7]);
    let b = materialize(source, &[1, 997, 13, 1024, 7]);
    assert_eq!(a.tiles.len(), 33);
    for (index, tile) in a.tiles {
        let other = &b.tiles[&index];
        assert_eq!(tile.reference, other.reference);
        assert_eq!(tile.bytes, other.bytes);
    }
}
#[test]
fn builder_trims_only_attested_edges_and_rejects_gaps_overlaps_and_early_eof() {
    let source = source(-37, 1000, 256);
    let builder = PcmTileBuilder::new(source);
    let prefix = builder
        .prepare(&block(-100, -50), PcmBuildLimits::default(), &|| false)
        .unwrap();
    assert!(prefix.tiles.is_empty());
    let candidate = prefix
        .next
        .prepare(&block(-50, 1030), PcmBuildLimits::default(), &|| false)
        .unwrap();
    assert!(candidate.next.complete());
    assert_eq!(candidate.tiles.len(), 5);
    assert_eq!(candidate.tiles[0].reference.first_sample.get(), -37);
    assert_eq!(candidate.tiles[4].reference.frames, 13);
    assert!(builder.verify_eof().is_err());
    assert_eq!(builder.completed_tiles(), 0);
    assert!(
        builder
            .prepare(&block(-36, 100), PcmBuildLimits::default(), &|| false)
            .is_err()
    );
    for first in [-51, -49] {
        assert!(
            prefix
                .next
                .prepare(&block(first, 100), PcmBuildLimits::default(), &|| false)
                .is_err()
        );
    }
}
#[test]
fn random_reverse_windows_equal_the_continuous_stream_under_bounded_cache() {
    let source = source(-37, 10000, 256);
    let mut store = materialize(Arc::clone(&source), &[997]);
    let mut reader = PcmTileReader::new(
        Arc::clone(&source),
        PcmReadLimits {
            max_cached_bytes: 2048,
            max_cached_tiles: 2,
            max_working_bytes: 16384,
            ..PcmReadLimits::default()
        },
    )
    .unwrap();
    for i in 0..120 {
        let first = if i < 60 {
            -37 + i * 151
        } else {
            9900 - (i - 60) * 157
        };
        let end = (first + 87).min(10000);
        let read = reader
            .read(
                source.binding().unwrap(),
                PcmRange::new(first, end).unwrap(),
                &mut store,
                &|| false,
            )
            .unwrap();
        assert_eq!(read.pcm.samples(), block(first, end).samples());
        assert!(read.stats.peak_working_bytes <= 16384);
        assert!(read.stats.retained_bytes <= 2048);
    }
    let reads = store.reads;
    reader
        .read(
            source.binding().unwrap(),
            PcmRange::new(637, 650).unwrap(),
            &mut store,
            &|| false,
        )
        .unwrap();
    let hit = reader
        .read(
            source.binding().unwrap(),
            PcmRange::new(638, 649).unwrap(),
            &mut store,
            &|| false,
        )
        .unwrap();
    assert_eq!(hit.stats.cache_hits, 1);
    assert!(store.reads <= reads + 1);
}
#[test]
fn identity_is_content_based_and_invalidates_on_decoder_or_geometry_changes() {
    let a = source(0, 1024, 256);
    let mut binding = a.binding().unwrap().clone();
    binding.resource = mo_common::ResourceId::new("other-project-resource").unwrap();
    let same = PcmSource::new(
        binding,
        a.asset().clone(),
        &Digest::from_sha256([2; 32]),
        256,
        256,
    )
    .unwrap();
    assert_eq!(a.identity(), same.identity());
    for (profile, frames) in [(3, 256), (2, 512)] {
        let b = PcmSource::new(
            a.binding().unwrap().clone(),
            a.asset().clone(),
            &Digest::from_sha256([profile; 32]),
            frames,
            256,
        )
        .unwrap();
        assert_ne!(a.identity(), b.identity());
    }
}
#[test]
fn missing_corrupted_or_misbound_tiles_never_become_silence_or_nearest_data() {
    let source = source(0, 1024, 256);
    let mut store = Memory::default();
    let mut reader = PcmTileReader::new(Arc::clone(&source), PcmReadLimits::default()).unwrap();
    assert!(matches!(
        reader.read(
            source.binding().unwrap(),
            PcmRange::new(500, 600).unwrap(),
            &mut store,
            &|| false
        ),
        Err(PcmError::Missing { index: 1 })
    ));
    store = materialize(Arc::clone(&source), &[1024]);
    store.wrong_index = true;
    assert!(matches!(
        reader.read(
            source.binding().unwrap(),
            PcmRange::new(500, 600).unwrap(),
            &mut store,
            &|| false
        ),
        Err(PcmError::Invalid("PCM tile lookup index"))
    ));
    store.wrong_index = false;
    store.tiles.get_mut(&1).unwrap().bytes[1] ^= 1;
    assert!(matches!(
        reader.read(
            source.binding().unwrap(),
            PcmRange::new(500, 600).unwrap(),
            &mut store,
            &|| false
        ),
        Err(PcmError::Invalid("PCM tile checksum"))
    ));
    store.tiles.get_mut(&1).unwrap().bytes[1] ^= 1;
    store.tiles.get_mut(&1).unwrap().reference.first_sample = mo_common::Ticks::new(255);
    assert!(
        reader
            .read(
                source.binding().unwrap(),
                PcmRange::new(500, 600).unwrap(),
                &mut store,
                &|| false
            )
            .is_err()
    );
    let mut wrong = source.binding().unwrap().clone();
    wrong.digest = Digest::from_sha256([3; 32]);
    assert!(
        reader
            .read(&wrong, PcmRange::new(0, 10).unwrap(), &mut store, &|| false)
            .is_err()
    );
}
#[test]
fn candidate_cancellation_and_budgets_leave_the_original_builder_unchanged() {
    let source = source(0, 2000, 256);
    let builder = PcmTileBuilder::new(source);
    let input = block(0, 1500);
    let calls = Cell::new(0);
    let expected = builder
        .prepare(&input, PcmBuildLimits::default(), &|| {
            calls.set(calls.get() + 1);
            false
        })
        .unwrap();
    for stop in 1..=calls.get() {
        let count = Cell::new(0);
        assert!(
            builder
                .prepare(&input, PcmBuildLimits::default(), &|| {
                    count.set(count.get() + 1);
                    count.get() == stop
                })
                .is_err()
        );
        assert_eq!(builder.completed_tiles(), 0);
        assert_eq!(builder.retained_bytes(), 0);
    }
    assert_eq!(
        builder
            .prepare(&input, PcmBuildLimits::default(), &|| false)
            .unwrap()
            .tiles[0]
            .reference,
        expected.tiles[0].reference
    );
    assert!(matches!(
        builder.prepare(
            &input,
            PcmBuildLimits {
                max_candidate_bytes: 100,
                max_tiles_per_step: 100
            },
            &|| false
        ),
        Err(PcmError::Budget)
    ));
}

#[test]
fn read_cancellation_and_memory_budgets_never_return_partial_audio() {
    let source = source(0, 2000, 256);
    let mut store = materialize(Arc::clone(&source), &[500]);
    let limits = PcmReadLimits {
        max_cached_tiles: 0,
        ..PcmReadLimits::default()
    };
    let mut reader = PcmTileReader::new(Arc::clone(&source), limits).unwrap();
    let range = PcmRange::new(77, 1537).unwrap();
    let calls = Cell::new(0);
    let expected = reader
        .read(source.binding().unwrap(), range, &mut store, &|| {
            calls.set(calls.get() + 1);
            false
        })
        .unwrap();
    for stop in 1..=calls.get() {
        let count = Cell::new(0);
        assert!(
            reader
                .read(source.binding().unwrap(), range, &mut store, &|| {
                    count.set(count.get() + 1);
                    count.get() == stop
                })
                .is_err()
        );
        assert_eq!(
            reader
                .read(source.binding().unwrap(), range, &mut store, &|| false)
                .unwrap()
                .pcm
                .samples(),
            expected.pcm.samples()
        );
    }
    let mut reader = PcmTileReader::new(
        Arc::clone(&source),
        PcmReadLimits {
            max_working_bytes: 500,
            max_cached_bytes: 0,
            ..PcmReadLimits::default()
        },
    )
    .unwrap();
    assert!(matches!(
        reader.read(
            source.binding().unwrap(),
            PcmRange::new(0, 10).unwrap(),
            &mut store,
            &|| false
        ),
        Err(PcmError::Budget)
    ));
}
#[test]
fn timestamp_normalization_requires_exact_lattice_and_metadata_binding() {
    use mo_media::{DecodedFrame, FramePlane};
    let source = source(-37, 2000, 256);
    let mut frame=DecodedFrame{info:serde_json::from_value(serde_json::json!({"generation":1,"kind":"audio","timeBase":{"numerator":1,"denominator":16000},"ptsTicks":-6,"durationTicks":9999999,"width":null,"height":null,"sampleRate":8000,"channels":1,"samples":2,"format":"flt","channelLayout":"mono","colors":{"primaries":2,"transfer":2,"matrix":2,"range":0,"chromaLocation":0},"sampleAspectRatio":null,"alphaMode":0,"keyFrame":true,"interlaced":false,"topFieldFirst":false})).unwrap(),planes:vec![FramePlane{row_bytes:8,rows:1,bytes:[0.25f32,-0.5].iter().flat_map(|v|v.to_le_bytes()).collect()}]};
    let normalize = |f: &DecodedFrame| {
        source.normalize(f, ByteOrder::Little, 1, PcmLimits::default(), &|| false)
    };
    let pcm = normalize(&frame).unwrap();
    assert_eq!(pcm.first_frame(), -3);
    assert_eq!(pcm.end_frame(), -1);
    assert_eq!(pcm.samples(), [0.25, -0.5]);
    frame.info.pts_ticks = Some(-5);
    assert!(normalize(&frame).is_err());
    frame.info.pts_ticks = None;
    assert!(normalize(&frame).is_err());
    frame.info.pts_ticks = Some(-6);
    frame.info.generation = 2;
    assert!(normalize(&frame).is_err());
    frame.info.generation = 1;
    frame.info.channel_layout = "stereo".into();
    assert!(normalize(&frame).is_err());
}
