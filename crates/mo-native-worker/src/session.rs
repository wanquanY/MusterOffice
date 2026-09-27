//! Sequential framed calls to one retained, isolated computation process.
//! The caller pumps non-Send input; one transport thread owns both pipes.
use crate::CHUNK_BYTES;
use std::{
    io::{Read, Write},
    process::{Child, ChildStdout, Command, Stdio},
    sync::mpsc::{self, Receiver, SyncSender, TryRecvError, TrySendError},
    thread::{self, JoinHandle},
    time::{Duration, Instant},
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SessionError {
    Cancelled,
    Deadline,
    Stopped,
    Input(String),
    Transport(String),
    Exit,
    ThreadFailed,
}
impl std::fmt::Display for SessionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "native worker session: {self:?}")
    }
}
impl std::error::Error for SessionError {}

enum Part {
    Bytes(Vec<u8>),
    End,
}

/// One call at a time, no hidden queue of computations or accumulated frames.
/// A transport/input/deadline/cancellation failure permanently stops this owner.
/// The configured worker must not spawn descendants retaining its pipes.
pub struct Session<T> {
    child: Option<Child>,
    sender: Option<SyncSender<Part>>,
    responses: Receiver<Result<T, String>>,
    wake: Receiver<()>,
    transport: Option<JoinHandle<Result<(), String>>>,
}

impl<T: Send + 'static> Session<T> {
    pub fn start(
        mut command: Command,
        mut read: impl FnMut(&mut ChildStdout) -> Result<T, String> + Send + 'static,
    ) -> Result<Self, SessionError> {
        let mut child = command
            .env_clear()
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|e| SessionError::Transport(e.to_string()))?;
        let mut input = child.stdin.take().expect("piped stdin");
        let mut output = child.stdout.take().expect("piped stdout");
        let (sender, receiver) = mpsc::sync_channel(1);
        let (responses_send, responses) = mpsc::sync_channel(1);
        let (wake_send, wake) = mpsc::sync_channel(1);
        // Fallible thread creation: a resource failure must not orphan the child.
        let transport = thread::Builder::new()
            .name("mo-worker-session".into())
            .spawn(move || {
                while let Ok(part) = receiver.recv() {
                    let _ = wake_send.try_send(());
                    match part {
                        Part::Bytes(bytes) => input.write_all(&bytes).map_err(|e| e.to_string())?,
                        Part::End => {
                            let result = read(&mut output);
                            let failed = result.is_err();
                            responses_send
                                .send(result)
                                .map_err(|_| "response abandoned")?;
                            let _ = wake_send.try_send(());
                            if failed {
                                return Err("invalid worker response".into());
                            }
                        }
                    }
                }
                drop(input);
                let mut extra = [0; 1];
                if output.read(&mut extra).map_err(|e| e.to_string())? != 0 {
                    return Err("trailing worker response bytes".into());
                }
                Ok(())
            });
        match transport {
            Ok(transport) => Ok(Self {
                child: Some(child),
                sender: Some(sender),
                responses,
                wake,
                transport: Some(transport),
            }),
            Err(error) => {
                let _ = child.kill();
                let _ = child.wait();
                Err(SessionError::Transport(error.to_string()))
            }
        }
    }

    /// Deadline covers this entire call, including input production and transfer.
    /// The producer is cooperative and must return promptly; its bytes are never
    /// retained after the pipe consumes them. Each chunk is at most 64 KiB.
    pub fn call(
        &mut self,
        timeout: Duration,
        check: &dyn Fn() -> bool,
        mut next: impl FnMut() -> Result<Option<Vec<u8>>, String>,
    ) -> Result<T, SessionError> {
        let start = Instant::now();
        let mut pending = None;
        let mut finished = false;
        let result = (|| loop {
            if self.child.is_none() {
                return Err(SessionError::Stopped);
            }
            if check() {
                return Err(SessionError::Cancelled);
            }
            if start.elapsed() >= timeout {
                return Err(SessionError::Deadline);
            }
            if !finished {
                if pending.is_none() {
                    pending = Some(match next().map_err(SessionError::Input)? {
                        Some(bytes) if bytes.len() <= CHUNK_BYTES => Part::Bytes(bytes),
                        Some(_) => return Err(SessionError::Input("input chunk limit".into())),
                        None => Part::End,
                    });
                }
                let part = pending.take().expect("pending input");
                let last = matches!(part, Part::End);
                match self
                    .sender
                    .as_ref()
                    .ok_or(SessionError::Stopped)?
                    .try_send(part)
                {
                    Ok(()) => finished = last,
                    Err(TrySendError::Full(part)) => pending = Some(part),
                    Err(TrySendError::Disconnected(_)) => {
                        return Err(SessionError::Transport("input closed".into()));
                    }
                }
            }
            match self.responses.try_recv() {
                Ok(result) => {
                    if !finished {
                        return Err(SessionError::Transport("unsolicited response".into()));
                    }
                    // Cancellation racing the last byte still cannot yield a frame.
                    if check() {
                        return Err(SessionError::Cancelled);
                    }
                    if start.elapsed() >= timeout {
                        return Err(SessionError::Deadline);
                    }
                    return result.map_err(SessionError::Transport);
                }
                Err(TryRecvError::Empty) => (),
                Err(TryRecvError::Disconnected) => {
                    return Err(SessionError::Transport("response closed".into()));
                }
            }
            if self
                .child
                .as_mut()
                .expect("live child")
                .try_wait()
                .map_err(|e| SessionError::Transport(e.to_string()))?
                .is_some()
            {
                return Err(SessionError::Exit);
            }
            if pending.is_some() || finished {
                let _ = self.wake.recv_timeout(
                    Duration::from_millis(2).min(timeout.saturating_sub(start.elapsed())),
                );
            }
        })();
        if result.is_err() {
            self.stop();
        }
        result
    }

    /// Close stdin, require no unsolicited output, then verify clean process exit.
    pub fn finish(
        mut self,
        timeout: Duration,
        check: &dyn Fn() -> bool,
    ) -> Result<(), SessionError> {
        if self.child.is_none() {
            return Err(SessionError::Stopped);
        }
        self.sender.take();
        let start = Instant::now();
        loop {
            if check() {
                return Err(SessionError::Cancelled);
            }
            if start.elapsed() >= timeout {
                return Err(SessionError::Deadline);
            }
            if let Some(status) = self
                .child
                .as_mut()
                .expect("live child")
                .try_wait()
                .map_err(|e| SessionError::Transport(e.to_string()))?
            {
                if !status.success() {
                    return Err(SessionError::Exit);
                }
                self.child.take();
                let result = self
                    .transport
                    .take()
                    .expect("transport owner")
                    .join()
                    .map_err(|_| SessionError::ThreadFailed)?;
                return result.map_err(SessionError::Transport);
            }
            thread::sleep(Duration::from_millis(2).min(timeout.saturating_sub(start.elapsed())));
        }
    }
}

impl<T> Session<T> {
    pub fn process_id(&self) -> Option<u32> {
        self.child.as_ref().map(Child::id)
    }
    pub fn is_stopped(&self) -> bool {
        self.child.is_none()
    }
    /// Idempotent stop, including blocked pipe I/O. Never detaches a worker.
    pub fn stop(&mut self) {
        if let Some(mut child) = self.child.take() {
            let _ = child.kill();
            self.sender.take();
            let _ = child.wait();
        }
        self.sender.take();
        if let Some(transport) = self.transport.take() {
            let _ = transport.join();
        }
    }
}
impl<T> Drop for Session<T> {
    fn drop(&mut self) {
        self.stop();
    }
}
