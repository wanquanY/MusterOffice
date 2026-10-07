mod support;
use mo_video_runtime::{
    authoring::model::*, render::*, source::SampleMode, time::ExactRatio, visual_source::*,
};
use std::{cell::Cell, rc::Rc, sync::Arc};
use support::*;
fn image(value: ResolvedMedia) -> Arc<LinearImage> {
    let ResolvedVisual::VideoFrame { frame, .. } = value.visual else {
        panic!("frame")
    };
    frame.image
}
fn output() -> OutputOptions {
    OutputOptions {
        tone_map: None,
        alpha: OutputAlpha::Straight,
        max_bytes: 1024,
    }
}
#[test]
fn gaps_exact_boundaries_repeats_dwell_and_alpha_reach_composition_without_rounding() {
    let (mut source, observed, _) = source();
    let gap = source
        .media(&binding(), &sample(0, 1, SampleSide::After), &|| false)
        .unwrap();
    assert!(matches!(gap.visual, ResolvedVisual::VideoGap { .. }));
    assert_eq!(observed.reads.get(), 0);
    let first = image(
        source
            .media(&binding(), &sample(1, 1, SampleSide::After), &|| false)
            .unwrap(),
    );
    assert!(source.last_stats().normalized);
    assert_eq!(
        &first.to_srgb8(output(), &|| false).unwrap().rgba[..4],
        &[255, 0, 0, 255]
    );
    // This boundary is 1 + (500001/1000003)/(98305/65536); no u32 timescale fits.
    let boundary = ExactRatio::integer(1)
        .add(
            &ExactRatio::new(
                (500_001u64 * 65536).into(),
                (1_000_003u64 * 98305).into(),
                256,
            )
            .unwrap(),
            256,
        )
        .unwrap();
    assert!(boundary.to_time_exact().is_err());
    let before = TimeSample {
        time: boundary.clone(),
        side: SampleSide::Before,
    };
    let after = TimeSample {
        time: boundary,
        side: SampleSide::After,
    };
    let same = image(source.media(&binding(), &before, &|| false).unwrap());
    assert!(Arc::ptr_eq(&first, &same));
    let green = image(source.media(&binding(), &after, &|| false).unwrap());
    assert_eq!(
        &green.to_srgb8(output(), &|| false).unwrap().rgba[..4],
        &[0, 255, 0, 128]
    );
    for time in [
        sample(2, 1, SampleSide::After),
        sample(29, 10, SampleSide::After),
    ] {
        let held = image(source.media(&binding(), &time, &|| false).unwrap());
        assert!(Arc::ptr_eq(&held, &green));
        assert!(source.last_stats().normalized_cache_hit);
    }
    let repeat = image(
        source
            .media(&binding(), &sample(3, 1, SampleSide::After), &|| false)
            .unwrap(),
    );
    assert!(Arc::ptr_eq(&repeat, &first));
    assert_eq!(source.retained_bytes(), 128);
    assert_eq!(observed.timeline_calls.get(), 7); // Every call, including all normalized-cache hits.
    let doc = document(&binding(), 3, false);
    let preview = render_frame(
        &doc.frame(48, Default::default(), &|| false).unwrap().value,
        &mut source,
        Sampling::Nearest,
        Default::default(),
        &|| false,
    )
    .unwrap();
    let rgba = preview.image.to_srgb8(output(), &|| false).unwrap().rgba;
    assert_eq!(&rgba[..4], &[0, 188, 187, 255]); // half-transparent green over blue, in linear light
}
#[test]
fn normalized_lru_is_bounded_and_caller_owned_images_survive_eviction() {
    let (mut source, _, _) = source();
    let red = image(
        source
            .media(&binding(), &sample(1, 1, SampleSide::After), &|| false)
            .unwrap(),
    );
    image(
        source
            .media(&binding(), &sample(3, 2, SampleSide::After), &|| false)
            .unwrap(),
    );
    image(
        source
            .media(&binding(), &sample(9, 5, SampleSide::After), &|| false)
            .unwrap(),
    );
    assert_eq!(source.retained_bytes(), 128);
    let red_again = image(
        source
            .media(&binding(), &sample(3, 1, SampleSide::After), &|| false)
            .unwrap(),
    );
    assert!(source.last_stats().normalized);
    assert!(!Arc::ptr_eq(&red, &red_again));
    assert_eq!(red.pixels(), red_again.pixels());
    source.abort();
    assert_eq!(source.retained_bytes(), 0);
    assert!(source.is_stopped());
    assert_eq!(red.pixels(), red_again.pixels());
}
#[test]
fn cached_pixels_do_not_hide_changed_binding_or_revoked_timing() {
    for changed_binding in [false, true] {
        let (mut source, observed, fail) = source();
        let sample = sample(1, 1, SampleSide::After);
        let old = image(source.media(&binding(), &sample, &|| false).unwrap());
        let mut b = binding();
        if changed_binding {
            b.digest = mo_video_runtime::common::Digest::from_sha256([2; 32]);
        } else {
            fail.set(true);
        }
        assert!(source.media(&b, &sample, &|| false).is_err());
        assert!(source.is_stopped());
        assert_eq!(source.retained_bytes(), 0);
        assert_eq!(observed.decoder_drops.get(), 1);
        assert_eq!(observed.timeline_drops.get(), 1);
        assert!(matches!(
            source.media(&binding(), &sample, &|| false),
            Err(RenderError::Stopped)
        ));
        assert_eq!(old.width(), 2);
    }
}
#[test]
fn budget_color_and_pixel_format_errors_drop_both_capabilities() {
    for fault in 0..3 {
        let observed: Rc<Observed> = Rc::default();
        let mut decoder = Decoder::new(observed.clone());
        decoder.broken_color = fault == 1;
        decoder.broken_pixel = fault == 2;
        let (source_limits, expect_budget) = (
            VisualSourceLimits {
                max_retained_bytes: if fault == 0 { 63 } else { 128 },
                ..Default::default()
            },
            fault == 0,
        );
        let (mut source, _) = source_with(decoder, source_limits);
        let error = source
            .media(&binding(), &sample(1, 1, SampleSide::After), &|| false)
            .err()
            .unwrap();
        if expect_budget {
            assert!(matches!(error, RenderError::Budget));
        } else {
            assert!(matches!(error, RenderError::Unsupported(_)));
        }
        assert!(source.is_stopped());
        assert_eq!(source.retained_bytes(), 0);
        assert_eq!(observed.decoder_drops.get(), 1);
        assert_eq!(observed.timeline_drops.get(), 1);
    }
}
#[test]
fn every_observed_cancel_point_and_callback_panic_discards_owned_cache_and_capabilities() {
    for cached in [false, true] {
        let point = sample(1, 1, SampleSide::After);
        let (mut baseline, _, _) = source();
        if cached {
            baseline.media(&binding(), &point, &|| false).unwrap();
        }
        let calls = Cell::new(0);
        baseline
            .media(&binding(), &point, &|| {
                calls.set(calls.get() + 1);
                false
            })
            .unwrap();
        for at in 1..=calls.get() {
            let (mut source, observed, _) = source();
            if cached {
                source.media(&binding(), &point, &|| false).unwrap();
            }
            let seen = Cell::new(0);
            assert!(matches!(
                source.media(&binding(), &point, &|| {
                    seen.set(seen.get() + 1);
                    seen.get() == at
                }),
                Err(RenderError::Cancelled)
            ));
            assert!(source.is_stopped());
            assert_eq!(source.retained_bytes(), 0);
            assert_eq!(observed.decoder_drops.get(), 1);
            assert_eq!(observed.timeline_drops.get(), 1);
        }
    }
    let (mut source, observed, _) = source();
    assert!(matches!(
        source.resolve(
            &binding(),
            &sample(1, 1, SampleSide::After),
            SampleMode::Seek,
            &|| panic!("host callback")
        ),
        Err(RenderError::CallbackPanic)
    ));
    assert_eq!(observed.decoder_drops.get(), 1);
    assert_eq!(observed.timeline_drops.get(), 1);
}
