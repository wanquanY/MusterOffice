//! Controlled transport faults, distinct from actual codec integration tests.
use super::*;
use mo_media_wire::{Response, header_bytes};
use std::{
    process::Command as ProcessCommand,
    sync::atomic::{AtomicBool, Ordering},
};
fn options() -> WorkerOptions {
    WorkerOptions {
        decode: DecodeLimits {
            max_input_bytes: 1 << 20,
            max_read_bytes: 1 << 20,
            max_frame_bytes: 1024,
            max_streams: 16,
            max_pixels: 4096,
            max_audio_samples: 4096,
            max_channels: 8,
            max_decode_steps: 1000,
        },
        process: ProcessLimits {
            additional_address_bytes: 32 << 20,
            cpu_seconds: 10,
            open_files: 16,
        },
        open_timeout: Duration::from_secs(5),
        call_timeout: Duration::from_millis(150),
    }
}
fn owner(output: Vec<u8>, completed: Arc<AtomicBool>, cancel_on_response: bool) -> Owner<'static> {
    // Octal escapes contain only backslash/digits, never executable user input.
    // printf and ':' are shell builtins: this fixture spawns no descendants.
    // The bounded busy loop deliberately never drains stdin, exercising both
    // blocked response reads and input-pipe backpressure before owner timeout.
    let encoded: String = output.iter().map(|b| format!("\\{b:03o}")).collect();
    let mut command = ProcessCommand::new("/bin/sh");
    command.args(["-c", &format!("printf '{encoded}'; while :; do :; done")]);
    let expected = Arc::new(Mutex::new(Expected {
        call_id: 0,
        frame: None,
    }));
    let reader_expected = Arc::clone(&expected);
    let limits = options().decode;
    let session = Session::start(command, move |input| {
        let reply = crate::response::read_reply(input, limits, *reader_expected.lock().unwrap())?;
        if cancel_on_response {
            completed.store(true, Ordering::SeqCst);
        }
        Ok(reply)
    })
    .unwrap();
    Owner {
        session: Some(session),
        source: Box::new(|_: u64, out: &mut [u8]| {
            out.fill(3);
            Ok(out.len())
        }),
        length: 100000,
        options: options(),
        call_id: 0,
        read_id: 0,
        read_bytes: 0,
        expected,
        selected: Some((MediaKind::Video, 1)),
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
#[test]
fn cancellation_after_last_response_byte_cannot_return_success() {
    let cancelled = Arc::new(AtomicBool::new(false));
    let mut owner = owner(
        header_bytes(&Response {
            call_id: 1,
            event: Event::Eof,
        })
        .unwrap(),
        Arc::clone(&cancelled),
        true,
    );
    let pid = owner.process_id().unwrap();
    let result = owner.call(Command::Next, &|| cancelled.load(Ordering::SeqCst));
    assert!(matches!(
        result,
        Err(WorkerError::Session(SessionError::Cancelled))
    ));
    assert!(owner.is_stopped());
    reaped(pid);
}
#[test]
fn stalled_output_and_backpressured_resource_replies_are_killed_and_reaped() {
    for reads in [0, 3] {
        let mut output = Vec::new();
        for i in 0..reads {
            output.extend(
                header_bytes(&Response {
                    call_id: 1,
                    event: Event::Read {
                        read_id: i + 1,
                        offset: i * 32768,
                        byte_length: 32768,
                    },
                })
                .unwrap(),
            );
        }
        let mut owner = owner(output, Arc::new(AtomicBool::new(false)), false);
        let pid = owner.process_id().unwrap();
        let start = Instant::now();
        assert!(matches!(
            owner.call(Command::Next, &|| false),
            Err(WorkerError::Session(SessionError::Deadline))
        ));
        assert!(start.elapsed() < Duration::from_secs(2));
        assert!(owner.is_stopped());
        reaped(pid);
    }
}
