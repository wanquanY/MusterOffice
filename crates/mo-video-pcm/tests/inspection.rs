use mo_common::RationalTime;
use mo_media::{DecodedFrame, FramePlane, TimeBase};
use mo_time::ExactRatio as R;
use mo_video_audio::{ByteOrder, PcmLimits};
use mo_video_model::TimeRange;
use mo_video_pcm::*;

fn frame(pts: i64, samples: u32) -> DecodedFrame {
    DecodedFrame {
        info: serde_json::from_value(serde_json::json!({
            "generation":1,"kind":"audio","timeBase":{"numerator":1,"denominator":16000},
            "ptsTicks":pts,"durationTicks":999999,"width":null,"height":null,
            "sampleRate":8000,"channels":1,"samples":samples,"format":"flt","channelLayout":"mono",
            "colors":{"primaries":2,"transfer":2,"matrix":2,"range":0,"chromaLocation":0},
            "sampleAspectRatio":null,"alphaMode":0,"keyFrame":true,"interlaced":false,"topFieldFirst":false
        })).unwrap(),
        planes: vec![FramePlane {
            row_bytes: samples as usize * 4,
            rows: 1,
            bytes: (0..samples).flat_map(|i| (i as f32 / 32768.).to_le_bytes()).collect(),
        }],
    }
}
fn scan() -> PcmInspection {
    PcmInspection::new(1, PcmInspectLimits::default()).unwrap()
}
fn t(n: i64, d: u32) -> RationalTime {
    RationalTime::new(n, d).unwrap()
}
#[test]
fn real_sample_counts_and_negative_fractional_origin_define_exact_coverage() {
    let mut scan = scan();
    scan.observe(&frame(-3, 17), ByteOrder::Little, &|| false)
        .unwrap();
    // Same clock expressed in different units, without duration accumulation.
    let mut next = frame(62, 103);
    next.info.time_base = TimeBase {
        numerator: 1,
        denominator: 32000,
    };
    next.info.format = "fltp".into();
    scan.observe(&next, ByteOrder::Little, &|| false).unwrap();
    let result = scan.finish_at_eof(&|| false).unwrap();
    assert_eq!(result.decode_frames(), 2);
    assert_eq!(result.asset().available.end, 120);
    assert!(
        result
            .asset()
            .sample_zero
            .compare(&R::from_time(t(-3, 16000)))
            .is_eq()
    );
    assert_eq!(
        result.decoded_range(256).unwrap(),
        TimeRange {
            start: t(-3, 16000),
            end: t(237, 16000)
        }
    );
    let subset = result
        .continuous_asset(
            TimeRange {
                start: t(0, 1),
                end: t(7, 1000),
            },
            256,
        )
        .unwrap();
    assert_eq!((subset.available.first, subset.available.end), (2, 58));
    for range in [
        TimeRange {
            start: t(-4, 16000),
            end: t(0, 1),
        },
        TimeRange {
            start: t(0, 1),
            end: t(238, 16000),
        },
    ] {
        assert!(result.continuous_asset(range, 256).is_err());
    }
}
#[test]
fn gap_overlap_off_grid_missing_pts_and_seek_cannot_finish_a_prefix() {
    for kind in 0..8 {
        let mut scan = scan();
        scan.observe(&frame(0, 16), ByteOrder::Little, &|| false)
            .unwrap();
        let mut next = frame(32, 16);
        match kind {
            0 => next.info.pts_ticks = Some(34),
            1 => next.info.pts_ticks = Some(30),
            2 => next.info.pts_ticks = Some(33),
            3 => next.info.pts_ticks = None,
            4 => next.info.generation = 2,
            5 => next.info.sample_rate = Some(16000),
            6 => next.info.channel_layout = "stereo".into(),
            _ => next.info.time_base.denominator = 0,
        }
        assert!(
            scan.observe(&next, ByteOrder::Little, &|| false).is_err(),
            "{kind}"
        );
        assert!(
            scan.observe(&frame(32, 16), ByteOrder::Little, &|| false)
                .is_err()
        );
        assert!(scan.finish_at_eof(&|| false).is_err());
    }
}
#[test]
fn corrupt_pcm_and_resource_budgets_are_checked_before_admission() {
    for kind in 0..6 {
        let mut limits = PcmInspectLimits::default();
        let mut input = frame(0, 16);
        match kind {
            0 => input.planes[0].bytes[0..4].copy_from_slice(&f32::NAN.to_le_bytes()),
            1 => input.planes[0].bytes.pop().map(|_| ()).unwrap(),
            2 => limits.max_sample_frames = 15,
            3 => {
                limits.frame = PcmLimits {
                    max_frames: 15,
                    max_bytes: 1024,
                }
            }
            4 => {
                limits.frame = PcmLimits {
                    max_frames: 100,
                    max_bytes: 63,
                }
            }
            _ => input.info.samples = Some(0),
        }
        let mut scan = PcmInspection::new(1, limits).unwrap();
        assert!(scan.observe(&input, ByteOrder::Little, &|| false).is_err());
        assert!(scan.finish_at_eof(&|| false).is_err());
    }
    let mut scan = PcmInspection::new(
        1,
        PcmInspectLimits {
            max_decode_frames: 1,
            ..Default::default()
        },
    )
    .unwrap();
    scan.observe(&frame(0, 1), ByteOrder::Little, &|| false)
        .unwrap();
    assert!(
        scan.observe(&frame(2, 1), ByteOrder::Little, &|| false)
            .is_err()
    );
    assert!(scan.finish_at_eof(&|| false).is_err());
}
#[test]
fn empty_cancelled_and_panicked_scans_never_return_partial_success() {
    assert!(scan().finish_at_eof(&|| false).is_err());
    let mut cancelled = scan();
    assert!(
        cancelled
            .observe(&frame(0, 1), ByteOrder::Little, &|| true)
            .is_err()
    );
    assert!(cancelled.finish_at_eof(&|| false).is_err());
    let mut panicked = scan();
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let _ = panicked.observe(&frame(0, 1), ByteOrder::Little, &|| panic!("host callback"));
    }));
    assert!(result.is_err());
    assert!(panicked.finish_at_eof(&|| false).is_err());
}
#[test]
fn huge_pts_stay_exact_and_sample_partition_does_not_change_asset() {
    let origin = 9_007_199_254_740_993;
    let mut whole = scan();
    whole
        .observe(&frame(origin, 100), ByteOrder::Little, &|| false)
        .unwrap();
    let mut chunks = scan();
    for i in 0..100 {
        chunks
            .observe(&frame(origin + i * 2, 1), ByteOrder::Little, &|| false)
            .unwrap();
    }
    let a = whole.finish_at_eof(&|| false).unwrap();
    let b = chunks.finish_at_eof(&|| false).unwrap();
    assert_eq!(a.asset(), b.asset());
    assert_eq!(a.decoded_range(256).unwrap().start, t(origin, 16000));
}

