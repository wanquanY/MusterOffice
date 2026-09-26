//! Shared cooperative execution checks for mutations, exports and private I/O.
use crate::{StandardHost, WorkLease, db};
use mo_operation_service::*;
use std::cell::{Cell, RefCell};
use std::time::{Duration, Instant};

pub(crate) fn guard_job(job: &JobInfo, lease: &WorkLease, now: UnixMillis) -> Result<(), Failure> {
    if job.id != lease.id
        || job.fence != lease.fence
        || job.executor_digest != lease.executor
        || job.request_digest != lease.request_digest
    {
        return Err(Failure::new(
            FailureCode::StaleExecution,
            "execution binding differs",
        ));
    }
    if job.cancel_requested || job.state == JobState::Cancelled {
        return Err(Failure::new(FailureCode::Cancelled, "execution cancelled"));
    }
    if job.state != JobState::Running || job.result.is_some() {
        return Err(Failure::new(
            FailureCode::StaleExecution,
            "execution is not active",
        ));
    }
    db::check_time(job, now)?;
    if now >= job.lease_until.ok_or_else(db::corrupt)? {
        return Err(Failure::new(
            FailureCode::ExecutionInterrupted,
            "execution lease expired",
        ));
    }
    Ok(())
}
impl StandardHost {
    fn execution_guard(
        &self,
        context: &CallContext,
        lease: &WorkLease,
        now: UnixMillis,
    ) -> Result<JobInfo, Failure> {
        self.check_lease_owner(context, lease)?;
        let info = db::job_info(&self.connection, context, &lease.id)?;
        guard_job(&info, lease, now)?;
        Ok(info)
    }
}

pub(crate) struct ExecutionCheck<'a> {
    host: &'a StandardHost,
    context: &'a CallContext,
    lease: &'a WorkLease,
    clock: &'a dyn Fn() -> UnixMillis,
    external: &'a dyn Fn() -> bool,
    next_renew: Cell<UnixMillis>,
    next_poll: Cell<UnixMillis>,
    poll_deadline: Cell<Instant>,
    poll_interval: Duration,
    last_now: Cell<UnixMillis>,
    error: RefCell<Option<Failure>>,
}
impl<'a> ExecutionCheck<'a> {
    pub fn new(
        host: &'a StandardHost,
        context: &'a CallContext,
        lease: &'a WorkLease,
        clock: &'a dyn Fn() -> UnixMillis,
        external: &'a dyn Fn() -> bool,
    ) -> Result<Self, Failure> {
        let now = clock();
        let info = host.execution_guard(context, lease, now)?;
        let poll_interval = Duration::from_millis((host.limits.lease_ms / 10).clamp(1, 100) as u64);
        Ok(Self {
            next_poll: Cell::new(now),
            poll_deadline: Cell::new(Instant::now()),
            poll_interval,
            last_now: Cell::new(now),
            host,
            context,
            lease,
            clock,
            external,
            next_renew: Cell::new(
                info.updated_at
                    .checked_add((host.limits.lease_ms / 3).max(1))
                    .ok_or_else(db::corrupt)?,
            ),
            error: RefCell::new(None),
        })
    }
    pub fn cancelled(&self) -> bool {
        if self.error.borrow().is_some() || (self.external)() {
            return true;
        }
        let now = (self.clock)();
        let result = (|| {
            if now < self.last_now.replace(now) {
                return Err(Failure::new(
                    FailureCode::InputInvalid,
                    "host clock moved backwards",
                ));
            }
            let instant = Instant::now();
            if now < self.next_poll.get()
                && now < self.next_renew.get()
                && instant < self.poll_deadline.get()
            {
                return Ok(());
            }
            let info = if now >= self.next_renew.get() {
                self.host.renew(self.context, self.lease, now)?
            } else {
                self.host.execution_guard(self.context, self.lease, now)?
            };
            guard_job(&info, self.lease, now)?;
            self.next_renew.set(
                info.updated_at
                    .checked_add((self.host.limits.lease_ms / 3).max(1))
                    .ok_or_else(db::corrupt)?,
            );
            self.next_poll.set(
                now.checked_add(self.poll_interval.as_millis() as i64)
                    .ok_or_else(db::corrupt)?,
            );
            self.poll_deadline.set(instant + self.poll_interval);
            Ok(())
        })();
        if let Err(error) = result {
            *self.error.borrow_mut() = Some(error);
            true
        } else {
            false
        }
    }
    pub fn finish<T>(self, result: Result<T, Failure>) -> Result<T, Failure> {
        match self.error.into_inner() {
            Some(error) => Err(error),
            None => result,
        }
    }
}

#[cfg(test)]
mod tests;
