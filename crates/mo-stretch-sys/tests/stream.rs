use mo_stretch_sys::NativeStretch;
use mo_video_audio::*;

struct Input(Vec<f32>);
impl StretchInput for Input {
    fn read(&mut self, first: u64, samples: &mut [f32], _: &dyn Fn() -> bool) -> Result<()> {
        samples.copy_from_slice(&self.0[first as usize..first as usize + samples.len()]);
        Ok(())
    }
}
fn render(input: Vec<f32>, count: usize) -> Vec<f32> {
    let engine = NativeStretch::new(48000, 1, 8 << 20).unwrap();
    let mut stream = StretchStream::new(
        engine,
        48000,
        ChannelLayout::mono(),
        input.len() as u64,
        count as u64,
        StretchLimits::default(),
    )
    .unwrap();
    let mut source = Input(input);
    let mut output = vec![];
    while let Some(block) = stream.next(&mut source, &|| false).unwrap() {
        assert_eq!(block.first_frame(), output.len() as i64);
        output.extend_from_slice(block.samples());
    }
    assert_eq!(output.len(), count);
    assert!(stream.engine().memory().unwrap().1 < 8 << 20);
    output
}
#[test]
fn preserves_pitch_and_repeats_exactly() {
    let input: Vec<_> = (0..96000)
        .map(|i| (i as f64 * std::f64::consts::TAU * 440. / 48000.).sin() as f32 * 0.3)
        .collect();
    for count in [48000, 72000, 128000, 192000, 288000] {
        let output = render(input.clone(), count);
        assert_eq!(output, render(input.clone(), count));
        // Independent positive-crossing count, away from finite-file edges.
        let middle = &output[9600..count - 9600];
        let crossings = middle
            .windows(2)
            .filter(|w| w[0] <= 0. && w[1] > 0.)
            .count();
        let frequency = crossings as f64 * 48000. / middle.len() as f64;
        assert!(
            (frequency - 440.).abs() < 3.,
            "count {count}: {frequency} Hz"
        );
        let rms = (middle.iter().map(|x| f64::from(*x).powi(2)).sum::<f64>() / middle.len() as f64)
            .sqrt();
        assert!((0.15..0.26).contains(&rms), "count {count}: {rms} RMS");
    }
}
#[test]
fn short_clips_and_tail_are_not_silently_discarded() {
    for length in [1, 16, 128, 2048, 5000] {
        let input = vec![0.25; length];
        for count in [length, length * 2, length.div_ceil(2)] {
            let output = render(input.clone(), count);
            assert!(output.iter().any(|v| v.abs() > 1e-8), "{length} -> {count}");
        }
    }
    let mut input = vec![0.; 48000];
    input[45000..].fill(0.25);
    let output = render(input, 96000);
    assert!(output[92000..].iter().map(|x| x.abs()).sum::<f32>() > 100.);
    assert!(output[..80000].iter().all(|x| x.abs() < 1e-5));
}
#[test]
fn rejected_inputs_do_not_advance_engine() {
    let mut engine = NativeStretch::new(48000, 1, 8 << 20).unwrap();
    let info = engine.info();
    assert!(
        engine
            .begin(&vec![f32::NAN; info.input_latency as usize], 1.)
            .is_err()
    );
    engine
        .begin(&vec![0.; info.input_latency as usize], 1.)
        .unwrap();
    assert!(
        engine
            .begin(&vec![0.; info.input_latency as usize], 1.)
            .is_err()
    );
    assert!(engine.process(&[f32::INFINITY], &mut [0.; 128]).is_err());
    let mut output = [1.; 128];
    engine.process(&[0.; 128], &mut output).unwrap();
    assert_eq!(output, [0.; 128]);
    assert!(NativeStretch::new(48000, 1, 4096).is_err());
}

