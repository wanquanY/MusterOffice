mod support;
use mo_common::Digest;
use mo_video_model::SampleSide::{After, Before};
use mo_video_source::*;
use std::cell::Cell;
use support::*;

#[test]
fn vfr_before_after_reverse_and_overlapping_decoder_durations_use_exact_pts() {
    let decoder = Fake::new(vec![-50, -10, 23, 100]);
    let observed = decoder.observed.clone();
    let mut s = VideoSampler::new(decoder, binding(), range(-50, 160), limits()).unwrap();
    for (t, side, expected) in [
        (-50, After, -50),
        (-10, Before, -50),
        (-10, After, -10),
        (23, Before, -10),
        (23, After, 23),
        (90, After, 23),
        (160, Before, 100),
        (23, Before, -10),
        (-50, After, -50),
    ] {
        let frame = selected(
            s.sample(&binding(), &sample(t, side), SampleMode::Continue, &|| {
                false
            })
            .unwrap(),
        );
        assert_eq!(frame.frame.info.pts_ticks, Some(expected));
        assert_eq!(frame.frame.info.duration_ticks, Some(1000)); // raw advisory metadata preserved
        assert!(s.retained_bytes() <= 5);
    }
    assert!(matches!(
        s.sample(
            &binding(),
            &sample(160, After),
            SampleMode::Continue,
            &|| false
        )
        .unwrap()
        .selection,
        SourceSelection::OutsideSpan
    ));
    let count = observed.reads.get();
    let result = s
        .sample(&binding(), &sample(-50, After), SampleMode::Seek, &|| false)
        .unwrap();
    assert!(result.stats.cache_hit);
    assert_eq!(observed.reads.get(), count);
}
#[test]
fn backwards_gop_queries_remain_bounded_and_return_owned_frames_after_close() {
    let mut l = limits();
    l.max_retained_frames = 3;
    l.max_retained_bytes = 3;
    let decoder = Fake::new((-50..160).step_by(5).collect());
    let observed = decoder.observed.clone();
    let mut s = VideoSampler::new(decoder, binding(), range(-50, 160), l).unwrap();
    let retained = selected(
        s.sample(
            &binding(),
            &sample(-50, After),
            SampleMode::Continue,
            &|| false,
        )
        .unwrap(),
    );
    for t in (-50i32..160)
        .step_by(5)
        .chain((-50i32..160).step_by(5).rev())
        .map(i64::from)
    {
        let f = selected(
            s.sample(&binding(), &sample(t, After), SampleMode::Continue, &|| {
                false
            })
            .unwrap(),
        );
        assert_eq!(f.frame.info.pts_ticks, Some(t));
        assert!(s.retained_bytes() <= 3);
    }
    drop(s);
    assert_eq!(observed.drops.get(), 1);
    assert_eq!(retained.frame.planes[0].bytes, vec![(-50i64) as u8]);
    assert!(observed.seeks.borrow().len() > 1);
}
#[test]
fn random_seek_before_exact_boundary_uses_preceding_tick_and_rewinds_overshoot() {
    let mut decoder = Fake::new(vec![-50, -10, 23, 100]);
    decoder.overshoot = true;
    let observed = decoder.observed.clone();
    let mut s = VideoSampler::new(decoder, binding(), range(-50, 160), limits()).unwrap();
    let result = s
        .sample(&binding(), &sample(23, Before), SampleMode::Seek, &|| false)
        .unwrap();
    assert_eq!(result.stats.seeks, 2);
    assert_eq!(selected(result).frame.info.pts_ticks, Some(-10));
    assert_eq!(*observed.seeks.borrow(), vec![22, -50]);
}
#[test]
fn empty_and_leading_spans_are_explicit_gaps_not_nearest_frame_substitutions() {
    for pts in [vec![], vec![0, 100], vec![200]] {
        let mut s =
            VideoSampler::new(Fake::new(pts), binding(), range(-50, 160), limits()).unwrap();
        assert!(matches!(
            s.sample(
                &binding(),
                &sample(-25, After),
                SampleMode::Continue,
                &|| false
            )
            .unwrap()
            .selection,
            SourceSelection::Gap(_)
        ));
    }
}
#[test]
fn invalid_frame_metadata_and_duplicate_pts_stop_and_drop_decoder() {
    for corruption in [
        "pts",
        "generation",
        "layout",
        "oversize",
        "dimensions",
        "duplicate",
        "unordered",
    ] {
        let mut decoder = Fake::new(match corruption {
            "duplicate" => vec![-50, -50],
            "unordered" => vec![23, -10],
            _ => vec![-50, 0],
        });
        decoder.corrupt = Some(corruption);
        let observed = decoder.observed.clone();
        let mut s = VideoSampler::new(decoder, binding(), range(-50, 160), limits()).unwrap();
        assert!(
            s.sample(&binding(), &sample(0, After), SampleMode::Continue, &|| {
                false
            })
            .is_err(),
            "{corruption}"
        );
        assert!(s.is_stopped());
        assert_eq!(s.retained_bytes(), 0);
        assert_eq!(observed.drops.get(), 1);
    }
}
#[test]
fn per_query_work_budget_does_not_return_an_old_picture() {
    let mut l = limits();
    l.max_decode_calls_per_query = 2;
    let decoder = Fake::new(vec![-50, -10, 23, 100]);
    let observed = decoder.observed.clone();
    let mut s = VideoSampler::new(decoder, binding(), range(-50, 160), l).unwrap();
    assert!(matches!(
        s.sample(
            &binding(),
            &sample(100, After),
            SampleMode::Continue,
            &|| false
        ),
        Err(SourceError::Budget)
    ));
    assert_eq!(observed.reads.get(), 2);
    assert_eq!(observed.drops.get(), 1);
}
#[test]
fn cancellation_at_every_observed_boundary_discards_candidate_and_releases_decoder() {
    let count = Cell::new(0);
    let mut s = VideoSampler::new(
        Fake::new(vec![-50, -10, 23, 100]),
        binding(),
        range(-50, 160),
        limits(),
    )
    .unwrap();
    s.sample(
        &binding(),
        &sample(100, After),
        SampleMode::Continue,
        &|| {
            count.set(count.get() + 1);
            false
        },
    )
    .unwrap();
    for cancel_at in 1..=count.get() {
        let decoder = Fake::new(vec![-50, -10, 23, 100]);
        let observed = decoder.observed.clone();
        let seen = Cell::new(0);
        let mut s = VideoSampler::new(decoder, binding(), range(-50, 160), limits()).unwrap();
        let result = s.sample(
            &binding(),
            &sample(100, After),
            SampleMode::Continue,
            &|| {
                seen.set(seen.get() + 1);
                seen.get() == cancel_at
            },
        );
        assert!(
            matches!(
                result,
                Err(SourceError::Cancelled)
                    | Err(SourceError::Compile(
                        mo_video_compile::CompileError::Cancelled
                    ))
            ),
            "checkpoint {cancel_at}"
        );
        assert_eq!(observed.drops.get(), 1);
        assert!(s.is_stopped());
    }
}
#[test]
fn stale_bindings_and_callback_panics_cannot_keep_live_decoders() {
    let mut changed = binding();
    changed.digest = Digest::from_sha256([2; 32]);
    let mut s = VideoSampler::new(
        Fake::new(vec![-50, 0]),
        binding(),
        range(-50, 160),
        limits(),
    )
    .unwrap();
    assert!(matches!(
        s.sample(&changed, &sample(0, After), SampleMode::Continue, &|| false),
        Err(SourceError::Invalid(_))
    ));
    assert!(s.is_stopped());
    let decoder = Fake::new(vec![-50, 0]);
    let observed = decoder.observed.clone();
    let mut s = VideoSampler::new(decoder, binding(), range(-50, 160), limits()).unwrap();
    assert!(matches!(
        s.sample(
            &binding(),
            &sample(0, After),
            SampleMode::Continue,
            &|| panic!("callback")
        ),
        Err(SourceError::CallbackPanic)
    ));
    assert_eq!(observed.drops.get(), 1);
}
