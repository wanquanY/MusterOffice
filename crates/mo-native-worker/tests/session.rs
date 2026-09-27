#![cfg(unix)]
use mo_native_worker::{CHUNK_BYTES, Session, SessionError};
use std::{
    cell::Cell,
    io::Read,
    process::Command,
    rc::Rc,
    time::{Duration, Instant},
};
fn command(script: &str) -> Command {
    let mut command = Command::new("/bin/sh");
    command.args(["-c", script]);
    command
}
fn byte(out: &mut std::process::ChildStdout) -> Result<u8, String> {
    let mut b = [0; 1];
    out.read_exact(&mut b).map_err(|e| e.to_string())?;
    Ok(b[0])
}
fn gone(pid: u32) {
    assert!(
        !Command::new("/bin/kill")
            .args(["-0", &pid.to_string()])
            .stderr(std::process::Stdio::null())
            .status()
            .unwrap()
            .success()
    );
}
#[test]
fn repeated_calls_keep_one_process_and_borrow_non_send_inputs() {
    let mut session = Session::start(command("exec cat"), byte).unwrap();
    let pid = session.process_id().unwrap();
    for expected in b"aBcD" {
        let produced = Rc::new(Cell::new(false));
        let p = produced.clone();
        assert_eq!(
            session
                .call(Duration::from_secs(3), &|| false, || {
                    Ok(if p.replace(true) {
                        None
                    } else {
                        Some(vec![*expected])
                    })
                })
                .unwrap(),
            *expected
        );
        assert!(produced.get());
        assert_eq!(session.process_id(), Some(pid));
    }
    session.finish(Duration::from_secs(3), &|| false).unwrap();
    gone(pid);
}
#[test]
fn blocked_write_and_read_cancel_or_expire_and_permanently_stop() {
    for write in [true, false] {
        for cancel in [true, false] {
            let mut session = Session::start(command("exec sleep 30"), byte).unwrap();
            let pid = session.process_id().unwrap();
            let start = Instant::now();
            let count = Cell::new(0);
            let result = session.call(
                Duration::from_millis(if cancel { 5000 } else { 80 }),
                &|| cancel && start.elapsed() >= Duration::from_millis(60),
                || {
                    count.set(count.get() + 1);
                    Ok(write.then(|| vec![1; CHUNK_BYTES]))
                },
            );
            assert_eq!(
                result.unwrap_err(),
                if cancel {
                    SessionError::Cancelled
                } else {
                    SessionError::Deadline
                }
            );
            assert!(start.elapsed() < Duration::from_secs(3));
            assert!(count.get() < 12);
            gone(pid);
            assert_eq!(
                session
                    .call(Duration::from_secs(1), &|| false, || Ok(None))
                    .unwrap_err(),
                SessionError::Stopped
            );
        }
    }
}
#[test]
fn drop_and_producer_failure_reap_the_process() {
    let session = Session::start(command("exec sleep 30"), byte).unwrap();
    let pid = session.process_id().unwrap();
    drop(session);
    gone(pid);
    let mut session = Session::start(command("exec cat"), byte).unwrap();
    let pid = session.process_id().unwrap();
    assert_eq!(
        session
            .call(Duration::from_secs(3), &|| false, || Err(
                "input unavailable".into()
            ))
            .unwrap_err(),
        SessionError::Input("input unavailable".into())
    );
    gone(pid);
}
#[test]
fn finish_rejects_trailing_output_or_a_hung_worker() {
    let session = Session::start(command("printf extra"), byte).unwrap();
    assert!(matches!(
        session.finish(Duration::from_secs(3), &|| false),
        Err(SessionError::Transport(_))
    ));
    let session = Session::start(command("exec sleep 30"), byte).unwrap();
    let pid = session.process_id().unwrap();
    assert_eq!(
        session
            .finish(Duration::from_millis(60), &|| false)
            .unwrap_err(),
        SessionError::Deadline
    );
    gone(pid);
}
