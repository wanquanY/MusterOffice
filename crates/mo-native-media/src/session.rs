use crate::WorkerBundle;
use crate::admission::validate_applied;
use crate::owner::Owner;
use mo_media::{DecodeLimits, DecodeTimeline, DecodedFrame, MediaError, ReadAt, StreamInfo};
use mo_media_wire::{Command, Event, PROFILE, validate_limits, validate_streams};
use mo_native_limits::{AppliedLimits, ProcessLimits};
use mo_native_worker::SessionError;
use std::{
    fmt, io,
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
pub struct NativeContainer<'a> {
    timeline: DecodeTimeline,
    owner: Owner<'a>,
    streams: Vec<StreamInfo>,
    applied: AppliedLimits,
}
pub struct NativeDecoder<'a> {
    timeline: DecodeTimeline,
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
        Self::open_with_timeline(
            bundle,
            source,
            byte_length,
            options,
            DecodeTimeline::Demuxer,
            cancelled,
        )
    }
    pub fn open_with_timeline(
        bundle: &WorkerBundle,
        source: impl ReadAt + 'a,
        byte_length: u64,
        options: WorkerOptions,
        timeline: DecodeTimeline,
        cancelled: &dyn Fn() -> bool,
    ) -> Result<Self, WorkerError> {
        let start = Instant::now();
        validate_limits(options.decode, byte_length)?;
        let limits = options.decode;
        let mut owner = Owner::start(
            bundle,
            source,
            byte_length,
            options.into(),
            Some(limits),
            start,
            cancelled,
        )?;
        let reply = owner.call_since(
            Command::Open {
                profile: PROFILE.into(),
                timeline,
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
                timeline: actual_timeline,
                streams,
                applied,
            } if profile == PROFILE && timeline == actual_timeline => {
                validate_streams(&streams, limits)?;
                validate_applied(applied, options.process)?;
                Ok(Self {
                    timeline,
                    owner,
                    streams,
                    applied,
                })
            }
            _ => Err(WorkerError::Invalid("unexpected open response")),
        }
    }
    pub fn timeline(&self) -> DecodeTimeline {
        self.timeline
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
            timeline: self.timeline,
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
    pub fn timeline(&self) -> DecodeTimeline {
        self.timeline
    }
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
    /// Low-level demux seek in stream ticks. For the fixed raw ISO BMFF backend,
    /// use an indexed access candidate's original DTS to find its access point;
    /// a reordered picture's PTS is not an interchangeable decode position.
    /// IndexedVideoSampler provides this policy and verifies the resulting PTS.
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
