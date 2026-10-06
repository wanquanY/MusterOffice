use mo_time::{ExactRatio as R, Rounding};
use mo_video_audio::*;
use std::cell::Cell;
fn ratio(n: i64, d: i64) -> R {
    R::new(n.into(), d.into(), 256).unwrap()
}
fn pcm(values: Vec<f32>, first: i64) -> PcmBlock {
    PcmBlock::new(
        48000,
        ChannelLayout::mono(),
        first,
        values,
        PcmLimits::default(),
        &|| false,
    )
    .unwrap()
}
fn render(s: &SincResampler, input: &PcmBlock, start: &R, first: i64, n: usize) -> PcmBlock {
    s.process(
        input,
        PcmRange::new(input.first_frame(), input.end_frame()).unwrap(),
        start,
        48000,
        first,
        n,
        ResampleLimits::default(),
        &|| false,
    )
    .unwrap()
}
#[test]
fn exact_phase_never_accumulates_rate_error_including_large_and_negative_origins() {
    for (start, step) in [
        (ratio(1, 7), ratio(147, 160)),
        (ratio(-3, 7), ratio(-1001, 300)),
        (R::integer(9_000_000_000_000_000), ratio(1001, 1000)),
        (
            ratio(1, 7),
            R::new(1.into(), (num_bigint::BigInt::from(1) << 127usize) + 1, 256).unwrap(),
        ),
    ] {
        let mut clock = PhaseClock::new(&start, &step, 256).unwrap();
        for index in 0..5000 {
            let expected = start
                .add(&step.mul(&R::integer(index), 256).unwrap(), 256)
                .unwrap();
            let floor = expected.round_integer(Rounding::Floor).unwrap();
            let fraction = expected
                .sub(&R::integer(floor), 256)
                .unwrap()
                .mul(&R::integer(1i64 << 32), 256)
                .unwrap()
                .round_integer(Rounding::Floor)
                .unwrap() as u32;
            assert_eq!(clock.position(), (floor, fraction));
            clock.advance().unwrap();
        }
    }
}
#[test]
fn identity_and_reverse_are_bit_exact_with_no_boundary_shift() {
    let input = pcm((0..100).map(|n| (n as f32 - 50.) / 50.).collect(), 0);
    for (step, start, expected) in [
        (1, 0, input.samples().to_vec()),
        (-1, 99, input.samples().iter().copied().rev().collect()),
    ] {
        let s = SincResampler::new(&R::integer(step), 256, ResampleLimits::default(), &|| false)
            .unwrap();
        assert_eq!(
            render(&s, &input, &R::integer(start), 0, 100).samples(),
            expected
        );
        assert_eq!(
            s.required_range(&R::integer(start), 100).unwrap(),
            PcmRange::new(0, 100).unwrap()
        );
    }
}
#[test]
fn arbitrary_output_partitions_and_random_access_produce_identical_samples() {
    let input = pcm(
        (0..24000)
            .map(|n| libm::sin(n as f64 * 0.371) as f32)
            .collect(),
        -1000,
    );
    for step in [ratio(147, 160), ratio(-1001, 500)] {
        let s = SincResampler::new(&step, 256, ResampleLimits::default(), &|| false).unwrap();
        let start = ratio(60001, 7);
        let full = render(&s, &input, &start, 0, 3000);
        for (offset, count) in [(0, 37), (37, 1024), (1700, 1), (2771, 229), (133, 125)] {
            let phase = start
                .add(&step.mul(&R::integer(offset), 256).unwrap(), 256)
                .unwrap();
            let block = render(&s, &input, &phase, offset, count);
            assert_eq!(
                block.samples(),
                &full.samples()[offset as usize..offset as usize + count]
            );
        }
    }
}
#[test]
fn passband_dc_and_alias_rejection_are_measured_against_analytic_signals() {
    let s = SincResampler::new(&ratio(3, 1), 256, ResampleLimits::default(), &|| false).unwrap();
    for (frequency, amplitude_tolerance) in [(0., 1e-6), (0.02, 3e-6), (0.25, 1e-5)] {
        let input = pcm(
            (0..20000)
                .map(|n| {
                    if frequency == 0. {
                        1.
                    } else {
                        libm::sin(2. * std::f64::consts::PI * frequency * n as f64) as f32
                    }
                })
                .collect(),
            0,
        );
        let out = render(&s, &input, &ratio(30001, 10), 0, 4096);
        let mut max: f64 = 0.;
        let mut squared = 0.;
        for (i, v) in out.samples().iter().enumerate() {
            let expected = if frequency == 0. {
                1.
            } else if frequency < 1. / 6. {
                libm::sin(2. * std::f64::consts::PI * frequency * (3000.1 + 3. * i as f64))
            } else {
                0.
            };
            let error = (f64::from(*v) - expected).abs();
            max = max.max(error);
            squared += error * error;
        }
        eprintln!(
            "frequency={frequency} maximum={max:e} rms={:e}",
            (squared / 4096.).sqrt()
        );
        assert!(
            max < amplitude_tolerance,
            "frequency={frequency} error={max}"
        );
    }
}
#[test]
fn missing_interior_pcm_is_not_zero_padded_and_edges_are_not_renormalized() {
    let s = SincResampler::new(&ratio(1, 2), 256, ResampleLimits::default(), &|| false).unwrap();
    let input = pcm(vec![1.; 100], 0);
    assert!(matches!(
        s.process(
            &input,
            PcmRange::new(0, 1000).unwrap(),
            &R::integer(95),
            48000,
            0,
            10,
            ResampleLimits::default(),
            &|| false
        ),
        Err(AudioError::Resource(_))
    ));
    let out = render(&s, &input, &ratio(-1, 2), 0, 1);
    assert!((out.samples()[0] - 0.5).abs() < 1e-5);
    assert!(s.required_range(&R::integer(i64::MIN), 1).is_err());
    assert!(s.required_range(&R::integer(i64::MAX), 1).is_err());
}
#[test]
fn budgets_and_cancellation_never_produce_a_partial_pcm_block() {
    let input = pcm(vec![1.; 4096], 0);
    assert!(matches!(
        SincResampler::new(
            &ratio(3, 2),
            256,
            ResampleLimits {
                max_kernel_bytes: 32,
                ..ResampleLimits::default()
            },
            &|| false
        ),
        Err(AudioError::Budget)
    ));
    let s = SincResampler::new(&ratio(3, 2), 256, ResampleLimits::default(), &|| false).unwrap();
    assert!(matches!(
        s.process(
            &input,
            PcmRange::new(0, 4096).unwrap(),
            &R::integer(100),
            48000,
            0,
            1024,
            ResampleLimits {
                max_multiply_adds: 100,
                ..ResampleLimits::default()
            },
            &|| false
        ),
        Err(AudioError::Budget)
    ));
    let calls = Cell::new(0);
    let check = || {
        calls.set(calls.get() + 1);
        false
    };
    s.process(
        &input,
        PcmRange::new(0, 4096).unwrap(),
        &R::integer(100),
        48000,
        0,
        1024,
        ResampleLimits::default(),
        &check,
    )
    .unwrap();
    for stop in 1..=calls.get() {
        let count = Cell::new(0);
        let result = s.process(
            &input,
            PcmRange::new(0, 4096).unwrap(),
            &R::integer(100),
            48000,
            0,
            1024,
            ResampleLimits::default(),
            &|| {
                count.set(count.get() + 1);
                count.get() == stop
            },
        );
        assert!(matches!(result, Err(AudioError::Cancelled)));
    }
    assert_eq!(input.samples(), vec![1.; 4096]);
}
