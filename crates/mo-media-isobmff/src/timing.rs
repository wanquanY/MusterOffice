//! Shared limits and errors for container movie/media time mapping.
use mo_time::TimeError;
use std::panic::{AssertUnwindSafe, catch_unwind};

#[derive(Clone, Copy, Debug)]
pub struct TimingLimits {
    pub max_samples: usize,
    pub max_edits: usize,
    pub exact_bits: u64,
}
impl Default for TimingLimits {
    fn default() -> Self {
        Self {
            max_samples: 2_000_000,
            max_edits: 4096,
            exact_bits: 256,
        }
    }
}
#[derive(Debug, thiserror::Error)]
pub enum TimingError {
    #[error("invalid container composition timing: {0}")]
    Invalid(&'static str),
    #[error("unsupported container composition timing: {0}")]
    Unsupported(&'static str),
    #[error("container composition timing budget exceeded")]
    Budget,
    #[error("container composition timing cancelled")]
    Cancelled,
    #[error("container composition callback panicked")]
    CallbackPanic,
    #[error(transparent)]
    Time(#[from] TimeError),
}
pub type Result<T> = std::result::Result<T, TimingError>;
pub(crate) fn active(check: &dyn Fn() -> bool) -> Result<()> {
    if check() {
        Err(TimingError::Cancelled)
    } else {
        Ok(())
    }
}
pub(crate) fn guarded<T>(work: impl FnOnce() -> Result<T>) -> Result<T> {
    match catch_unwind(AssertUnwindSafe(work)) {
        Ok(result) => result,
        Err(payload) => {
            std::mem::forget(payload);
            Err(TimingError::CallbackPanic)
        }
    }
}