#[test]
fn inspected_blocks_transfer_owned_pcm_on_the_same_exact_lattice() {
    let mut scan = scan();
    let mut input = frame(-3, 17);
    let first = scan
        .observe_pcm(&input, ByteOrder::Little, &|| false)
        .unwrap();
    input.planes[0].bytes.fill(0);
    assert_eq!(first.first_frame(), 0);
    assert_eq!(first.end_frame(), 17);
    assert_eq!(first.samples()[16].to_bits(), (16.0f32 / 32768.).to_bits());
    let mut input = frame(31, 13);
    // Changing packed/planar labels and host byte order must not change the
    // canonical values or create a second timestamp conversion in the host.
    input.info.format = "fltp".into();
    for bytes in input.planes[0].bytes.chunks_exact_mut(4) {
        bytes.reverse();
    }
    let next = scan.observe_pcm(&input, ByteOrder::Big, &|| false).unwrap();
    assert_eq!(next.first_frame(), 17);
    assert_eq!(next.end_frame(), 30);
    assert_eq!(next.samples()[12].to_bits(), (12.0f32 / 32768.).to_bits());
    let result = scan.finish_at_eof(&|| false).unwrap();
    assert_eq!(result.asset().available.end, next.end_frame());
    assert_eq!(result.decoded_range(256).unwrap().end, t(57, 16000));
}

#[test]
fn provisional_pcm_is_not_a_complete_asset_after_later_failure() {
    let mut scan = scan();
    let block = scan
        .observe_pcm(&frame(0, 16), ByteOrder::Little, &|| false)
        .unwrap();
    assert_eq!(block.frames(), 16);
    let mut broken = frame(32, 16);
    broken.planes[0].bytes[0..4].copy_from_slice(&f32::INFINITY.to_le_bytes());
    assert!(
        scan.observe_pcm(&broken, ByteOrder::Little, &|| false)
            .is_err()
    );
    assert!(
        scan.observe_pcm(&frame(32, 16), ByteOrder::Little, &|| false)
            .is_err()
    );
    assert!(scan.finish_at_eof(&|| false).is_err());
}
