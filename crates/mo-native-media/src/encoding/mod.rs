//! Candidate encoder in a verified disposable process. Host callbacks are
//! cooperative; process I/O, encoding and teardown are deadline/cancel bounded.
mod payload;
mod response;
use crate::{
    WorkerBundle, WorkerError,
    admission::{boundary, guarded_boundary, validate_applied},
};
use mo_media::{EncodeConfig, EncodeLimits, EncodeStats, EncodeVideoFrame, VideoEncoder, WriteAt};
use mo_media_wire::{
    Command, Event, PROFILE, Request, encode_audio_bytes, encode_video_lengths, header_bytes,
};
use mo_native_limits::{AppliedLimits, ProcessLimits};
use mo_native_worker::{Session, SessionError};
use response::Reply;
use std::{
    panic::{AssertUnwindSafe, catch_unwind},
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};
#[derive(Debug, Clone, Copy)]
pub struct EncoderOptions {
    pub encode: EncodeLimits,
    pub process: ProcessLimits,
    pub open_timeout: Duration,
    pub call_timeout: Duration,
}
pub struct NativeEncoder<'a> {
    session: Option<Session<Reply>>,
    sink: Box<dyn WriteAt + 'a>,
    cancel: Box<dyn Fn() -> bool + 'a>,
    config: EncodeConfig,
    options: EncoderOptions,
    applied: Option<AppliedLimits>,
    expected: Arc<Mutex<u64>>,
    call: u64,
    write_id: u64,
    write_bytes: u64,
    byte_length: u64,
    video_frames: u32,
    audio_samples: i64,
}
impl<'a> NativeEncoder<'a> {
    pub fn open(
        bundle: &WorkerBundle,
        sink: impl WriteAt + 'a,
        config: EncodeConfig,
        options: EncoderOptions,
        cancel: impl Fn() -> bool + 'a,
    ) -> Result<Self, WorkerError> {
        let start = Instant::now();
        encode_video_lengths(config, options.encode)?;
        options
            .process
            .validate()
            .map_err(|_| WorkerError::Invalid("worker process limits"))?;
        if [options.open_timeout, options.call_timeout]
            .iter()
            .any(|t| t.is_zero() || *t > Duration::from_secs(3600))
        {
            return Err(WorkerError::Invalid("encoder call timeout"));
        }
        guarded_boundary(start, options.open_timeout, &cancel)?;
        let command = bundle.command()?;
        guarded_boundary(start, options.open_timeout, &cancel)?;
        let expected = Arc::new(Mutex::new(0));
        let shared = expected.clone();
        let session = Session::start(command, move |input| {
            let call = *shared.lock().map_err(|_| "poisoned encoder expectation")?;
            response::read(input, call)
        })?;
        let mut encoder = Self {
            session: Some(session),
            sink: Box::new(sink),
            cancel: Box::new(cancel),
            config,
            options,
            applied: None,
            expected,
            call: 0,
            write_id: 0,
            write_bytes: 0,
            byte_length: 0,
            video_frames: 0,
            audio_samples: 0,
        };
        let result = encoder.execute(
            Command::OpenEncoder {
                profile: PROFILE.into(),
                configuration: config,
                limits: options.encode,
                process_limits: options.process,
            },
            std::iter::empty(),
            start,
            options.open_timeout,
        )?;
        match result {
            Event::EncoderOpened {
                profile,
                configuration,
                applied,
            } if profile == PROFILE && configuration == config => {
                validate_applied(applied, options.process)?;
                encoder.applied = Some(applied);
                Ok(encoder)
            }
            _ => Err(WorkerError::Invalid("unexpected encoder open response")),
        }
    }
    pub fn is_stopped(&self) -> bool {
        self.session.as_ref().is_none_or(Session::is_stopped)
    }
    pub fn process_id(&self) -> Option<u32> {
        self.session.as_ref().and_then(Session::process_id)
    }
    pub fn applied_limits(&self) -> AppliedLimits {
        self.applied.expect("admitted encoder")
    }
    pub fn abort(&mut self) {
        self.session.take();
    }
    fn execute(
        &mut self,
        command: Command,
        payload: impl Iterator<Item = Vec<u8>>,
        start: Instant,
        timeout: Duration,
    ) -> Result<Event, WorkerError> {
        let result = catch_unwind(AssertUnwindSafe(|| {
            self.pump(command, payload, start, timeout)
        }));
        let result = match result {
            Ok(value) => value,
            Err(panic) => {
                std::mem::forget(panic);
                Err(WorkerError::CallbackPanic)
            }
        };
        if result.is_err() {
            self.abort();
        }
        result
    }
    fn pump(
        &mut self,
        command: Command,
        payload: impl Iterator<Item = Vec<u8>>,
        start: Instant,
        timeout: Duration,
    ) -> Result<Event, WorkerError> {
        if self.is_stopped() {
            return Err(SessionError::Stopped.into());
        }
        self.call = self
            .call
            .checked_add(1)
            .ok_or(WorkerError::Invalid("encoder call id exhausted"))?;
        *self
            .expected
            .lock()
            .map_err(|_| WorkerError::Invalid("encoder expectation lock"))? = self.call;
        let allow_writes = !matches!(command, Command::Close);
        let header = header_bytes(&Request {
            call_id: self.call,
            command,
        })?;
        let mut initial = std::iter::once(header).chain(payload);
        boundary(start, timeout, &*self.cancel)?;
        let mut reply = self.session.as_mut().ok_or(SessionError::Stopped)?.call(
            timeout.saturating_sub(start.elapsed()),
            &*self.cancel,
            || Ok(initial.next()),
        )?;
        let mut write_calls = 0u32;
        loop {
            boundary(start, timeout, &*self.cancel)?;
            match reply.event {
                Event::Write {
                    write_id,
                    offset,
                    byte_length,
                } => {
                    let limits = self.options.encode;
                    write_calls = write_calls
                        .checked_add(1)
                        .ok_or(WorkerError::Invalid("write calls overflow"))?;
                    if !allow_writes
                        || write_calls > limits.max_write_calls
                        || Some(write_id) != self.write_id.checked_add(1)
                        || offset > limits.max_file_bytes
                        || u64::from(byte_length) > limits.max_file_bytes - offset
                        || u64::from(byte_length)
                            > limits.max_write_bytes.saturating_sub(self.write_bytes)
                        || reply.bytes.len() != byte_length as usize
                    {
                        return Err(WorkerError::Invalid(
                            "candidate write range, sequence or budget",
                        ));
                    }
                    let n = self.sink.write_at(offset, &reply.bytes)?;
                    boundary(start, timeout, &*self.cancel)?;
                    if n == 0 || n > reply.bytes.len() {
                        return Err(WorkerError::Invalid("candidate callback byte count"));
                    }
                    self.write_id = write_id;
                    self.write_bytes += n as u64;
                    self.byte_length = self.byte_length.max(offset + n as u64);
                    let mut bytes = Some(header_bytes(&Request {
                        call_id: self.call,
                        command: Command::WriteReply {
                            write_id,
                            byte_length: n as u32,
                        },
                    })?);
                    reply = self.session.as_mut().ok_or(SessionError::Stopped)?.call(
                        timeout.saturating_sub(start.elapsed()),
                        &*self.cancel,
                        || Ok(bytes.take()),
                    )?;
                }
                Event::Error { error } => return Err(WorkerError::Media(error)),
                event => return Ok(event),
            }
        }
    }
    fn video_inner(&mut self, frame: EncodeVideoFrame<'_>) -> Result<(), WorkerError> {
        if self.is_stopped() {
            return Err(SessionError::Stopped.into());
        }
        let start = Instant::now();
        let command = Command::EncodeVideo {
            pts: frame.pts,
            duration: frame.duration,
        };
        let payload = payload::Video::new(frame, self.config)?;
        let event = self.execute(command, payload, start, self.options.call_timeout)?;
        if !matches!(event, Event::VideoEncoded) {
            return Err(WorkerError::Invalid("unexpected encoded frame response"));
        }
        self.video_frames = self
            .video_frames
            .checked_add(1)
            .ok_or(WorkerError::Invalid("frame count overflow"))?;
        Ok(())
    }
    fn audio_inner(&mut self, first: i64, samples: &[f32]) -> Result<(), WorkerError> {
        if self.is_stopped() {
            return Err(SessionError::Stopped.into());
        }
        let start = Instant::now();
        let channels = self.config.audio_channels as usize;
        if channels == 0 || !samples.len().is_multiple_of(channels) {
            return Err(WorkerError::Invalid("encoder audio layout"));
        }
        let frames = u32::try_from(samples.len() / channels)
            .map_err(|_| WorkerError::Invalid("audio size"))?;
        encode_audio_bytes(self.config, self.options.encode, frames)?;
        if samples.iter().any(|v| !v.is_finite()) {
            return Err(WorkerError::Invalid("nonfinite encoder PCM"));
        }
        let payload = samples
            .chunks(mo_native_worker::CHUNK_BYTES / 4)
            .map(|s| s.iter().flat_map(|v| v.to_le_bytes()).collect());
        let event = self.execute(
            Command::EncodeAudio {
                first_sample: first,
                frames,
            },
            payload,
            start,
            self.options.call_timeout,
        )?;
        if !matches!(event, Event::AudioEncoded) {
            return Err(WorkerError::Invalid("unexpected encoded audio response"));
        }
        self.audio_samples = self
            .audio_samples
            .checked_add(i64::from(frames))
            .ok_or(WorkerError::Invalid("sample count overflow"))?;
        Ok(())
    }
    fn finish_inner(&mut self) -> Result<EncodeStats, WorkerError> {
        let start = Instant::now();
        let timeout = self.options.call_timeout;
        let event = self.execute(Command::FinishEncoder, std::iter::empty(), start, timeout)?;
        let Event::Encoded { stats } = event else {
            return Err(WorkerError::Invalid("unexpected encoder completion"));
        };
        if stats.video_frames != u64::from(self.config.video_frames)
            || stats.video_frames != u64::from(self.video_frames)
            || stats.video_packets != stats.video_frames
            || stats.audio_samples != self.config.audio_samples as u64
            || stats.audio_samples != self.audio_samples as u64
            || stats.byte_length != self.byte_length
            || stats.write_bytes != self.write_bytes
            || stats.byte_length == 0
            || stats.peak_pending_bytes > self.options.encode.max_pending_bytes
        {
            return Err(WorkerError::Invalid(
                "encoder completion does not match observed input/output",
            ));
        }
        if !matches!(
            self.execute(Command::Close, std::iter::empty(), start, timeout)?,
            Event::Closed
        ) {
            return Err(WorkerError::Invalid("unexpected encoder close response"));
        }
        let session = self.session.take().ok_or(SessionError::Stopped)?;
        let result = catch_unwind(AssertUnwindSafe(|| {
            session.finish(timeout.saturating_sub(start.elapsed()), &*self.cancel)
        }));
        match result {
            Ok(value) => value?,
            Err(panic) => {
                std::mem::forget(panic);
                return Err(WorkerError::CallbackPanic);
            }
        }
        Ok(stats)
    }
}
impl VideoEncoder for NativeEncoder<'_> {
    type Error = WorkerError;
    fn configuration(&self) -> EncodeConfig {
        self.config
    }
    fn video(&mut self, frame: EncodeVideoFrame<'_>) -> Result<(), WorkerError> {
        let result = self.video_inner(frame);
        if result.is_err() {
            self.abort();
        }
        result
    }
    fn audio(&mut self, first: i64, samples: &[f32]) -> Result<(), WorkerError> {
        let result = self.audio_inner(first, samples);
        if result.is_err() {
            self.abort();
        }
        result
    }
    fn finish(&mut self) -> Result<EncodeStats, WorkerError> {
        let result = self.finish_inner();
        if result.is_err() {
            self.abort();
        }
        result
    }
}

#[cfg(all(test, unix))]
mod tests;
