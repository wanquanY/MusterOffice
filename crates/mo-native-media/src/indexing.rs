//! A retained raw index inside the verified, resource-limited worker.
use crate::{
    WorkerBundle, WorkerError,
    admission::validate_applied,
    owner::{Owner, OwnerOptions},
};
use mo_media::ReadAt;
pub use mo_media_isobmff::{ByteRange, Edit, IndexLimits, Sample};
use mo_media_wire::{Command, Event, PROFILE};
pub use mo_media_wire::{
    IndexPage, IndexQuery, IndexSummary, IndexTrack, MAX_INDEX_BLOB_BYTES, MAX_INDEX_ITEMS,
};
use mo_native_limits::{AppliedLimits, ProcessLimits};
use std::time::{Duration, Instant};

#[derive(Clone, Copy, Debug)]
pub struct NativeIndexOptions {
    pub index: IndexLimits,
    pub max_read_bytes: u64,
    pub process: ProcessLimits,
    pub open_timeout: Duration,
    pub call_timeout: Duration,
}
pub struct NativeIndex<'a> {
    owner: Owner<'a>,
    summary: IndexSummary,
    limits: IndexLimits,
    length: u64,
    applied: AppliedLimits,
}
impl<'a> NativeIndex<'a> {
    pub fn open(
        bundle: &WorkerBundle,
        source: impl ReadAt + 'a,
        byte_length: u64,
        options: NativeIndexOptions,
        cancelled: &dyn Fn() -> bool,
    ) -> Result<Self, WorkerError> {
        let start = Instant::now();
        options
            .index
            .validate()
            .map_err(|_| WorkerError::Invalid("container index limits"))?;
        if byte_length < 8 || byte_length > options.index.max_input_bytes {
            return Err(WorkerError::Invalid("container index source length"));
        }
        let transport = OwnerOptions {
            max_read_bytes: options.max_read_bytes,
            process: options.process,
            open_timeout: options.open_timeout,
            call_timeout: options.call_timeout,
        };
        let mut owner = Owner::start(
            bundle,
            source,
            byte_length,
            transport,
            None,
            start,
            cancelled,
        )?;
        let reply = owner.call_since(
            Command::OpenIndex {
                profile: PROFILE.into(),
                byte_length,
                limits: options.index,
                process_limits: options.process,
            },
            start,
            options.open_timeout,
            cancelled,
        )?;
        match reply.event {
            Event::Indexed {
                profile,
                summary,
                applied,
            } if profile == PROFILE => {
                summary.validate(options.index, options.max_read_bytes)?;
                validate_applied(applied, options.process)?;
                Ok(Self {
                    owner,
                    summary,
                    limits: options.index,
                    length: byte_length,
                    applied,
                })
            }
            _ => Err(WorkerError::Invalid("unexpected container index response")),
        }
    }
    pub fn summary(&self) -> IndexSummary {
        self.summary
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
    /// Count/byte bounds are enforced on both sides. An invalid local query is
    /// rejected before I/O; a worker failure permanently stops this owner.
    pub fn query(
        &mut self,
        query: IndexQuery,
        cancelled: &dyn Fn() -> bool,
    ) -> Result<IndexPage, WorkerError> {
        query.validate(self.summary)?;
        let reply = self.owner.call(Command::QueryIndex { query }, cancelled)?;
        if let Event::IndexPage { page } = reply.event {
            if let Err(error) = page.validate(query, self.summary, self.limits, self.length) {
                self.owner.stop();
                return Err(error.into());
            }
            Ok(page)
        } else {
            self.owner.stop();
            Err(WorkerError::Invalid("unexpected container index page"))
        }
    }
    pub fn close(self, cancelled: &dyn Fn() -> bool) -> Result<(), WorkerError> {
        self.owner.close(cancelled)
    }
}
