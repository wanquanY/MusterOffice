//! Immutable index and lazy shared movie-time projections. No retained I/O port.
use crate::presentation::{TimingError, TimingLimits, VideoTiming};
use crate::{IndexError, IndexLimits, IsoBmffIndex};
use mo_media::{ErrorKind, MediaError, ReadAt};
use std::collections::BTreeMap;

pub struct IndexedMovie {
    raw: IsoBmffIndex,
    limits: IndexLimits,
    timing: BTreeMap<u32, VideoTiming>,
}
impl IndexedMovie {
    pub fn open(
        source: &mut dyn ReadAt,
        length: u64,
        limits: IndexLimits,
        check: &dyn Fn() -> bool,
    ) -> Result<Self, MediaError> {
        Ok(Self {
            raw: IsoBmffIndex::read(source, length, limits, check).map_err(index_error)?,
            limits,
            timing: BTreeMap::new(),
        })
    }
    pub fn raw(&self) -> &IsoBmffIndex {
        &self.raw
    }
    pub fn timing(
        &mut self,
        track: u32,
        check: &dyn Fn() -> bool,
    ) -> Result<&VideoTiming, MediaError> {
        crate::timing::active(check).map_err(timing_error)?;
        if let std::collections::btree_map::Entry::Vacant(entry) = self.timing.entry(track) {
            let raw = self
                .raw
                .tracks
                .get(track as usize)
                .ok_or_else(|| MediaError::new(ErrorKind::Invalid, "track index"))?;
            let timing = VideoTiming::build(
                raw,
                self.raw.movie_timescale,
                TimingLimits {
                    max_samples: self.limits.max_samples,
                    max_edits: self.limits.max_edits,
                    exact_bits: 256,
                },
                check,
            )
            .map_err(timing_error)?;
            entry.insert(timing);
        }
        Ok(&self.timing[&track])
    }
}
pub fn timing_error(error: TimingError) -> MediaError {
    let kind = match &error {
        TimingError::Invalid(_) => ErrorKind::Invalid,
        TimingError::Unsupported(_) => ErrorKind::Unsupported,
        TimingError::Budget | TimingError::Time(_) => ErrorKind::Limit,
        TimingError::Cancelled => ErrorKind::Cancelled,
        TimingError::CallbackPanic => ErrorKind::CallbackPanic,
    };
    MediaError::new(kind, error.to_string())
}
pub fn index_error(error: IndexError) -> MediaError {
    let kind = match &error {
        IndexError::Io(_) => ErrorKind::Io,
        IndexError::Invalid(_) => ErrorKind::Invalid,
        IndexError::Unsupported(_) => ErrorKind::Unsupported,
        IndexError::Budget => ErrorKind::Limit,
        IndexError::Cancelled => ErrorKind::Cancelled,
        IndexError::CallbackPanic => ErrorKind::CallbackPanic,
    };
    MediaError::new(kind, error.to_string())
}
