//! Shared read-capability transport and physical lifetime for decode and index sessions.
use crate::admission::{boundary, guarded_boundary};
use crate::response::{Expected, Reply};
use crate::{WorkerBundle, WorkerError};
use mo_media::{DecodeLimits, MediaKind, ReadAt};
use mo_media_wire::{Command, Event, MAX_READ_BYTES, Request, header_bytes};
use mo_native_limits::ProcessLimits;
use mo_native_worker::{CHUNK_BYTES, Session, SessionError};
use std::{
    panic::{AssertUnwindSafe, catch_unwind},
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};
#[derive(Clone, Copy)]
pub(crate) struct OwnerOptions {
    pub max_read_bytes: u64,
    pub process: ProcessLimits,
    pub open_timeout: Duration,
    pub call_timeout: Duration,
}
impl From<crate::WorkerOptions> for OwnerOptions {
    fn from(options: crate::WorkerOptions) -> Self {
        Self {
            max_read_bytes: options.decode.max_read_bytes,
            process: options.process,
            open_timeout: options.open_timeout,
            call_timeout: options.call_timeout,
        }
    }
}
pub(crate) struct Owner<'a> {
    session: Option<Session<Reply>>,
    source: Box<dyn ReadAt + 'a>,
    length: u64,
    options: OwnerOptions,
    call_id: u64,
    read_id: u64,
    read_bytes: u64,
    expected: Arc<Mutex<Expected>>,
    pub(crate) selected: Option<(MediaKind, u64)>,
}
impl<'a> Owner<'a> {
    pub(crate) fn start(
        bundle: &WorkerBundle,
        source: impl ReadAt + 'a,
        length: u64,
        options: OwnerOptions,
        decode: Option<DecodeLimits>,
        start: Instant,
        check: &dyn Fn() -> bool,
    ) -> Result<Self, WorkerError> {
        options
            .process
            .validate()
            .map_err(|_| WorkerError::Invalid("worker process limits"))?;
        if options.max_read_bytes == 0
            || [options.open_timeout, options.call_timeout]
                .iter()
                .any(|t| t.is_zero() || *t > Duration::from_secs(3600))
        {
            return Err(WorkerError::Invalid("worker read budget or timeout"));
        }
        guarded_boundary(start, options.open_timeout, check)?;
        let command = bundle.command()?;
        guarded_boundary(start, options.open_timeout, check)?;
        let expected = Arc::new(Mutex::new(Expected {
            call_id: 0,
            frame: None,
        }));
        let read_expected = Arc::clone(&expected);
        let session = Session::start(command, move |input| {
            let expected = *read_expected
                .lock()
                .map_err(|_| "poisoned expected response")?;
            crate::response::read_reply(input, decode, expected)
        })?;
        Ok(Self {
            session: Some(session),
            source: Box::new(source),
            length,
            options,
            call_id: 0,
            read_id: 0,
            read_bytes: 0,
            expected,
            selected: None,
        })
    }
}
impl Owner<'_> {
    pub(crate) fn call(
        &mut self,
        command: Command,
        check: &dyn Fn() -> bool,
    ) -> Result<Reply, WorkerError> {
        self.call_since(command, Instant::now(), self.options.call_timeout, check)
    }
    pub(crate) fn call_since(
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
                            > self.options.max_read_bytes.saturating_sub(self.read_bytes)
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
    pub(crate) fn close(mut self, check: &dyn Fn() -> bool) -> Result<(), WorkerError> {
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
    pub(crate) fn is_stopped(&self) -> bool {
        self.session.as_ref().is_none_or(Session::is_stopped)
    }
    pub(crate) fn stop(&mut self) {
        self.session.take();
    }
    pub(crate) fn process_id(&self) -> Option<u32> {
        self.session.as_ref().and_then(Session::process_id)
    }
}
#[cfg(all(test, unix))]
mod tests;
