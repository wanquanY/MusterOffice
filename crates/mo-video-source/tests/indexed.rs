mod indexed_support;
mod support;
use indexed_support::*;
use mo_media::DecodeTimeline;
use mo_video_model::{
    SampleSide::{After, Before},
    TimeSample,
};
use mo_video_source::{SampleMode, SourceError, indexed::*};
use std::{cell::Cell, sync::Arc};
use support::{limits, sample};

#[test]
fn container_edits_select_exact_pictures_with_owned_bounded_cache() {
    let decoder = decoder();
    let observed = decoder.observed.clone();
    let timeline = Timeline::new();
    let drops = timeline.drops.clone();
    let mut l = limits();
    l.max_retained_frames = 2;
    l.max_retained_bytes = 2;
    let mut source = IndexedVideoSampler::new(decoder, timeline, binding(), l).unwrap();
    let mut keep = None;
    for (at, side, expected) in [
        (0, After, None),
        (20, Before, None),
        (20, After, Some(0)),
        (40, Before, Some(0)),
        (40, After, Some(30)),
        (60, After, Some(60)),
        (80, Before, Some(60)),
        (80, After, Some(0)),
        (110, After, Some(30)),
        (120, Before, Some(30)),
        (120, After, Some(60)),
        (159, After, Some(60)),
        (160, After, None),
        (180, After, None),
        (200, Before, None),
        (200, After, None),
        (20, After, Some(0)),
    ] {
        let got = source
            .sample(&binding(), &sample(at, side), SampleMode::Continue, &|| {
                false
            })
            .unwrap();
        assert_eq!(
            got.frame().map(|f| f.info.pts_ticks.unwrap()),
            expected,
            "{at} {side:?}"
        );
        assert!(got.stats.retained_bytes <= 2);
        if let Some(frame) = got.frame() {
            assert_eq!(frame.info.duration_ticks, Some(1000)); // advisory duration never changes selection
            let VideoTimeSelection::Sample { movie, .. } = got.selection() else {
                panic!()
            };
            assert!(movie.contains(
                &sample(at, side).time,
                match side {
                    After => Side::After,
                    Before => Side::Before,
                }
            ));
            keep = Some(frame.clone());
        }
    }
    assert!(!observed.seeks.borrow().is_empty());
    drop(source);
    assert_eq!(observed.drops.get(), 1);
    assert_eq!(drops.get(), 1);
    assert_eq!(keep.unwrap().planes[0].bytes, vec![0]);
}

#[test]
fn cached_pictures_survive_seek_but_each_hit_still_checks_the_timeline() {
    let decoder = decoder();
    let observed = decoder.observed.clone();
    let timeline = Timeline::new();
    let calls = timeline.calls.clone();
    let mut source = IndexedVideoSampler::new(decoder, timeline, binding(), limits()).unwrap();
    let first = source
        .sample(&binding(), &sample(20, After), SampleMode::Seek, &|| false)
        .unwrap()
        .into_frame()
        .unwrap();
    source
        .sample(&binding(), &sample(60, After), SampleMode::Seek, &|| false)
        .unwrap();
    let reads = observed.reads.get();
    let seeks = observed.seeks.borrow().len();
    let queried = calls.get();
    let repeated = source
        .sample(&binding(), &sample(80, After), SampleMode::Seek, &|| false)
        .unwrap();
    assert!(repeated.stats.cache_hit && Arc::ptr_eq(&first, repeated.frame().unwrap()));
    assert_eq!(observed.reads.get(), reads);
    assert_eq!(observed.seeks.borrow().len(), seeks);
    assert_eq!(calls.get(), queried + 1);
}

