mod support;
use mo_native_media::{NativeContainer, WorkerError};
use mo_native_worker::SessionError;
use std::{
    cell::Cell,
    time::{Duration, Instant},
};
use support::*;

#[test]
#[ignore = "requires explicitly built and pinned media worker bundle"]
fn worker_roundtrip_short_reads_seek_eof_owned_frames_and_graceful_close() {
    let bundle = bundle();
    let (data, expected) = wav(4096);
    for max_read in [13, usize::MAX] {
        let calls = Cell::new(0);
        let container = NativeContainer::open(
            &bundle,
            |offset, out: &mut [u8]| {
                calls.set(calls.get() + 1);
                read(&data, offset, out, max_read)
            },
            data.len() as u64,
            options(),
            &|| false,
        )
        .unwrap();
        let pid = container.process_id().unwrap();
        assert_ne!(pid, std::process::id());
        assert_eq!(container.applied_limits().cpu_seconds, 60);
        let mut decoder = container.select(0, &|| false).unwrap();
        let first = decoder.next_frame(&|| false).unwrap().unwrap();
        let saved = first.planes[0].bytes.clone();
        let mut bytes = saved.clone();
        while let Some(frame) = decoder.next_frame(&|| false).unwrap() {
            assert_eq!(frame.info.generation, 1);
            bytes.extend(&frame.planes[0].bytes);
        }
        assert_eq!(bytes, expected);
        assert!(decoder.next_frame(&|| false).unwrap().is_none());
        decoder.seek(0, &|| false).unwrap();
        let again = decoder.next_frame(&|| false).unwrap().unwrap();
        assert_eq!(again.info.generation, 2);
        assert_eq!(again.planes[0].bytes, saved);
        decoder.close(&|| false).unwrap();
        assert_reaped(pid);
        assert_eq!(first.planes[0].bytes, saved);
        if max_read == 13 {
            assert!(calls.get() > 500);
        }
    }
}
#[test]
#[ignore = "requires explicitly built and pinned media worker bundle"]
fn cancellation_kills_reaps_and_poison_is_not_eof() {
    let bundle = bundle();
    let (data, _) = wav(24000);
    let mut decoder = NativeContainer::open(
        &bundle,
        |offset, out: &mut [u8]| read(&data, offset, out, usize::MAX),
        data.len() as u64,
        options(),
        &|| false,
    )
    .unwrap()
    .select(0, &|| false)
    .unwrap();
    let pid = decoder.process_id().unwrap();
    assert!(decoder.next_frame(&|| false).unwrap().is_some());
    assert!(matches!(
        decoder.next_frame(&|| true),
        Err(WorkerError::Session(SessionError::Cancelled))
    ));
    assert!(decoder.is_stopped());
    assert_reaped(pid);
    assert!(matches!(
        decoder.next_frame(&|| false),
        Err(WorkerError::Session(SessionError::Stopped))
    ));
}
#[test]
#[ignore = "requires explicitly built and pinned media worker bundle"]
fn callback_panic_during_resource_transfer_stops_and_reaps() {
    let bundle = bundle();
    let (data, _) = wav(200000);
    let fault = Cell::new(false);
    let mut decoder = NativeContainer::open(
        &bundle,
        |offset, out: &mut [u8]| {
            assert!(!fault.get(), "injected host reader panic");
            read(&data, offset, out, usize::MAX)
        },
        data.len() as u64,
        options(),
        &|| false,
    )
    .unwrap()
    .select(0, &|| false)
    .unwrap();
    let pid = decoder.process_id().unwrap();
    fault.set(true);
    let error = loop {
        match decoder.next_frame(&|| false) {
            Ok(Some(_)) => (),
            Ok(None) => panic!("expected uncached source read"),
            Err(error) => break error,
        }
    };
    assert!(matches!(error, WorkerError::CallbackPanic));
    assert!(decoder.is_stopped());
    assert_reaped(pid);
}
#[test]
#[ignore = "requires explicitly built and pinned media worker bundle"]
fn total_deadline_is_not_renewed_by_each_resource_read() {
    let bundle = bundle();
    let (data, _) = wav(200000);
    let calls = Cell::new(0);
    let slow = Cell::new(false);
    let mut config = options();
    config.call_timeout = Duration::from_millis(300);
    let mut decoder = NativeContainer::open(
        &bundle,
        |offset, out: &mut [u8]| {
            if slow.get() {
                calls.set(calls.get() + 1);
                std::thread::sleep(Duration::from_millis(20));
            }
            read(&data, offset, out, if slow.get() { 13 } else { usize::MAX })
        },
        data.len() as u64,
        config,
        &|| false,
    )
    .unwrap()
    .select(0, &|| false)
    .unwrap();
    let pid = decoder.process_id().unwrap();
    slow.set(true);
    let start = Instant::now();
    let result = decoder
        .seek(150000, &|| false)
        .and_then(|_| decoder.next_frame(&|| false).map(|_| ()));
    assert!(matches!(
        result,
        Err(WorkerError::Session(SessionError::Deadline))
    ));
    assert!(
        calls.get() > 1,
        "deadline should expire during interactive reads"
    );
    assert!(start.elapsed() < Duration::from_secs(2));
    assert!(decoder.is_stopped());
    assert_reaped(pid);
}

#[test]
#[ignore = "requires explicitly built and pinned media worker bundle"]
fn crashed_worker_is_reaped_and_never_returns_an_old_frame() {
    let bundle = bundle();
    let (data, _) = wav(24000);
    let mut decoder = NativeContainer::open(
        &bundle,
        |offset, out: &mut [u8]| read(&data, offset, out, usize::MAX),
        data.len() as u64,
        options(),
        &|| false,
    )
    .unwrap()
    .select(0, &|| false)
    .unwrap();
    let pid = decoder.process_id().unwrap();
    decoder.next_frame(&|| false).unwrap().unwrap();
    rustix::process::kill_process(
        rustix::process::Pid::from_raw(pid as i32).unwrap(),
        rustix::process::Signal::KILL,
    )
    .unwrap();
    assert!(decoder.next_frame(&|| false).is_err());
    assert!(decoder.is_stopped());
    assert_reaped(pid);
}
#[test]
#[ignore = "requires explicitly built and pinned media worker bundle"]
fn dropping_live_decoder_kills_reaps_and_cancel_panic_is_contained() {
    let bundle = bundle();
    let (data, _) = wav(24000);
    let decoder = NativeContainer::open(
        &bundle,
        |offset, out: &mut [u8]| read(&data, offset, out, usize::MAX),
        data.len() as u64,
        options(),
        &|| false,
    )
    .unwrap()
    .select(0, &|| false)
    .unwrap();
    let pid = decoder.process_id().unwrap();
    drop(decoder);
    assert_reaped(pid);
    let mut decoder = NativeContainer::open(
        &bundle,
        |offset, out: &mut [u8]| read(&data, offset, out, usize::MAX),
        data.len() as u64,
        options(),
        &|| false,
    )
    .unwrap()
    .select(0, &|| false)
    .unwrap();
    let pid = decoder.process_id().unwrap();
    assert!(matches!(
        decoder.next_frame(&|| panic!("injected cancel panic")),
        Err(WorkerError::CallbackPanic)
    ));
    assert!(decoder.is_stopped());
    assert_reaped(pid);
}
