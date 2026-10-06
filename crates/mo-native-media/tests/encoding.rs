mod encode_support;
mod support;
use mo_media::*;
use mo_native_media::{NativeEncoder, WorkerError};
use mo_native_worker::SessionError;
use std::{cell::Cell, rc::Rc};
fn config() -> EncodeConfig {
    EncodeConfig {
        profile: EncodeProfile::Ffv1PcmMp4,
        width: 128,
        height: 96,
        video_time_scale: 25,
        frame_rate_num: 25,
        frame_rate_den: 1,
        video_duration: 1,
        video_frames: 1,
        video_bit_rate: 1000000,
        audio_sample_rate: 48000,
        audio_channels: 0,
        audio_bit_rate: 0,
        gop_frames: 1,
        audio_samples: 0,
    }
}
#[test]
#[ignore = "requires explicitly built and pinned worker bundle"]
fn real_encoder_failures_cancel_callback_panic_and_drop_reap() {
    let bundle = support::bundle();
    for mode in 0..10 {
        let fault = Rc::new(Cell::new(false));
        let cancelled = Rc::new(Cell::new(false));
        let write_fault = fault.clone();
        let write_cancel = cancelled.clone();
        let check_cancel = cancelled.clone();
        let check_fault = fault.clone();
        let sink = encode_support::Sink::default();
        let mut output = sink.clone();
        let mut encoder = NativeEncoder::open(
            &bundle,
            move |offset, bytes: &[u8]| {
                if write_fault.get() {
                    match mode {
                        1 => panic!("injected sink panic"),
                        2 => return Err(std::io::Error::other("injected sink error")),
                        3 => return Ok(0),
                        4 => return Ok(bytes.len() + 1),
                        5 => write_cancel.set(true),
                        _ => (),
                    }
                }
                output.write_at(offset, bytes)
            },
            config(),
            encode_support::options(),
            move || {
                assert!(!(mode == 6 && check_fault.get()), "injected cancel panic");
                check_cancel.get()
            },
        )
        .unwrap();
        let pid = encoder.process_id().unwrap();
        assert_ne!(pid, std::process::id());
        fault.set(true);
        if mode == 0 {
            cancelled.set(true);
        }
        if mode == 9 {
            drop(encoder);
            support::assert_reaped(pid);
            continue;
        }
        let y = vec![64; 128 * 96];
        let uv = vec![128; 64 * 48];
        let result = if mode == 8 {
            encoder.finish().map(|_| ())
        } else {
            encoder
                .video(EncodeVideoFrame {
                    pts: 0,
                    duration: if mode == 7 { 2 } else { 1 },
                    planes: [&y, &uv, &uv],
                    strides: [128, 64, 64],
                })
                .and_then(|_| encoder.finish().map(|_| ()))
        };
        let error = result.unwrap_err();
        match mode {
            0 | 5 => assert!(matches!(
                error,
                WorkerError::Session(SessionError::Cancelled)
            )),
            1 | 6 => assert!(matches!(error, WorkerError::CallbackPanic)),
            2 => assert!(matches!(error, WorkerError::Io(_))),
            3 | 4 => assert!(matches!(error, WorkerError::Invalid(_))),
            7 | 8 => assert!(matches!(error, WorkerError::Media(_))),
            _ => unreachable!(),
        }
        assert!(encoder.is_stopped());
        support::assert_reaped(pid);
        assert!(matches!(
            encoder.finish(),
            Err(WorkerError::Session(SessionError::Stopped))
        ));
    }
}