#[test]
fn cancellation_never_publishes_a_partial_chunk_or_reuses_a_lost_output() {
    use std::cell::Cell;
    let make = || {
        StretchStream::new(
            NativeStretch::new(48000, 1, 8 << 20).unwrap(),
            48000,
            ChannelLayout::mono(),
            1000,
            2000,
            StretchLimits::default(),
        )
        .unwrap()
    };
    let mut reference = make();
    let mut source = Input(vec![0.2; 1000]);
    let calls = Cell::new(0);
    let first = reference
        .next(&mut source, &|| {
            calls.set(calls.get() + 1);
            false
        })
        .unwrap()
        .unwrap();
    let mut recoverable = 0;
    let mut poisoned = 0;
    for position in 0..calls.get() {
        let mut stream = make();
        let counter = Cell::new(0);
        assert!(
            matches!(
                stream.next(&mut source, &|| {
                    let index = counter.get();
                    counter.set(index + 1);
                    index == position
                }),
                Err(AudioError::Cancelled)
            ),
            "cancel checkpoint {position}"
        );
        match stream.next(&mut source, &|| false) {
            Ok(Some(block)) => {
                assert_eq!(block.first_frame(), 0);
                assert_eq!(block.samples(), first.samples());
                recoverable += 1;
            }
            Err(AudioError::Invalid("poisoned stretch stream")) => poisoned += 1,
            other => panic!("unexpected cancellation recovery: {other:?}"),
        }
    }
    assert!(recoverable > 0 && poisoned > 0);
}

#[test]
fn stream_admits_all_scratch_and_preroll_work_before_starting() {
    for limits in [
        StretchLimits {
            max_buffer_bytes: 16,
            ..StretchLimits::default()
        },
        StretchLimits {
            max_preroll_calls: 1,
            ..StretchLimits::default()
        },
    ] {
        assert!(matches!(
            StretchStream::new(
                NativeStretch::new(48000, 1, 8 << 20).unwrap(),
                48000,
                ChannelLayout::mono(),
                1000,
                2000,
                limits
            ),
            Err(AudioError::Budget)
        ));
    }
    assert!(
        StretchStream::new(
            NativeStretch::new(48000, 1, 8 << 20).unwrap(),
            44100,
            ChannelLayout::mono(),
            1000,
            2000,
            StretchLimits::default()
        )
        .is_err()
    );
}

#[test]
fn engine_unwind_cannot_leave_reusable_mutated_state() {
    struct Panicking(NativeStretch);
    impl StretchEngine for Panicking {
        fn info(&self) -> StretchInfo {
            self.0.info()
        }
        fn begin(&mut self, input: &[f32], rate: f64) -> Result<()> {
            self.0.begin(input, rate)
        }
        fn process(&mut self, input: &[f32], output: &mut [f32]) -> Result<()> {
            self.0.process(input, output)?;
            panic!("injected host engine panic after DSP mutation");
        }
    }
    let mut stream = StretchStream::new(
        Panicking(NativeStretch::new(48000, 1, 8 << 20).unwrap()),
        48000,
        ChannelLayout::mono(),
        1000,
        2000,
        StretchLimits::default(),
    )
    .unwrap();
    let mut source = Input(vec![0.2; 1000]);
    assert!(
        std::panic::catch_unwind(std::panic::AssertUnwindSafe(
            || stream.next(&mut source, &|| false)
        ))
        .is_err()
    );
    assert!(matches!(
        stream.next(&mut source, &|| false),
        Err(AudioError::Invalid("poisoned stretch stream"))
    ));
}

#[test]
fn maximum_integer_durations_do_not_round_rate_hint_out_of_range() {
    struct Silence;
    impl StretchInput for Silence {
        fn read(&mut self, _: u64, samples: &mut [f32], _: &dyn Fn() -> bool) -> Result<()> {
            samples.fill(0.);
            Ok(())
        }
    }
    let maximum = i64::MAX as u64;
    for (source, output) in [
        (maximum, maximum.div_ceil(64)),
        (maximum.div_ceil(64), maximum),
    ] {
        let mut stream = StretchStream::new(
            NativeStretch::new(48000, 1, 8 << 20).unwrap(),
            48000,
            ChannelLayout::mono(),
            source,
            output,
            StretchLimits::default(),
        )
        .unwrap();
        let block = stream.next(&mut Silence, &|| false).unwrap().unwrap();
        assert_eq!(block.first_frame(), 0);
        assert!(block.samples().iter().all(|v| *v == 0.));
    }
}
