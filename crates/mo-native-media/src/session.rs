use crate::WorkerBundle;
use crate::response::{Expected, Reply};
use mo_media::{DecodeLimits, DecodedFrame, MediaError, MediaKind, ReadAt, StreamInfo};
use mo_media_wire::{
    Command, Event, MAX_READ_BYTES, PROFILE, Request, header_bytes, validate_limits,
    validate_streams,
};
use mo_native_limits::{AppliedLimits, ProcessLimits};
use mo_native_worker::{CHUNK_BYTES, Session, SessionError};
use std::{
    fmt, io,
    panic::{AssertUnwindSafe, catch_unwind},
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

#[derive(Debug)]
pub enum WorkerError {
    Invalid(&'static str),
    Artifact(String),
    Io(io::Error),
    Session(SessionError),
    Media(MediaError),
    CallbackPanic,
}
impl fmt::Display for WorkerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "native media: {self:?}")
    }
}
impl std::error::Error for WorkerError {}
impl From<io::Error> for WorkerError {
    fn from(e: io::Error) -> Self {
        Self::Io(e)
    }
}
impl From<SessionError> for WorkerError {
    fn from(e: SessionError) -> Self {
        Self::Session(e)
    }
}
#[derive(Clone, Copy, Debug)]
pub struct WorkerOptions {
    pub decode: DecodeLimits,
    pub process: ProcessLimits,
    /// Admission includes bundle verification, spawn, probing and input transfer.
    pub open_timeout: Duration,
    /// Each later operation has one deadline across all resource round trips.
    pub call_timeout: Duration,
}
struct Owner<'a> {
    session: Option<Session<Reply>>,
    source: Box<dyn ReadAt + 'a>,
    length: u64,
    options: WorkerOptions,
    call_id: u64,
    read_id: u64,
    read_bytes: u64,
    expected: Arc<Mutex<Expected>>,
    selected: Option<(MediaKind, u64)>,
}
pub struct NativeContainer<'a> {
    owner: Owner<'a>,
    streams: Vec<StreamInfo>,
    applied: AppliedLimits,
}
pub struct NativeDecoder<'a> {
    owner: Owner<'a>,
    stream: StreamInfo,
    applied: AppliedLimits,
}
impl<'a> NativeContainer<'a> {
    pub fn open(
        bundle: &WorkerBundle,
        source: impl ReadAt + 'a,
        byte_length: u64,
        options: WorkerOptions,
        cancelled: &dyn Fn() -> bool,
    ) -> Result<Self, WorkerError> {
        let start = Instant::now();
        validate_limits(options.decode, byte_length)?;
        options
            .process
            .validate()
            .map_err(|_| WorkerError::Invalid("worker process limits"))?;
        if [options.open_timeout, options.call_timeout]
            .iter()
            .any(|t| t.is_zero() || *t > Duration::from_secs(3600))
        {
            return Err(WorkerError::Invalid("worker call timeout"));
        }
        let limits = options.decode;
        guarded_boundary(start, options.open_timeout, cancelled)?;
        let command = bundle.command()?;
        guarded_boundary(start, options.open_timeout, cancelled)?;
        let expected = Arc::new(Mutex::new(Expected {
            call_id: 0,
            frame: None,
        }));
        let read_expected = Arc::clone(&expected);
        let session = Session::start(command, move |input| {
            let expected = *read_expected
                .lock()
                .map_err(|_| "poisoned expected response")?;
            crate::response::read_reply(input, limits, expected)
        })?;
        let mut owner = Owner {
            session: Some(session),
            source: Box::new(source),
            length: byte_length,
            options,
            call_id: 0,
            read_id: 0,
            read_bytes: 0,
            expected,
            selected: None,
        };
        let reply = owner.call_since(
            Command::Open {
                profile: PROFILE.into(),
                byte_length,
                limits,
                process_limits: options.process,
            },
            start,
            options.open_timeout,
            cancelled,
        )?;
        match reply.event {
            Event::Opened {
                profile,
                streams,
                applied,
            } if profile == PROFILE => {
                validate_streams(&streams, limits)?;
                validate_applied(applied, options.process)?;
                Ok(Self {
                    owner,
                    streams,
                    applied,
                })
            }
            _ => Err(WorkerError::Invalid("unexpected open response")),
        }
    }
    pub fn streams(&self) -> &[StreamInfo] {
        &self.streams
    }
    pub fn applied_limits(&self) -> AppliedLimits {
        self.applied
    }
    pub fn process_id(&self) -> Option<u32> {
        self.owner.process_id()
    }
    pub fn select(
        mut self,
        index: u32,
        cancelled: &dyn Fn() -> bool,
    ) -> Result<NativeDecoder<'a>, WorkerError> {
        let stream = self
            .streams
            .iter()
            .find(|s| s.index == index)
            .cloned()
            .ok_or(WorkerError::Invalid("stream index"))?;
        let reply = self.owner.call(Command::Select { index }, cancelled)?;
        if !matches!(reply.event, Event::Selected) {
            return Err(WorkerError::Invalid("unexpected select response"));
        }
        self.owner.selected = Some((stream.kind, 1));
        Ok(NativeDecoder {
            owner: self.owner,
            stream,
            applied: self.applied,
        })
    }
    pub fn close(self, cancelled: &dyn Fn() -> bool) -> Result<(), WorkerError> {
        self.owner.close(cancelled)
    }
}
impl NativeDecoder<'_> {
    pub fn stream(&self) -> &StreamInfo {
        &self.stream
    }
    pub fn applied_limits(&self) -> AppliedLimits {
        self.applied
    }
    pub fn process_id(&self) -> Option<u32> {
        self.owner.process_id()
    }
    pub fn is_stopped(&self) -> bool {
        self.owner.is_stopped()
    }
    pub fn next_frame(
        &mut self,
        cancelled: &dyn Fn() -> bool,
    ) -> Result<Option<DecodedFrame>, WorkerError> {
        let reply = self.owner.call(Command::Next, cancelled)?;
        match reply.event {
            Event::Frame { info, .. }
                if self.owner.selected == Some((info.kind, info.generation)) =>
            {
                Ok(Some(DecodedFrame {
                    info,
                    planes: reply.planes,
                }))
            }
            Event::Eof => Ok(None),
            _ => {
                self.owner.stop();
                Err(WorkerError::Invalid(
                    "unexpected frame kind, generation or response",
                ))
            }
        }
    }
    pub fn seek(&mut self, ticks: i64, cancelled: &dyn Fn() -> bool) -> Result<(), WorkerError> {
        let Some(generation) = self.owner.selected.and_then(|(_, n)| n.checked_add(1)) else {
            self.owner.stop();
            return Err(WorkerError::Invalid("seek generation exhausted"));
        };
        let reply = self.owner.call(Command::Seek { ticks }, cancelled)?;
        if !matches!(reply.event, Event::Seeked) {
            self.owner.stop();
            return Err(WorkerError::Invalid("unexpected seek response"));
        }
        self.owner.selected = Some((self.stream.kind, generation));
        Ok(())
    }
    pub fn close(self, cancelled: &dyn Fn() -> bool) -> Result<(), WorkerError> {
        self.owner.close(cancelled)
    }
}
impl Owner<'_> {
    fn call(&mut self, command: Command, check: &dyn Fn() -> bool) -> Result<Reply, WorkerError> {
        self.call_since(command, Instant::now(), self.options.call_timeout, check)
    }
    fn call_since(
        &mut self,
        command: Command,
        start: Instant,
        timeout: Duration,
        check: &dyn Fn() -> bool,
    ) -> Result<Reply, WorkerError> {
        if self.is_stopped() {
            return Err(SessionError::Stopped.into());
        }
        let result = catch_unwind(AssertUnwindSafe(|| {
            self.pump(command, start, timeout, check)
        }));
        let result = match result {
            Ok(result) => result,
            Err(payload) => {
                std::mem::forget(payload);
                Err(WorkerError::CallbackPanic)
            }
        };
        if result.is_err() {
            self.stop();
        }
        result
    }
    fn pump(
        &mut self,
        command: Command,
        start: Instant,
        timeout: Duration,
        check: &dyn Fn() -> bool,
    ) -> Result<Reply, WorkerError> {
        self.call_id = self
            .call_id
            .checked_add(1)
            .ok_or(WorkerError::Invalid("call id exhausted"))?;
        *self
            .expected
            .lock()
            .map_err(|_| WorkerError::Invalid("expected response lock"))? = Expected {
            call_id: self.call_id,
            frame: if matches!(command, Command::Next) {
                self.selected
            } else {
                None
            },
        };
        let mut bytes = header_bytes(&Request {
            call_id: self.call_id,
            command,
        })?;
        loop {
            boundary(start, timeout, check)?;
            let mut parts = bytes.chunks(CHUNK_BYTES);
            let reply = self.session.as_mut().ok_or(SessionError::Stopped)?.call(
                timeout.saturating_sub(start.elapsed()),
                check,
                || Ok(parts.next().map(|p| p.to_vec())),
            )?;
            if reply.call_id != self.call_id {
                return Err(WorkerError::Invalid("worker response call id"));
            }
            match reply.event {
                Event::Read {
                    read_id,
                    offset,
                    byte_length,
                } => {
                    if Some(read_id) != self.read_id.checked_add(1)
                        || byte_length == 0
                        || byte_length > MAX_READ_BYTES
                        || offset > self.length
                        || u64::from(byte_length) > self.length - offset
                        || u64::from(byte_length)
                            > self
                                .options
                                .decode
                                .max_read_bytes
                                .saturating_sub(self.read_bytes)
                    {
                        return Err(WorkerError::Invalid(
                            "worker resource range, sequence or byte budget",
                        ));
                    }
                    self.read_id = read_id;
                    let mut buffer = vec![0u8; byte_length as usize];
                    let n = self.source.read_at(offset, &mut buffer)?;
                    boundary(start, timeout, check)?;
                    if n == 0 || n > buffer.len() {
                        return Err(WorkerError::Invalid("resource callback byte count"));
                    }
                    self.read_bytes += n as u64;
                    bytes = header_bytes(&Request {
                        call_id: self.call_id,
                        command: Command::ReadReply {
                            read_id,
                            byte_length: n as u32,
                        },
                    })?;
                    bytes.extend_from_slice(&buffer[..n]);
                }
                Event::Error { error } => return Err(WorkerError::Media(error)),
                _ => {
                    boundary(start, timeout, check)?;
                    return Ok(reply);
                }
            }
        }
    }
    fn close(mut self, check: &dyn Fn() -> bool) -> Result<(), WorkerError> {
        let start = Instant::now();
        if !matches!(
            self.call_since(Command::Close, start, self.options.call_timeout, check)?
                .event,
            Event::Closed
        ) {
            return Err(WorkerError::Invalid("unexpected close response"));
        }
        let session = self.session.take().ok_or(SessionError::Stopped)?;
        match catch_unwind(AssertUnwindSafe(|| {
            session.finish(
                self.options.call_timeout.saturating_sub(start.elapsed()),
                check,
            )
        })) {
            Ok(result) => result?,
            Err(payload) => {
                std::mem::forget(payload);
                return Err(WorkerError::CallbackPanic);
            }
        }
        Ok(())
    }
    fn is_stopped(&self) -> bool {
        self.session.as_ref().is_none_or(Session::is_stopped)
    }
    fn stop(&mut self) {
        self.session.take();
    }
    fn process_id(&self) -> Option<u32> {
        self.session.as_ref().and_then(Session::process_id)
    }
}
fn boundary(
    start: Instant,
    timeout: Duration,
    check: &dyn Fn() -> bool,
) -> Result<(), WorkerError> {
    if check() {
        return Err(SessionError::Cancelled.into());
    }
    if start.elapsed() >= timeout {
        return Err(SessionError::Deadline.into());
    }
    Ok(())
}
fn validate_applied(applied: AppliedLimits, request: ProcessLimits) -> Result<(), WorkerError> {
    if applied.startup_virtual_bytes == 0
        || applied.address_ceiling_bytes <= applied.startup_virtual_bytes
        || applied.address_ceiling_bytes - applied.startup_virtual_bytes
            > request.additional_address_bytes
        || applied.cpu_seconds == 0
        || applied.cpu_seconds > u64::from(request.cpu_seconds)
        || applied.open_files < 3
        || applied.open_files > u64::from(request.open_files)
        || applied.core_file_bytes != 0
        || applied.regular_file_bytes != 0
    {
        return Err(WorkerError::Invalid(
            "worker did not establish requested resource ceilings",
        ));
    }
    Ok(())
}

fn guarded_boundary(
    start: Instant,
    timeout: Duration,
    check: &dyn Fn() -> bool,
) -> Result<(), WorkerError> {
    match catch_unwind(AssertUnwindSafe(|| boundary(start, timeout, check))) {
        Ok(result) => result,
        Err(payload) => {
            std::mem::forget(payload);
            Err(WorkerError::CallbackPanic)
        }
    }
}

#[cfg(all(test, unix))]
mod tests;