#[test]
fn one_frame_budget_handles_dwell_and_scrubbing_without_lookahead() {
    let mut l = limits();
    l.max_retained_frames = 1;
    l.max_retained_bytes = 1;
    let mut source = IndexedVideoSampler::new(decoder(), timing(), binding(), l).unwrap();
    for at in [120, 20, 60, 40, 130, 80, 159] {
        let got = source
            .sample(&binding(), &sample(at, After), SampleMode::Seek, &|| false)
            .unwrap();
        assert_eq!(got.stats.retained_bytes, 1);
    }
}

#[test]
fn trimmed_origin_and_large_forward_jump_seek_before_exhausting_decode_budget() {
    let mut raw = raw_track();
    raw.edits.clear();
    raw.samples.clear();
    raw.declared_track_duration = Some(10000);
    raw.decode_duration = 10000;
    for i in 0..10000 {
        raw.samples.push(mo_media_isobmff::Sample {
            bytes: mo_media_isobmff::ByteRange {
                offset: i,
                length: 1,
            },
            dts: i as i64,
            pts: i as i64,
            decode_duration: 1,
            description: 0,
            sync_sample: true,
        });
    }
    let timeline = VideoTiming::build(&raw, 1000, Default::default(), &|| false).unwrap();
    let mut d = decoder();
    d.pts = (0..10000).collect();
    let observed = d.observed.clone();
    let mut b = binding();
    let mo_video_model::MediaStream::Video { available, .. } = &mut b.descriptor else {
        unreachable!()
    };
    *available = support::range(0, 10000);
    let mut l = limits();
    l.max_decode_calls_per_query = 2;
    let mut source = IndexedVideoSampler::new(d, timeline, b.clone(), l).unwrap();
    for at in [5000, 9000, 5001] {
        let got = source
            .sample(&b, &sample(at, After), SampleMode::Continue, &|| false)
            .unwrap();
        assert_eq!(got.frame().unwrap().info.pts_ticks, Some(at));
        assert!(got.stats.decoded_frames <= 2);
    }
    assert_eq!(observed.reads.get(), 4);
    assert_eq!(*observed.seeks.borrow(), vec![5000, 9000, 5001]);
}

#[test]
fn identity_clock_layout_missing_frames_and_budget_fail_closed() {
    for fault in 0..4 {
        let mut d = decoder();
        match fault {
            0 => d.timeline = DecodeTimeline::Demuxer,
            1 => d.stream.container_track_id = Some(8),
            2 => d.stream.time_base.as_mut().unwrap().numerator = 2,
            _ => d.generation = 2,
        }
        assert!(IndexedVideoSampler::new(d, timing(), binding(), limits()).is_err());
    }
    for fault in 0..10 {
        let mut d = decoder();
        let observed = d.observed.clone();
        let t = Timeline::new();
        let drops = t.drops.clone();
        let mut l = limits();
        match fault {
            0 => d.pts = vec![0, 60, 90],
            1 => d.pts = vec![0],
            2 => d.pts = vec![0, 0, 30],
            3 => d.corrupt = Some("pts"),
            4 => d.corrupt = Some("generation"),
            5 => d.corrupt = Some("layout"),
            6 => d.corrupt = Some("oversize"),
            7 => d.corrupt = Some("dimensions"),
            8 => l.max_decode_calls_per_query = 1,
            _ => d.pts = vec![-1, 0, 30],
        }
        let mut source = IndexedVideoSampler::new(d, t, binding(), l).unwrap();
        if fault == 2 {
            source
                .sample(
                    &binding(),
                    &sample(20, After),
                    SampleMode::Continue,
                    &|| false,
                )
                .unwrap();
        }
        assert!(
            source
                .sample(
                    &binding(),
                    &sample(if fault == 9 { 20 } else { 40 }, After),
                    SampleMode::Continue,
                    &|| false
                )
                .is_err(),
            "{fault}"
        );
        assert!(source.is_stopped());
        assert_eq!(source.retained_bytes(), 0);
        assert_eq!(observed.drops.get(), 1);
        assert_eq!(drops.get(), 1);
        assert!(matches!(
            source.sample(
                &binding(),
                &sample(20, After),
                SampleMode::Continue,
                &|| false
            ),
            Err(IndexedSourceError::Source(SourceError::Stopped))
        ));
    }
}

