#[path = "audio_movie/support.rs"]
mod support;
use mo_video_runtime::{
    audio::*,
    audio_movie::*,
    isobmff::{movie::*, presentation::*},
};
use support::*;
#[test]
fn priming_padding_empty_and_repeated_edits_are_exact_samples() {
    let samples: Vec<f32> = (0..1024).map(|i| i as f32).collect();
    let (mut source, binding, asset) = resource(
        samples.clone(),
        vec![
            edit(0, 10, None, 65536),
            edit(10, 200, Some(32), 65536),
            edit(210, 100, Some(32), 65536),
            edit(310, 17, None, 65536),
            edit(327, 53, Some(600), 65536),
        ],
        8000,
        MovieAudioLimits::default(),
    );
    let expected: Vec<f32> = [
        vec![0.; 10],
        samples[32..232].to_vec(),
        samples[32..132].to_vec(),
        vec![0.; 17],
        samples[600..653].to_vec(),
    ]
    .concat();
    assert_eq!(read(&mut source, &binding, &asset, 0, 380), expected);
    let mut split = vec![];
    for first in (0..380).step_by(13) {
        split.extend(read(
            &mut source,
            &binding,
            &asset,
            first,
            (first + 13).min(380),
        ));
    }
    assert_eq!(split, expected);
    for range in &source.raw_mut().requests {
        assert!(range.first >= 32 && range.end <= 653);
    }
}
#[test]
fn fractional_edit_boundaries_use_absolute_ceil_without_drift() {
    let samples = vec![0.5; 2000];
    let (mut source, binding, asset) = resource(
        samples,
        vec![
            edit(0, 1, None, 65536),
            edit(1, 1, Some(100), 65536),
            edit(2, 1, None, 65536),
            edit(3, 1, Some(200), 65536),
        ],
        30000,
        MovieAudioLimits::default(),
    );
    // [0, 4/30000) contains output samples 0 and 1 only. Sample 1 lies
    // in the fourth edit; no per-edit rounding can invent two extra samples.
    assert_eq!(asset.available.end, 2);
    let out = read(&mut source, &binding, &asset, 0, 2);
    assert_eq!(out[0], 0.);
    assert!(out[1] > 0. && out[1] < 0.6);
    assert_eq!(source.raw_mut().requests.len(), 1);
    assert_eq!(source.raw_mut().requests[0].first, 200);
}
#[test]
fn varispeed_repeated_mapping_is_partition_and_seek_invariant() {
    let samples: Vec<f32> = (0..16000)
        .map(|i| ((i as f64 * 0.07).sin() * 0.4) as f32)
        .collect();
    let (mut source, binding, asset) = resource(
        samples,
        vec![
            edit(0, 5000, Some(1000), 98304),
            edit(5000, 1000, None, 65536),
            edit(6000, 4000, Some(2000), 32768),
        ],
        8000,
        MovieAudioLimits::default(),
    );
    let mut baseline = read(&mut source, &binding, &asset, 0, 8000);
    baseline.extend(read(&mut source, &binding, &asset, 8000, 10000));
    for first in (0..10000i32).step_by(137).rev() {
        let first = i64::from(first);
        let end = (first + 137).min(10000);
        assert_eq!(
            read(&mut source, &binding, &asset, first, end),
            baseline[first as usize..end as usize]
        );
        assert!(source.retained_filter_bytes() <= 32 << 20);
    }
    assert!(baseline[5000..6000].iter().all(|v| *v == 0.));
}
#[test]
fn missing_media_never_becomes_silence_and_limits_cancel_before_success() {
    let (mut source, binding, asset) = resource(
        vec![0.5; 4000],
        vec![edit(0, 1000, Some(0), 65536)],
        8000,
        MovieAudioLimits::default(),
    );
    source.raw_mut().broken = true;
    assert!(
        source
            .read(
                &binding,
                &asset,
                PcmRange::new(0, 100).unwrap(),
                PcmLimits::default(),
                &|| false
            )
            .is_err()
    );
    source.raw_mut().broken = false;
    assert!(
        source
            .read(
                &binding,
                &asset,
                PcmRange::new(0, 100).unwrap(),
                PcmLimits::default(),
                &|| true
            )
            .is_err()
    );
    assert!(
        source
            .read(
                &binding,
                &asset,
                PcmRange::new(0, 100).unwrap(),
                PcmLimits {
                    max_frames: 99,
                    max_bytes: 10000
                },
                &|| false
            )
            .is_err()
    );
    assert_eq!(read(&mut source, &binding, &asset, 0, 100), vec![0.5; 100]);
    let (mut tiny, binding, asset) = resource(
        vec![0.5; 4000],
        vec![edit(0, 1000, Some(0), 65536)],
        8000,
        MovieAudioLimits {
            max_working_bytes: 100,
            ..Default::default()
        },
    );
    assert!(
        tiny.read(
            &binding,
            &asset,
            PcmRange::new(0, 100).unwrap(),
            PcmLimits::default(),
            &|| false
        )
        .is_err()
    );
    assert!(tiny.raw_mut().requests.is_empty());
}
#[test]
fn unsupported_dwell_and_unproven_media_coverage_are_explicit_errors() {
    let decoded = inspect(&vec![0.1; 1000], 8000);
    let raw = binding(&decoded);
    for edit in [
        edit(0, 100, Some(0), 0),
        edit(0, 1001, Some(0), 65536),
        edit(0, 100, Some(999), 65536),
    ] {
        let timeline = MovieTimeline::build(
            &track(vec![edit], 8000),
            8000,
            &r(1, 8),
            TimingLimits::default(),
            &|| false,
        )
        .unwrap();
        assert!(
            MovieAudioPlan::new(
                &timeline,
                raw.clone(),
                &decoded,
                MovieAudioLimits::default(),
                &|| false
            )
            .is_err()
        );
    }
}

#[test]
fn changed_capability_and_cancellation_at_each_read_stage_return_no_audio() {
    use std::cell::Cell;
    let (mut source, binding, asset) = resource(
        vec![0.25; 4000],
        vec![edit(0, 1000, Some(0), 65536)],
        8000,
        MovieAudioLimits::default(),
    );
    read(&mut source, &binding, &asset, 0, 100); // Hold one already-admitted filter.
    let calls = Cell::new(0);
    let range = PcmRange::new(30, 77).unwrap();
    let expected = source
        .read(&binding, &asset, range, PcmLimits::default(), &|| {
            calls.set(calls.get() + 1);
            false
        })
        .unwrap();
    for stop in 1..=calls.get() {
        let at = Cell::new(0);
        assert!(
            source
                .read(&binding, &asset, range, PcmLimits::default(), &|| {
                    at.set(at.get() + 1);
                    at.get() == stop
                })
                .is_err()
        );
    }
    assert_eq!(
        read(&mut source, &binding, &asset, 30, 77),
        expected.samples()
    );
    source.raw_mut().asset.sample_rate = 16000;
    assert!(
        source
            .read(&binding, &asset, range, PcmLimits::default(), &|| false)
            .is_err()
    );
}
