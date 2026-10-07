#[path = "stream/support.rs"]
mod fixture;
mod support;
use fixture::*;
use mo_common::{ByteLength, Digest, Ticks};
use mo_video_audio::{ByteOrder, PcmRange};
use mo_video_pcm::*;
use std::{cell::Cell, sync::Arc};

#[test]
fn unknown_endpoint_matches_known_extent_tiles_across_codec_partitions() {
    for channels in [1, 2] {
        let (a, tiles) = streamed(8199, &[1, 511, 3, 4096], channels);
        let (b, other) = streamed(8199, &[127, 31, 1024], channels);
        assert_eq!(a.source().identity(), b.source().identity());
        let mut known = PcmTileBuilder::new(Arc::clone(a.source()));
        let expected = known
            .prepare(&pcm(0, 8199, channels), PcmBuildLimits::default(), &|| {
                false
            })
            .unwrap();
        known = expected.next;
        known.verify_eof().unwrap();
        assert_eq!(tiles.len(), 33);
        for ((tile, other), expected) in tiles.iter().zip(&other).zip(&expected.tiles) {
            assert_eq!(tile.reference, other.reference);
            assert_eq!(tile.bytes, other.bytes);
            assert_eq!(a.seal_ref(&tile.reference).unwrap(), expected.reference);
            assert_eq!(tile.bytes, expected.bytes);
        }
        assert_eq!(tiles.last().unwrap().reference.frames, 7);
        assert_eq!(a.decoded().asset().available.end, 8199);
        assert_eq!(
            a.decoded().decoded_range(256).unwrap().start.ticks.get(),
            -3
        );
    }
}
#[test]
fn sealed_tiles_feed_existing_random_reader_without_redecode() {
    let (sealed, tiles) = streamed(1031, &[3, 97, 13], 2);
    let mut store = support::Memory::default();
    for tile in tiles {
        let reference = sealed.seal_ref(&tile.reference).unwrap();
        store.tiles.insert(
            reference.index,
            PcmTile {
                reference,
                bytes: tile.bytes,
            },
        );
    }
    let source = Arc::clone(sealed.source());
    let mut reader = PcmTileReader::new(source.clone(), PcmReadLimits::default()).unwrap();
    for (first, end) in [(1000, 1031), (0, 1), (243, 521), (17, 499), (256, 256)] {
        let result = reader
            .read(
                source.binding().unwrap(),
                PcmRange::new(first, end).unwrap(),
                &mut store,
                &|| false,
            )
            .unwrap();
        assert_eq!(result.pcm.samples(), pcm(first, end, 2).samples());
    }
    reader.clear();
    store.tiles.get_mut(&0).unwrap().bytes[0] ^= 1;
    assert!(
        reader
            .read(
                source.binding().unwrap(),
                PcmRange::new(0, 1).unwrap(),
                &mut store,
                &|| false
            )
            .is_err()
    );
}
#[test]
fn exact_tail_empty_source_and_wrong_seal_identity_are_explicit() {
    let (complete, tiles) = streamed(512, &[512], 1);
    assert!(complete.tail.is_none());
    assert_eq!(tiles.len(), 2);
    assert!(
        PcmStreamBuilder::new(origin(), 1, PcmStreamLimits::default())
            .unwrap()
            .finish_at_eof(&|| false)
            .is_err()
    );
    for kind in 0..5 {
        let mut reference = tiles[0].reference.clone();
        match kind {
            0 => reference.origin = Digest::from_sha256([3; 32]),
            1 => reference.index = 2,
            2 => reference.first_sample = Ticks::new(1),
            3 => reference.frames = 255,
            _ => reference.byte_length = ByteLength::new(1),
        }
        assert!(complete.seal_ref(&reference).is_err());
    }
}
#[test]
fn later_decode_failure_and_every_budget_cannot_complete_a_prefix() {
    for kind in 0..7 {
        let mut limits = PcmStreamLimits {
            tile_frames: 256,
            ..Default::default()
        };
        if kind == 4 {
            limits.max_tiles = 1;
        }
        if kind == 5 {
            limits.max_tiles_per_step = 1;
        }
        if kind == 6 {
            limits.max_working_bytes = 4096;
        }
        let mut builder = PcmStreamBuilder::new(origin(), 1, limits).unwrap();
        builder
            .push(&frame(0, 200, 1, false), ByteOrder::Little, &|| false)
            .unwrap();
        let mut next = frame(200, 1024, 1, false);
        match kind {
            0 => next.info.pts_ticks = Some(398), // off-grid
            1 => next.info.generation = 2,
            2 => next.info.sample_rate = Some(16000),
            3 => next.planes[0].bytes[..4].copy_from_slice(&f32::NAN.to_le_bytes()),
            _ => {}
        }
        assert!(
            builder.push(&next, ByteOrder::Little, &|| false).is_err(),
            "{kind}"
        );
        assert!(
            builder
                .push(&frame(200, 1, 1, false), ByteOrder::Little, &|| false)
                .is_err()
        );
        assert!(builder.finish_at_eof(&|| false).is_err());
    }
}
#[test]
fn cancellation_or_panic_in_any_push_stage_poison_the_whole_attempt() {
    let limits = PcmStreamLimits {
        tile_frames: 256,
        ..Default::default()
    };
    let count = Cell::new(0);
    PcmStreamBuilder::new(origin(), 1, limits)
        .unwrap()
        .push(&frame(0, 1025, 2, true), ByteOrder::Little, &|| {
            count.set(count.get() + 1);
            false
        })
        .unwrap();
    for stop in 0..count.get() {
        let mut builder = PcmStreamBuilder::new(origin(), 1, limits).unwrap();
        let calls = Cell::new(0);
        assert!(
            builder
                .push(&frame(0, 1025, 2, true), ByteOrder::Little, &|| {
                    let i = calls.get();
                    calls.set(i + 1);
                    i == stop
                })
                .is_err()
        );
        assert!(builder.finish_at_eof(&|| false).is_err());
    }
    let mut builder = PcmStreamBuilder::new(origin(), 1, limits).unwrap();
    assert!(
        std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| builder.push(
            &frame(0, 1, 1, false),
            ByteOrder::Little,
            &|| panic!("host callback")
        )))
        .is_err()
    );
    assert!(builder.finish_at_eof(&|| false).is_err());
}
