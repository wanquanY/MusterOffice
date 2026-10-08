mod support;
use mo_common::Ticks;
use mo_time::ExactRatio;
use mo_video_export::*;
use support::*;

#[test]
fn acknowledgement_is_exact_and_retries_do_not_skip_work() {
    let d = compile(document());
    let plan = ExportPlan::new(&d, options(), ExportLimits::default(), &|| false).unwrap();
    let mut schedule = plan.schedule();
    let first = schedule.next();
    assert_eq!(schedule.next(), first);
    let ExportWork::Video(mut altered) = first else {
        panic!()
    };
    altered.duration = Ticks::new(altered.duration.get() + 1);
    assert!(matches!(
        schedule.accept(ExportWork::Video(altered)),
        Err(ScheduleError::Acknowledgement)
    ));
    assert_eq!(schedule.progress(), ExportProgress::default());
    schedule.accept(first).unwrap();
    assert!(matches!(
        schedule.accept(first),
        Err(ScheduleError::Acknowledgement)
    ));
    assert_eq!(schedule.progress().video_frames, 1);
    while schedule.next() != ExportWork::Complete {
        schedule.accept(schedule.next()).unwrap();
    }
    assert_eq!(
        schedule.progress().video_frames,
        plan.configuration().video_frames
    );
    assert_eq!(
        schedule.progress().audio_samples,
        plan.configuration().audio_samples
    );
    assert!(matches!(
        schedule.accept(ExportWork::Complete),
        Err(ScheduleError::Acknowledgement)
    ));
    assert!(matches!(
        schedule.video(plan.configuration().video_frames),
        Err(ScheduleError::Range)
    ));
}
#[test]
fn fractional_video_and_audio_transport_uses_absolute_endpoints_and_retains_residuals() {
    let d = compile(document());
    let plan = ExportPlan::new(&d, options(), ExportLimits::default(), &|| false).unwrap();
    let mut schedule = plan.schedule();
    let mut previous_video_end = 0;
    let mut previous_audio_end = 0;
    while schedule.next() != ExportWork::Complete {
        let work = schedule.next();
        let (clock, start, end, scale, previous) = match work {
            ExportWork::Video(v) => (
                v.codec_interval().unwrap(),
                v.pts.get(),
                v.pts.get() + v.duration.get(),
                v.timescale,
                &mut previous_video_end,
            ),
            ExportWork::Audio(a) => (
                a.codec_interval().unwrap(),
                a.first_sample.get(),
                a.first_sample.get() + i64::from(a.frames),
                a.sample_rate,
                &mut previous_audio_end,
            ),
            ExportWork::Complete => unreachable!(),
        };
        assert_eq!(clock.timestamp_us.get(), *previous);
        *previous = clock.timestamp_us.get() + clock.duration_us.get();
        for (exact, transport, residual) in [
            (start, clock.timestamp_us.get(), &clock.start_residual),
            (end, *previous, &clock.end_residual),
        ] {
            let recovered = ExactRatio::integer(transport)
                .div(&ExactRatio::integer(1_000_000), 256)
                .unwrap()
                .add(residual, 256)
                .unwrap();
            let original = ExactRatio::integer(exact)
                .div(&ExactRatio::integer(i64::from(scale)), 256)
                .unwrap();
            assert!(recovered.compare(&original).is_eq());
        }
        schedule.accept(work).unwrap();
    }
    assert_eq!(previous_video_end, 503271);
    assert_eq!(previous_audio_end, 503271);
    let last = schedule
        .video(plan.configuration().video_frames - 1)
        .unwrap();
    assert_eq!(
        last.pts.get() + last.duration.get(),
        plan.configuration().video_duration
    );
    assert!(last.duration.get() < schedule.video(0).unwrap().duration.get());
}
#[test]
fn hour_of_ntsc_frames_is_independent_of_audio_partition() {
    let mut source = document();
    source
        .clips
        .get_mut(&mo_video_model::ClipId::new("green").unwrap())
        .unwrap()
        .timeline
        .end = t(3600 * 48000 + 1);
    source.audio_sample_rate = 44100;
    let d = compile(source);
    let mut baselines = Vec::new();
    for block in [1024, 4093] {
        let plan = ExportPlan::new(
            &d,
            options(),
            ExportLimits {
                audio_block_frames: block,
                ..Default::default()
            },
            &|| false,
        )
        .unwrap();
        let mut s = plan.schedule();
        let mut count = 0;
        while s.next() != ExportWork::Complete {
            let work = s.next();
            if let ExportWork::Video(v) = work {
                // Multiples of 1001/30000 seconds; no repeated rounded duration.
                assert_eq!(
                    i128::from(v.pts.get()) * 30000,
                    i128::from(v.index) * 1001 * i128::from(v.timescale)
                );
                assert_eq!(v.key_frame, v.index % 25 == 0);
                count += 1;
            }
            s.accept(work).unwrap();
        }
        baselines.push((
            s.progress(),
            s.video(count - 1).unwrap().codec_interval().unwrap(),
        ));
    }
    assert_eq!(baselines[0], baselines[1]);
    assert_eq!(baselines[0].0.video_frames, 107893);
    assert_eq!(baselines[0].0.audio_samples, 158760001);
}
#[test]
fn exact_native_tail_is_not_extended_to_fit_browser_microseconds() {
    let sample = ExportVideoSample {
        index: 0,
        pts: Ticks::new(0),
        duration: Ticks::new(1),
        timescale: 300_000_000,
        key_frame: true,
    };
    assert!(matches!(
        sample.codec_interval(),
        Err(ScheduleError::Transport)
    ));
    let overflow = ExportVideoSample {
        pts: Ticks::new(i64::MAX),
        ..sample
    };
    assert!(overflow.codec_interval().is_err());
    let audio = ExportAudioSample {
        first_sample: Ticks::new(-1),
        frames: 1024,
        sample_rate: 48000,
    };
    assert!(audio.codec_interval().is_err());
}
#[test]
fn wire_counts_are_strings_and_silent_export_has_no_audio_steps() {
    let d = compile(document());
    let plan = ExportPlan::new(
        &d,
        ExportOptions {
            audio: ExportAudio::Omit,
            ..options()
        },
        ExportLimits::default(),
        &|| false,
    )
    .unwrap();
    let mut s = plan.schedule();
    let value = serde_json::to_value(s.next()).unwrap();
    assert_eq!(value["kind"], "video");
    assert_eq!(value["pts"], "0");
    assert!(value["duration"].is_string());
    while s.next() != ExportWork::Complete {
        assert!(matches!(s.next(), ExportWork::Video(_)));
        s.accept(s.next()).unwrap();
    }
    assert_eq!(s.progress().audio_samples, 0);
}
