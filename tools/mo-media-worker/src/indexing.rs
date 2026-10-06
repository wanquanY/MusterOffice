use crate::wire::Wire;
use mo_media::{ErrorKind, MediaError};
use mo_media_isobmff::presentation::{TimingError, TimingLimits, VideoTiming};
use mo_media_isobmff::{IndexError, IndexLimits, IsoBmffIndex};
use std::{cell::RefCell, rc::Rc};
pub struct Indexed {
    pub raw: IsoBmffIndex,
    limits: IndexLimits,
    timing: std::collections::BTreeMap<u32, VideoTiming>,
}
impl Indexed {
    pub fn timing(&mut self, track: u32) -> Result<&VideoTiming, MediaError> {
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
                &|| false,
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
pub fn open(
    io: Rc<RefCell<Wire>>,
    length: u64,
    limits: IndexLimits,
) -> Result<Indexed, MediaError> {
    let mut input =
        move |offset, destination: &mut [u8]| io.borrow_mut().read_source(offset, destination);
    let raw = IsoBmffIndex::read(&mut input, length, limits, &|| false).map_err(|error| {
        let kind = match &error {
            IndexError::Io(_) => ErrorKind::Io,
            IndexError::Invalid(_) => ErrorKind::Invalid,
            IndexError::Unsupported(_) => ErrorKind::Unsupported,
            IndexError::Budget => ErrorKind::Limit,
            IndexError::Cancelled => ErrorKind::Cancelled,
            IndexError::CallbackPanic => ErrorKind::CallbackPanic,
        };
        MediaError::new(kind, error.to_string())
    })?;
    Ok(Indexed {
        raw,
        limits,
        timing: std::collections::BTreeMap::new(),
    })
}
