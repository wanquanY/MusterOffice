//! Native host transport. A bounded producer runs on the caller's thread, so
//! scoped readers need not be Send/Sync. Blocking pipe I/O stays in a worker
//! thread; the owner polls cancellation/deadline and always kills/reaps failure.
use std::{
    io::{Read, Write},
    process::{Command, Stdio},
    sync::mpsc,
    thread,
    time::{Duration, Instant},
};
pub const CHUNK_BYTES: usize = 64 * 1024;

/// One isolated document computation with bounded input and output queues.
/// Events are consumed on the caller thread, allowing non-Send storage owners.
/// A failed consumer, timeout or cancellation always kills and reaps the child.
/// Events are private candidates until this function confirms successful exit.
pub fn exchange_events<E: Send + 'static>(
    mut command: Command,
    timeout: Duration,
    check: &dyn Fn() -> bool,
    mut next: impl FnMut() -> Result<Option<Vec<u8>>, String>,
    read: impl FnOnce(
        &mut std::process::ChildStdout,
        &mut dyn FnMut(E) -> Result<(), String>,
    ) -> Result<(), String>
    + Send
    + 'static,
    mut consume: impl FnMut(E) -> Result<(), String>,
) -> Result<(), String> {
    if timeout.is_zero() {
        return Err("worker deadline is zero".into());
    }
    if check() {
        return Err("worker cancelled".into());
    }
    let before = Instant::now();
    let mut child = command
        .env_clear()
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|e| format!("start worker: {e}"))?;
    let mut input = child.stdin.take().expect("piped stdin");
    let mut output = child.stdout.take().expect("piped stdout");
    let (sender, receiver) = mpsc::sync_channel::<Option<Vec<u8>>>(1);
    let (result_send, result_receive) = mpsc::sync_channel(1);
    let (event_send, event_receive) = mpsc::sync_channel(1);
    let (wake_send, wake_receive) = mpsc::sync_channel(1);
    let transport = thread::spawn(move || {
        let result = (|| {
            while let Some(bytes) = receiver.recv().map_err(|_| "worker input abandoned")? {
                let _ = wake_send.try_send(());
                input.write_all(&bytes).map_err(|e| e.to_string())?;
            }
            drop(input);
            read(&mut output, &mut |event| {
                event_send
                    .send(event)
                    .map_err(|_| "worker consumer abandoned")?;
                let _ = wake_send.try_send(());
                Ok(())
            })?;
            let mut extra = [0; 1];
            if output.read(&mut extra).map_err(|e| e.to_string())? != 0 {
                return Err("trailing worker response bytes".into());
            }
            Ok(())
        })();
        let _ = result_send.send(result);
        let _ = wake_send.try_send(());
    });
    let mut result = None;
    let mut pending = None;
    let mut finished_input = false;
    let mut events_finished = false;
    let completion = loop {
        if before.elapsed() >= timeout {
            break Err("worker timed out".into());
        }
        if check() {
            break Err("worker cancelled".into());
        }
        if !finished_input {
            if pending.is_none() {
                match next() {
                    Ok(part) if part.as_ref().is_none_or(|v| v.len() <= CHUNK_BYTES) => {
                        pending = Some(part)
                    }
                    Ok(_) => break Err("worker input chunk limit".into()),
                    Err(error) => break Err(error),
                }
            }
            let part = pending.take().expect("pending input");
            let last = part.is_none();
            match sender.try_send(part) {
                Ok(()) => finished_input = last,
                Err(mpsc::TrySendError::Full(part)) => pending = Some(part),
                Err(mpsc::TrySendError::Disconnected(_)) => {
                    break Err("worker input transport failed".into());
                }
            }
        }
        match event_receive.try_recv() {
            Ok(event) => {
                if let Err(error) = consume(event) {
                    break Err(error);
                }
            }
            Err(mpsc::TryRecvError::Empty) => (),
            Err(mpsc::TryRecvError::Disconnected) => events_finished = true,
        }
        if result.is_none() {
            match result_receive.try_recv() {
                Ok(value) => result = Some(value),
                Err(mpsc::TryRecvError::Empty) => (),
                Err(mpsc::TryRecvError::Disconnected) => {
                    break Err("worker transport failed".into());
                }
            }
        }
        if events_finished && result.as_ref().is_some_and(Result::is_err) {
            break result.take().unwrap();
        }
        match child.try_wait() {
            Ok(Some(status)) if !status.success() => {
                break Err("worker exited unsuccessfully".into());
            }
            Ok(Some(_)) if result.is_some() && events_finished => break result.take().unwrap(),
            Ok(_) => (),
            Err(error) => break Err(error.to_string()),
        }
        // Pump immediately while the queue accepts input; back off only while
        // waiting for pipe consumption, response completion or process exit.
        if pending.is_some() || finished_input {
            let _ = wake_receive.recv_timeout(
                Duration::from_millis(2).min(timeout.saturating_sub(before.elapsed())),
            );
        }
    };
    if completion.is_err() {
        let _ = child.kill();
    }
    drop(sender);
    drop(event_receive);
    let status = child.wait().map_err(|e| e.to_string());
    if transport.join().is_err() {
        return Err("worker transport thread failed".into());
    }
    status?;
    completion
}

/// Single-response adapter over the same bounded transport.
pub fn exchange_stream<T: Send + 'static>(
    command: Command,
    timeout: Duration,
    check: &dyn Fn() -> bool,
    next: impl FnMut() -> Result<Option<Vec<u8>>, String>,
    read: impl FnOnce(&mut std::process::ChildStdout) -> Result<T, String> + Send + 'static,
) -> Result<T, String> {
    let mut output = None;
    exchange_events(
        command,
        timeout,
        check,
        next,
        move |stdout, emit| emit(read(stdout)?),
        |value| {
            output = Some(value);
            Ok(())
        },
    )?;
    output.ok_or_else(|| "worker returned no response".into())
}

/// Existing bounded-memory callers may pass owned parts. New resource clients
/// should use exchange_stream with their actual range-readable storage.
pub fn exchange<T: Send + 'static>(
    command: Command,
    parts: Vec<Vec<u8>>,
    timeout: Duration,
    read: impl FnOnce(&mut std::process::ChildStdout) -> Result<T, String> + Send + 'static,
) -> Result<T, String> {
    let mut parts = parts.into_iter();
    let mut current = Vec::new();
    let mut offset = 0;
    exchange_stream(
        command,
        timeout,
        &|| false,
        || {
            while offset == current.len() {
                let Some(next) = parts.next() else {
                    return Ok(None);
                };
                current = next;
                offset = 0;
            }
            let end = (offset + CHUNK_BYTES).min(current.len());
            let part = current[offset..end].to_vec();
            offset = end;
            Ok(Some(part))
        },
        read,
    )
}
