mod encode_support;
use encode_support::*;
use mo_media_ffmpeg::*;
use std::{cell::Cell, io, rc::Rc};

#[test]
fn browser_profile_is_not_advertised_or_substituted_by_native_encoder() {
    assert!(
        !encoding_profiles()
            .unwrap()
            .contains(&EncodeProfile::H264WebCodecsAacMp4)
    );
    let writes = Cell::new(0);
    let error = Encoder::open(
        |_: u64, bytes: &[u8]| -> io::Result<usize> {
            writes.set(writes.get() + 1);
            Ok(bytes.len())
        },
        config(EncodeProfile::H264WebCodecsAacMp4, 1, false, false),
        EncodeLimits::default(),
        || false,
    )
    .err()
    .unwrap();
    assert_eq!(error.kind, ErrorKind::Unsupported);
    assert_eq!(writes.get(), 0);
}

#[test]
fn lossless_short_tail_fractional_clock_and_audio_partition_roundtrip() {
    assert!(
        encoding_profiles()
            .unwrap()
            .contains(&EncodeProfile::Ffv1PcmMp4)
    );
    for (frames, fractional, audio) in [
        (1, false, false),
        (2, true, true),
        (51, false, true),
        (64, true, true),
    ] {
        let c = config(EncodeProfile::Ffv1PcmMp4, frames, fractional, audio);
        let (bytes, _) = encode(c, 13, 997);
        let (video, samples) = decoded(&bytes, c);
        let expected: Vec<_> = (0..frames)
            .flat_map(|i| planes(i).into_iter().flatten())
            .collect();
        assert_eq!(video, expected);
        assert_eq!(samples, pcm(c, 0, c.audio_samples as usize));
        let (other, _) = encode(c, 0, 4093);
        assert_eq!(decoded(&other, c), (video, samples));
    }
}
#[test]
fn invalid_inputs_poison_session_and_never_finish_candidate() {
    for mode in 0..8 {
        let c = config(EncodeProfile::Ffv1PcmMp4, 3, false, true);
        let mut encoder =
            Encoder::open(Sink::default(), c, EncodeLimits::default(), || false).unwrap();
        let p = planes(0);
        let error = match mode {
            0 => encoder.finish().unwrap_err(),
            1 => {
                let mut f = frame(c, 0, &p);
                f.pts = 1;
                encoder.video(f).unwrap_err()
            }
            2 => {
                let mut f = frame(c, 0, &p);
                f.duration -= 1;
                encoder.video(f).unwrap_err()
            }
            3 => {
                let mut f = frame(c, 0, &p);
                f.planes[0] = &[];
                encoder.video(f).unwrap_err()
            }
            4 => encoder.audio(1, &[0.0; 2]).unwrap_err(),
            5 => encoder.audio(0, &[f32::NAN, 0.0]).unwrap_err(),
            6 => encoder.audio(0, &[0.0; 3]).unwrap_err(),
            _ => {
                let mut f = frame(c, 0, &p);
                f.strides[0] = usize::MAX;
                encoder.video(f).unwrap_err()
            }
        };
        assert_eq!(error.kind, ErrorKind::Invalid);
        assert!(encoder.is_closed());
        assert_eq!(encoder.finish().unwrap_err().kind, ErrorKind::State);
    }
}
#[test]
fn callback_contract_io_panics_and_cancellation_close_native_state() {
    let c = config(EncodeProfile::Ffv1PcmMp4, 1, false, false);
    for mode in 0..4 {
        let error = Encoder::open(
            move |_: u64, bytes: &[u8]| -> io::Result<usize> {
                match mode {
                    0 => Ok(0),
                    1 => Ok(bytes.len() + 1),
                    2 => Err(io::Error::other("sink unavailable")),
                    _ => panic!("host write panic"),
                }
            },
            c,
            EncodeLimits::default(),
            || false,
        )
        .err()
        .unwrap();
        assert_eq!(
            error.kind,
            match mode {
                0 | 1 => ErrorKind::CallbackContract,
                2 => ErrorKind::Io,
                _ => ErrorKind::CallbackPanic,
            }
        );
    }
    let cancelled = Rc::new(Cell::new(false));
    let flag = cancelled.clone();
    let mut encoder = Encoder::open(Sink::default(), c, EncodeLimits::default(), move || {
        flag.get()
    })
    .unwrap();
    cancelled.set(true);
    assert_eq!(
        encoder.video(frame(c, 0, &planes(0))).unwrap_err().kind,
        ErrorKind::Cancelled
    );
    assert!(encoder.is_closed());
    let flag = Rc::new(Cell::new(false));
    let copy = flag.clone();
    let mut encoder = Encoder::open(Sink::default(), c, EncodeLimits::default(), move || {
        assert!(!copy.get(), "cancel panic");
        false
    })
    .unwrap();
    flag.set(true);
    assert_eq!(encoder.finish().unwrap_err().kind, ErrorKind::CallbackPanic);
}
#[test]
fn exact_configuration_admission_and_bounded_backpressure() {
    let c = config(EncodeProfile::Ffv1PcmMp4, 30, false, true);
    for mode in 0..5 {
        let mut bad = c;
        match mode {
            0 => bad.width = 127,
            1 => bad.video_frames = 0,
            2 => bad.video_time_scale = 47999,
            3 => {
                bad.video_time_scale = 999999999;
                bad.frame_rate_num = 1;
                bad.video_frames = 1;
                bad.video_duration = 1;
                bad.audio_sample_rate = 44100;
            }
            _ => bad.audio_channels = 3,
        }
        assert_eq!(
            Encoder::open(Sink::default(), bad, EncodeLimits::default(), || false)
                .err()
                .unwrap()
                .kind,
            ErrorKind::Invalid
        );
    }
    for mode in 0..5 {
        let mut limit = EncodeLimits::default();
        match mode {
            0 => limit.max_file_bytes = 100,
            1 => limit.max_pending_packets = 1,
            2 => limit.max_packet_bytes = 1,
            3 => limit.max_steps = 1,
            _ => limit.max_write_calls = 1,
        }
        let result =
            Encoder::open(Sink(Rc::default(), 13), c, limit, || false).and_then(|mut e| {
                feed(&mut e, c, 997)?;
                e.finish()
            });
        assert_eq!(result.unwrap_err().kind, ErrorKind::Limit, "mode {mode}");
    }
}
#[test]
#[ignore = "actual hardware and independent media tools; run media-encode-check.py"]
fn real_export_profiles() {
    let output = std::path::PathBuf::from(
        std::env::var_os("MO_ENCODE_TEST_OUTPUT").expect("evidence output"),
    );
    std::fs::create_dir(&output).unwrap();
    let mut rows = Vec::new();
    let mut profiles = vec![EncodeProfile::Ffv1PcmMp4];
    if std::env::var_os("MO_ENCODE_TEST_HARDWARE").is_some() {
        profiles.push(EncodeProfile::H264VideoToolboxAacMp4);
    }
    for profile in profiles {
        for (frames, fractional, audio) in [
            (1, false, false),
            (2, false, true),
            (3, false, true),
            (8, true, true),
            (51, false, true),
            (64, true, true),
        ] {
            let c = config(profile, frames, fractional, audio);
            let name = format!(
                "{}-{frames}",
                if profile == EncodeProfile::Ffv1PcmMp4 {
                    "lossless"
                } else {
                    "h264"
                }
            );
            let (bytes, stats) = encode(c, 17, 997);
            let (video, samples) = decoded(&bytes, c);
            std::fs::write(output.join(format!("{name}.mp4")), bytes).unwrap();
            std::fs::write(output.join(format!("{name}.decoded.yuv")), video).unwrap();
            std::fs::write(
                output.join(format!("{name}.decoded.f32")),
                samples
                    .into_iter()
                    .flat_map(f32::to_le_bytes)
                    .collect::<Vec<_>>(),
            )
            .unwrap();
            std::fs::write(
                output.join(format!("{name}.input.yuv")),
                (0..frames)
                    .flat_map(|i| planes(i).into_iter().flatten())
                    .collect::<Vec<_>>(),
            )
            .unwrap();
            std::fs::write(
                output.join(format!("{name}.input.f32")),
                pcm(c, 0, c.audio_samples as usize)
                    .into_iter()
                    .flat_map(f32::to_le_bytes)
                    .collect::<Vec<_>>(),
            )
            .unwrap();
            rows.push(serde_json::json!({"name":name,"config":c,"stats":stats}));
        }
    }
    std::fs::write(
        output.join("exports.json"),
        serde_json::to_vec_pretty(&rows).unwrap(),
    )
    .unwrap();
}
