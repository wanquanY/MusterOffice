mod support;
use mo_media_isobmff::{presentation::*, *};
use mo_time::ExactRatio;
use std::cell::Cell;
fn time(n: i64, d: u32) -> ExactRatio {
    ExactRatio::new(n.into(), d.into(), 256).unwrap()
}
fn track() -> TrackIndex {
    let data = support::signed_fixture().file();
    IsoBmffIndex::read(
        &mut |offset, out: &mut [u8]| {
            let start = offset as usize;
            let n = out.len().min(data.len() - start);
            out[..n].copy_from_slice(&data[start..start + n]);
            Ok(n)
        },
        data.len() as u64,
        IndexLimits::default(),
        &|| false,
    )
    .unwrap()
    .tracks
    .remove(0)
}
fn selected(t: &VideoTiming, n: i64, d: u32, side: Side) -> Option<u32> {
    match t.select(&time(n, d), side, &|| false).unwrap() {
        VideoTimeSelection::Sample { sample, movie, .. } => {
            assert!(movie.contains(&time(n, d), side));
            Some(sample)
        }
        _ => None,
    }
}
#[test]
fn edits_empty_repeated_dwell_and_b_frame_boundaries_are_exact() {
    let t = VideoTiming::build(&track(), 1000, TimingLimits::default(), &|| false).unwrap();
    assert_eq!(t.summary().raw_span, Some((0, 40)));
    for (n, after, before) in [
        (0, None, None),
        (1, Some(0), None),
        (2, Some(2), Some(0)),
        (3, Some(3), Some(2)),
        (4, Some(1), Some(3)),
        (5, Some(3), Some(1)),
        (6, Some(3), Some(3)),
        (7, None, Some(3)),
    ] {
        assert_eq!(selected(&t, n, 10, Side::After), after);
        assert_eq!(selected(&t, n, 10, Side::Before), before);
    }
    let mut raw = track();
    raw.edits = vec![
        Edit {
            movie_start: 0,
            movie_duration: 100,
            media_time: Some(10),
            rate_16_16: 65536,
        },
        Edit {
            movie_start: 100,
            movie_duration: 200,
            media_time: Some(0),
            rate_16_16: 65536,
        },
    ];
    let t = VideoTiming::build(&raw, 1000, TimingLimits::default(), &|| false).unwrap();
    assert_eq!(selected(&t, 0, 1, Side::After), Some(2));
    assert_eq!(selected(&t, 1, 10, Side::Before), Some(2));
    assert_eq!(selected(&t, 1, 10, Side::After), Some(0));
    assert_eq!(selected(&t, 3, 10, Side::Before), Some(2));
}
#[test]
fn vfr_uses_next_composition_time_and_only_final_display_sample_decode_duration() {
    let mut raw = track();
    raw.edits.clear();
    raw.declared_track_duration = None;
    raw.samples[0].pts = 20;
    raw.samples[1].pts = 80;
    raw.samples[2].pts = 40;
    raw.samples[3].pts = 100;
    // Decoding durations deliberately do not match reordered display spacing.
    let t = VideoTiming::build(&raw, 1000, TimingLimits::default(), &|| false).unwrap();
    assert_eq!(t.summary().raw_span, Some((20, 110)));
    assert!(t.summary().movie_end.compare(&time(11, 10)).is_eq());
    for (n, want) in [
        (0, None),
        (19, None),
        (20, Some(0)),
        (39, Some(0)),
        (40, Some(2)),
        (79, Some(2)),
        (80, Some(1)),
        (100, Some(3)),
        (109, Some(3)),
        (110, None),
    ] {
        assert_eq!(selected(&t, n, 100, Side::After), want);
    }
    raw.declared_track_duration = Some(1500);
    let t = VideoTiming::build(&raw, 1000, TimingLimits::default(), &|| false).unwrap();
    let VideoTimeSelection::Gap { movie, .. } =
        t.select(&time(12, 10), Side::After, &|| false).unwrap()
    else {
        panic!("suffix gap")
    };
    assert!(movie.start.compare(&time(11, 10)).is_eq());
    assert!(movie.end.compare(&time(15, 10)).is_eq());
}
#[test]
fn fractional_edits_preserve_subtick_boundaries_and_dwell_gaps() {
    let mut raw = track();
    raw.edits = vec![Edit {
        movie_start: 0,
        movie_duration: 2,
        media_time: Some(0),
        rate_16_16: 98304,
    }];
    let t = VideoTiming::build(&raw, 3, TimingLimits::default(), &|| false).unwrap();
    let q = time(1, 15);
    let VideoTimeSelection::Sample {
        sample,
        movie,
        media_time,
        ..
    } = t.select(&q, Side::After, &|| false).unwrap()
    else {
        panic!()
    };
    assert_eq!(sample, 2);
    assert!(movie.start.compare(&time(1, 15)).is_eq());
    assert!(media_time.compare(&time(1, 10)).is_eq());
    assert_eq!(selected(&t, 1, 15, Side::Before), Some(0));
    assert!(matches!(
        t.select(&time(1, 3), Side::After, &|| false).unwrap(),
        VideoTimeSelection::Gap { .. }
    ));
    raw.edits[0].rate_16_16 = 0;
    raw.edits[0].media_time = Some(99);
    let t = VideoTiming::build(&raw, 3, TimingLimits::default(), &|| false).unwrap();
    assert!(matches!(
        t.select(&time(1, 3), Side::After, &|| false).unwrap(),
        VideoTimeSelection::Gap { .. }
    ));
}
#[test]
fn generated_permutations_rates_and_partial_edits_match_integer_oracle() {
    for seed in 1..129u32 {
        let mut raw = track();
        raw.samples.clear();
        raw.timescale = 97;
        let n = 65u32;
        let step = 17u32;
        for i in 0..n {
            raw.samples.push(Sample {
                bytes: ByteRange {
                    offset: i as u64,
                    length: 1,
                },
                dts: i64::from(i * 3),
                pts: i64::from(((i * step + seed) % n) * 7) - 14,
                decode_duration: 3,
                description: 0,
                sync_sample: true,
            });
        }
        raw.decode_duration = u64::from(n * 3);
        let rate = 16384 * (1 + seed % 9);
        let media = (seed % 40) as i64;
        raw.edits = vec![Edit {
            movie_start: 0,
            movie_duration: 3,
            media_time: Some(media),
            rate_16_16: rate as i32,
        }];
        let t = VideoTiming::build(&raw, 1, TimingLimits::default(), &|| false).unwrap();
        let mut frames: Vec<_> = raw.samples.iter().enumerate().collect();
        frames.sort_by_key(|(_, s)| s.pts);
        for tick in 0..151i64 {
            // Independent integer cross-products for movie time tick/50 and media ticks.
            let num = i128::from(media) * 50 * 65536 + i128::from(tick) * 97 * i128::from(rate);
            let den = 50 * 65536i128;
            for side in [Side::After, Side::Before] {
                let inside = match side {
                    Side::After => tick < 150,
                    Side::Before => tick > 0,
                };
                let want = if !inside {
                    None
                } else {
                    frames.iter().enumerate().find_map(|(j, (index, s))| {
                        let end = frames.get(j + 1).map_or(s.pts + 3, |(_, v)| v.pts);
                        let contains = match side {
                            Side::After => {
                                i128::from(s.pts) * den <= num && num < i128::from(end) * den
                            }
                            Side::Before => {
                                i128::from(s.pts) * den < num && num <= i128::from(end) * den
                            }
                        };
                        contains.then_some(*index as u32)
                    })
                };
                assert_eq!(
                    selected(&t, tick, 50, side),
                    want,
                    "seed {seed} tick {tick} {side:?}"
                );
            }
        }
    }
}
#[test]
fn malformed_and_cancelled_timing_never_yields_a_partial_index() {
    let raw = track();
    let count = Cell::new(0);
    VideoTiming::build(&raw, 1000, TimingLimits::default(), &|| {
        count.set(count.get() + 1);
        false
    })
    .unwrap();
    for stop in 1..=count.get() {
        let calls = Cell::new(0);
        assert!(matches!(
            VideoTiming::build(&raw, 1000, TimingLimits::default(), &|| {
                calls.set(calls.get() + 1);
                calls.get() == stop
            }),
            Err(TimingError::Cancelled)
        ));
    }
    assert!(matches!(
        VideoTiming::build(&raw, 1000, TimingLimits::default(), &|| panic!(
            "cancel callback"
        )),
        Err(TimingError::CallbackPanic)
    ));
    for kind in 0..6 {
        let mut raw = track();
        match kind {
            0 => raw.samples[1].pts = raw.samples[0].pts,
            1 => raw.samples[0].dts = 1,
            2 => raw.samples[0].decode_duration = 0,
            3 => raw.edits[0].rate_16_16 = -1,
            4 => raw.edits[1].movie_start = 999,
            _ => raw.handler = *b"soun",
        }
        assert!(VideoTiming::build(&raw, 1000, TimingLimits::default(), &|| false).is_err());
    }
    assert!(matches!(
        VideoTiming::build(
            &raw,
            1000,
            TimingLimits {
                max_samples: 1,
                ..TimingLimits::default()
            },
            &|| false
        ),
        Err(TimingError::Budget)
    ));
}

#[test]
fn every_selection_path_observes_cancellation_and_arithmetic_limits() {
    let t = VideoTiming::build(&track(), 1000, TimingLimits::default(), &|| false).unwrap();
    for at in [time(-1, 1), time(0, 1), time(2, 10), time(6, 10)] {
        let calls = Cell::new(0);
        t.select(&at, Side::After, &|| {
            calls.set(calls.get() + 1);
            false
        })
        .unwrap();
        for stop in 1..=calls.get() {
            let observed = Cell::new(0);
            assert!(matches!(
                t.select(&at, Side::After, &|| {
                    observed.set(observed.get() + 1);
                    observed.get() == stop
                }),
                Err(TimingError::Cancelled)
            ));
        }
    }
    assert!(matches!(
        t.select(&time(1, 10), Side::After, &|| panic!("query callback")),
        Err(TimingError::CallbackPanic)
    ));
    let mut too_large = ExactRatio::integer(i64::MAX);
    for _ in 0..4 {
        too_large = too_large.mul(&ExactRatio::integer(i64::MAX), 1024).unwrap();
    }
    assert!(matches!(
        t.select(&too_large, Side::After, &|| false),
        Err(TimingError::Time(_))
    ));
}
