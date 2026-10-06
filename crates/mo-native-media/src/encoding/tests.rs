use super::*;
use mo_media::{EncodeProfile, WriteAt};
use mo_media_wire::Response;
use std::{
    io,
    process::Command as ProcessCommand,
    sync::atomic::{AtomicBool, Ordering},
};
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
fn reaped(pid: u32) {
    use rustix::{
        io::Errno,
        process::{Pid, WaitOptions, waitpid},
    };
    assert!(matches!(
        waitpid(Pid::from_raw(pid as i32), WaitOptions::NOHANG),
        Err(Errno::CHILD)
    ));
}
fn encoder(
    output: Vec<u8>,
    sink: impl WriteAt + 'static,
    cancel: Arc<AtomicBool>,
    race: bool,
) -> NativeEncoder<'static> {
    let encoded: String = output.iter().map(|b| format!("\\{b:03o}")).collect();
    let mut command = ProcessCommand::new("/bin/sh");
    command.args(["-c", &format!("printf '{encoded}'; while :; do :; done")]);
    let expected = Arc::new(Mutex::new(0));
    let shared = expected.clone();
    let flag = cancel.clone();
    let session = Session::start(command, move |input| {
        let result = response::read(input, *shared.lock().unwrap());
        if race {
            flag.store(true, Ordering::SeqCst);
        }
        result
    })
    .unwrap();
    NativeEncoder {
        session: Some(session),
        sink: Box::new(sink),
        cancel: Box::new(move || cancel.load(Ordering::SeqCst)),
        config: config(),
        options: EncoderOptions {
            encode: EncodeLimits::default(),
            process: ProcessLimits {
                additional_address_bytes: 32 << 20,
                cpu_seconds: 10,
                open_files: 16,
            },
            open_timeout: Duration::from_secs(5),
            call_timeout: Duration::from_millis(150),
        },
        applied: None,
        expected,
        call: 0,
        write_id: 0,
        write_bytes: 0,
        byte_length: 0,
        video_frames: 0,
        audio_samples: 0,
    }
}
fn bytes(event: Event) -> Vec<u8> {
    header_bytes(&Response { call_id: 1, event }).unwrap()
}
#[test]
fn blocked_encoder_input_or_output_and_last_response_cancel_are_reaped() {
    for mode in 0..3 {
        let flag = Arc::new(AtomicBool::new(false));
        let mut e = encoder(
            if mode == 2 {
                bytes(Event::AudioEncoded)
            } else {
                vec![]
            },
            |_: u64, b: &[u8]| Ok(b.len()),
            flag,
            mode == 2,
        );
        let pid = e.process_id().unwrap();
        let start = Instant::now();
        let payload = std::iter::repeat_n(vec![0; 65536], if mode == 1 { 16 } else { 0 });
        let error = e
            .execute(
                Command::FinishEncoder,
                payload,
                Instant::now(),
                e.options.call_timeout,
            )
            .unwrap_err();
        assert!(matches!(
            error,
            WorkerError::Session(SessionError::Deadline | SessionError::Cancelled)
        ));
        assert!(start.elapsed() < Duration::from_secs(2));
        assert!(e.is_stopped());
        reaped(pid);
    }
}
#[test]
fn forged_write_ranges_sequences_and_callback_failures_abort() {
    for mode in 0..8 {
        let mut output = bytes(Event::Write {
            write_id: if mode == 0 { 2 } else { 1 },
            offset: if mode == 1 { u64::MAX } else { 0 },
            byte_length: 4,
        });
        output.extend([1, 2, 3, 4]);
        let flag = Arc::new(AtomicBool::new(false));
        let for_sink = flag.clone();
        let mut e = encoder(
            output,
            move |_: u64, b: &[u8]| -> io::Result<usize> {
                match mode {
                    2 => Ok(0),
                    3 => Ok(b.len() + 1),
                    4 => Err(io::Error::other("write failure")),
                    5 => panic!("sink panic"),
                    6 => {
                        for_sink.store(true, Ordering::SeqCst);
                        Ok(b.len())
                    }
                    _ => panic!("must reject before invoking sink"),
                }
            },
            flag,
            false,
        );
        let pid = e.process_id().unwrap();
        let error = e
            .execute(
                if mode == 7 {
                    Command::Close
                } else {
                    Command::FinishEncoder
                },
                std::iter::empty(),
                Instant::now(),
                e.options.call_timeout,
            )
            .unwrap_err();
        assert!(matches!(
            error,
            WorkerError::Invalid(_)
                | WorkerError::Io(_)
                | WorkerError::CallbackPanic
                | WorkerError::Session(SessionError::Cancelled)
        ));
        assert!(e.is_stopped());
        reaped(pid);
    }
}
#[test]
fn write_headers_reject_unsolicited_or_oversized_data_before_payload_read() {
    for event in [
        Event::Write {
            write_id: 1,
            offset: 0,
            byte_length: 0,
        },
        Event::Write {
            write_id: 1,
            offset: 0,
            byte_length: u32::MAX,
        },
        Event::Read {
            read_id: 1,
            offset: 0,
            byte_length: 8,
        },
    ] {
        let mut header = io::Cursor::new(bytes(event));
        assert!(response::read(&mut header, 1).is_err());
    }
    let mut header = io::Cursor::new(bytes(Event::Write {
        write_id: 1,
        offset: 0,
        byte_length: 4,
    }));
    assert!(response::read(&mut header, 2).is_err()); // no payload exists, wrong call checked first
}
#[test]
fn transport_packs_active_rows_with_no_padding_and_bounded_chunks() {
    let c = config();
    let widths = [128, 64, 64];
    let heights = [96, 48, 48];
    let planes: Vec<Vec<u8>> = (0..3)
        .map(|p| {
            let mut bytes = vec![99; (heights[p] - 1) * (widths[p] + 7) + widths[p]];
            for row in 0..heights[p] {
                bytes[row * (widths[p] + 7)..row * (widths[p] + 7) + widths[p]].fill(p as u8);
            }
            bytes
        })
        .collect();
    let make = || EncodeVideoFrame {
        pts: 0,
        duration: 1,
        planes: [&planes[0], &planes[1], &planes[2]],
        strides: [135, 71, 71],
    };
    let packets: Vec<_> = payload::Video::new(make(), c).unwrap().collect();
    assert!(packets.iter().all(|p| p.len() <= 65536));
    let packed: Vec<_> = packets.into_iter().flatten().collect();
    assert_eq!(packed.len(), 18432);
    assert!(!packed.contains(&99));
    let mut bad = make();
    bad.strides[0] = usize::MAX;
    assert!(payload::Video::new(bad, c).is_err());
}