#[test]
fn invalid_timeline_and_changed_resource_never_return_cached_pixels() {
    for fault in 1..=3 {
        let d = decoder();
        let observed = d.observed.clone();
        let mut t = Timeline::new();
        t.fault = fault;
        let drops = t.drops.clone();
        let mut source = IndexedVideoSampler::new(d, t, binding(), limits()).unwrap();
        assert!(
            source
                .sample(
                    &binding(),
                    &sample(20, After),
                    SampleMode::Continue,
                    &|| false
                )
                .is_err()
        );
        assert_eq!(observed.drops.get(), 1);
        assert_eq!(drops.get(), 1);
    }
    let mut source = IndexedVideoSampler::new(decoder(), timing(), binding(), limits()).unwrap();
    source
        .sample(
            &binding(),
            &sample(20, After),
            SampleMode::Continue,
            &|| false,
        )
        .unwrap();
    let mut changed = binding();
    changed.digest = mo_common::Digest::from_sha256([2; 32]);
    assert!(
        source
            .sample(&changed, &sample(20, After), SampleMode::Continue, &|| {
                false
            })
            .is_err()
    );
    assert!(source.is_stopped());
}

#[test]
fn missing_access_candidate_preserves_timing_but_cannot_claim_decodable_pixels() {
    let mut raw = raw_track();
    for sample in &mut raw.samples {
        sample.sync_sample = false;
    }
    let timing = VideoTiming::build(&raw, 1000, Default::default(), &|| false).unwrap();
    assert!(matches!(
        timing
            .select(&sample(20, After).time, Side::After, &|| false)
            .unwrap(),
        VideoTimeSelection::Sample { access: None, .. }
    ));
    let mut source = IndexedVideoSampler::new(decoder(), timing, binding(), limits()).unwrap();
    assert!(matches!(
        source.sample(&binding(), &sample(20, After), SampleMode::Seek, &|| false),
        Err(IndexedSourceError::Source(SourceError::Unsupported(_)))
    ));
    assert!(source.is_stopped());
}

#[test]
fn every_observed_cancel_point_drops_both_capabilities_and_callback_panics_fail_closed() {
    for (at, mode) in [
        (0, SampleMode::Continue),
        (20, SampleMode::Continue),
        (60, SampleMode::Seek),
        (180, SampleMode::Continue),
    ] {
        let calls = Cell::new(0);
        let mut source =
            IndexedVideoSampler::new(decoder(), timing(), binding(), limits()).unwrap();
        source
            .sample(&binding(), &sample(at, After), mode, &|| {
                calls.set(calls.get() + 1);
                false
            })
            .unwrap();
        for stop in 1..=calls.get() {
            let d = decoder();
            let observed = d.observed.clone();
            let t = Timeline::new();
            let drops = t.drops.clone();
            let count = Cell::new(0);
            let mut source = IndexedVideoSampler::new(d, t, binding(), limits()).unwrap();
            assert!(
                source
                    .sample(&binding(), &sample(at, After), mode, &|| {
                        count.set(count.get() + 1);
                        count.get() == stop
                    })
                    .is_err()
            );
            assert!(source.is_stopped());
            assert_eq!(observed.drops.get(), 1);
            assert_eq!(drops.get(), 1);
        }
    }
    let mut source = IndexedVideoSampler::new(decoder(), timing(), binding(), limits()).unwrap();
    assert!(matches!(
        source.sample(
            &binding(),
            &TimeSample {
                time: ExactRatio::integer(0),
                side: After
            },
            SampleMode::Continue,
            &|| panic!("cancel")
        ),
        Err(IndexedSourceError::Source(SourceError::CallbackPanic))
    ));
}
