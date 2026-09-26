//! Shared cooperative execution checks for mutations, exports and private I/O.
use crate::{StandardHost, WorkLease, db};
use mo_operation_service::*;
use rusqlite::params;
use std::cell::{Cell, RefCell};

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
        // The immutable request was validated at claim. A checkpoint must not
        // repeatedly decode a large document just to inspect its lease.
        let json: Option<String> = self.connection.query_row(
            "SELECT CASE WHEN length(info)<=65536 THEN info ELSE NULL END FROM jobs WHERE scope=?1 AND principal=?2 AND id=?3",
            params![context.scope.as_str(),context.principal.as_str(),lease.id.as_str()], |r| r.get(0),
        ).map_err(db::error)?;
        let info = db::decode(&json.ok_or_else(db::corrupt)?)?;
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
        Ok(Self {
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
            if now >= self.next_renew.get() {
                self.host.renew(self.context, self.lease, now)?;
                self.next_renew.set(
                    now.checked_add((self.host.limits.lease_ms / 3).max(1))
                        .ok_or_else(db::corrupt)?,
                );
            }
            self.host.execution_guard(self.context, self.lease, now)
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
