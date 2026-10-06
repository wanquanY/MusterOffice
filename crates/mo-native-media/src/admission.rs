use crate::WorkerError;
use mo_native_limits::{AppliedLimits, ProcessLimits};
use mo_native_worker::SessionError;
use std::{
    panic::{AssertUnwindSafe, catch_unwind},
    time::{Duration, Instant},
};

pub(crate) fn boundary(
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
pub(crate) fn validate_applied(
    applied: AppliedLimits,
    request: ProcessLimits,
) -> Result<(), WorkerError> {
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

pub(crate) fn guarded_boundary(
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
