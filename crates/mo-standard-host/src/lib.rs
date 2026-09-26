//! Durable standard host. Embedded products use mo-operation-service computation
//! with their own authoritative store, rather than nesting this owner.
mod assets;
mod db;
mod execution;
mod exports;
mod jobs;
mod operation_host;
mod queue;
mod runtime;
pub use runtime::{HostClock, NativeRuntime, NativeSession, RuntimeOptions, StandardHostConfig};
mod results;
pub use assets::{AssetLimits, AssetReader, BoundResources};
pub use mo_native_io::{FileSpool, SealedFile};
pub use results::{ResultLimits, ResultSpec, SqlResultReader, SqlResultSink};
#[cfg(test)]
mod tests;
pub use jobs::{WorkItem, WorkLease};
use mo_common::{Digest, DocumentId};
use mo_operation_service::*;
use mo_presentation_edit::SnapshotRecord;
use rusqlite::Connection;
use std::{path::Path, time::Duration};

#[derive(Debug, Clone, Copy)]
pub struct HostLimits {
    pub max_jobs_per_principal: u32,
    pub max_documents_per_scope: u32,
    pub lease_ms: i64,
    pub assets: AssetLimits,
    pub results: ResultLimits,
}
impl Default for HostLimits {
    fn default() -> Self {
        Self {
            max_jobs_per_principal: 10_000,
            max_documents_per_scope: 10_000,
            lease_ms: 30_000,
            assets: AssetLimits::default(),
            results: ResultLimits::default(),
        }
    }
}
pub struct StandardHost {
    connection: Connection,
    executor: Digest,
    limits: HostLimits,
    renderer: Option<Box<dyn mo_presentation_delivery::PreviewRenderer + Send>>,
}
impl StandardHost {
    /// Path is trusted operator configuration, never an operation argument. The
    /// parent directory and database must be protected by the host installation.
    pub fn open(
        path: impl AsRef<Path>,
        executor: Digest,
        limits: HostLimits,
    ) -> Result<Self, Failure> {
        limits.assets.validate()?;
        limits.results.validate()?;
        if limits.max_jobs_per_principal == 0
            || limits.max_documents_per_scope == 0
            || !(1..=3_600_000).contains(&limits.lease_ms)
        {
            return Err(Failure::new(
                FailureCode::InputInvalid,
                "invalid host limits",
            ));
        }
        let mut connection = Connection::open(path).map_err(db::error)?;
        connection
            .busy_timeout(Duration::from_secs(5))
            .map_err(db::error)?;
        connection.execute_batch("PRAGMA foreign_keys=ON; PRAGMA trusted_schema=OFF; PRAGMA synchronous=FULL; PRAGMA fullfsync=ON;").map_err(db::error)?;
        db::initialize(&mut connection)?;
        connection
            .execute_batch("PRAGMA journal_mode=WAL;")
            .map_err(db::error)?;
        Ok(Self {
            connection,
            executor,
            limits,
            renderer: None,
        })
    }
    pub fn read_document(
        &self,
        context: &CallContext,
        id: &DocumentId,
        revision: Option<&Digest>,
    ) -> Result<SnapshotRecord, Failure> {
        context.require(Permission::ReadDocument)?;
        let revision = match revision {
            Some(r) => r.clone(),
            None => db::head(&self.connection, &context.scope, id)?.ok_or_else(db::not_found)?,
        };
        db::snapshot(&self.connection, &context.scope, id, &revision)?.ok_or_else(db::not_found)
    }
    /// Bounded synchronous execution helper for already admitted work. Explicit
    /// job mode only queues; an operator worker calls run_job independently.
    pub fn run_job(
        &mut self,
        context: &CallContext,
        id: &JobId,
        now: UnixMillis,
        finished_at: &dyn Fn() -> UnixMillis,
        check: &dyn Fn() -> bool,
    ) -> Result<JobInfo, Failure> {
        let Some(work) = self.claim(context, id, now)? else {
            // Operation execution/replay requires its mutation permission;
            // the separate read-job permission controls the query endpoint.
            let stored = db::job(&self.connection, context, id)?;
            context.authorize(&stored.request)?;
            return Ok(stored.info);
        };
        self.execute_work(context, work, finished_at, check)
    }
    pub(crate) fn execute_work(
        &mut self,
        context: &CallContext,
        work: WorkItem,
        finished_at: &dyn Fn() -> UnixMillis,
        check: &dyn Fn() -> bool,
    ) -> Result<JobInfo, Failure> {
        if matches!(work.request.action, DocumentAction::Export { .. }) {
            return self.run_export_work(context, work, finished_at, check);
        }
        let candidate = (|| {
            let monitor =
                execution::ExecutionCheck::new(self, context, &work.lease, finished_at, check)?;
            let candidate = compute_mutation(&work.request, work.snapshot, &|| monitor.cancelled());
            monitor.finish(candidate)
        })();
        self.finish(context, &work.lease, candidate, finished_at())
    }
    pub fn dispatch(
        &mut self,
        context: &CallContext,
        request: HostRequest,
        clock: &dyn Fn() -> UnixMillis,
        check: &dyn Fn() -> bool,
    ) -> HostResponse {
        dispatch_host(self, context, request, clock, check)
    }

    pub fn dispatch_json(
        &mut self,
        context: &CallContext,
        input: &str,
        clock: &dyn Fn() -> UnixMillis,
        check: &dyn Fn() -> bool,
    ) -> String {
        dispatch_host_json(self, context, input, clock, check)
    }
}
