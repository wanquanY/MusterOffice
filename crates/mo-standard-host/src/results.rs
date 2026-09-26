//! Private job-owned output storage. A sealed candidate is not a published
//! asset or successful job result. Publication belongs to the job transaction.
mod published;
mod reader;
mod sink;
mod storage;
pub(crate) use published::{
    PublishedKey, commit_publication, migrate_published, prepare_publication, published_asset,
    published_chunk,
};
pub use reader::SqlResultReader;
pub use sink::SqlResultSink;
pub(crate) use storage::{job_changed, migrate, reap};

use crate::{StandardHost, WorkLease, assets, db, execution::guard_job};
use mo_common::{Digest, RequestId};
use mo_operation_service::*;
use rusqlite::{Transaction, TransactionBehavior, params};
use serde::{Deserialize, Serialize};
use std::cell::Cell;
use std::rc::Rc;

#[derive(Debug, Clone, Copy)]
pub struct ResultLimits {
    pub max_outputs_per_job: u32,
    pub max_outputs_per_scope: u32,
}
impl Default for ResultLimits {
    fn default() -> Self {
        Self {
            max_outputs_per_job: 1024,
            max_outputs_per_scope: 10_000,
        }
    }
}
impl ResultLimits {
    pub(crate) fn validate(&self) -> Result<(), Failure> {
        if self.max_outputs_per_job == 0 || self.max_outputs_per_scope < self.max_outputs_per_job {
            return Err(Failure::new(
                FailureCode::InputInvalid,
                "invalid result limits",
            ));
        }
        Ok(())
    }
}

/// Trusted worker declaration, not a public tool or filesystem argument.
/// `max_bytes` reserves capacity before computation; the actual digest and
/// length are measured only after the worker has finished writing.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ResultSpec {
    pub name: RequestId,
    pub media_type: String,
    pub max_bytes: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
enum State {
    Writing,
    Sealing,
    Sealed,
    Published,
    Discarded,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Record {
    spec: ResultSpec,
    state: State,
    received: u64,
    digest: Option<Digest>,
    updated_at: UnixMillis,
}

#[derive(Clone)]
struct Owner<'a> {
    host: &'a StandardHost,
    context: CallContext,
    lease: WorkLease,
    name: RequestId,
    clock: &'a dyn Fn() -> UnixMillis,
    last_seen: Rc<Cell<UnixMillis>>,
}
impl Owner<'_> {
    fn now(&self) -> Result<UnixMillis, Failure> {
        let now = (self.clock)();
        if now < self.last_seen.get() {
            return Err(Failure::new(
                FailureCode::InputInvalid,
                "result clock moved backwards",
            ));
        }
        self.last_seen.set(now);
        Ok(now)
    }
}

impl StandardHost {
    /// Exclusive writer for a single named output in this execution. Names
    /// cannot be reused, including after abandonment. No public asset is made.
    pub fn create_result<'a>(
        &'a self,
        context: &CallContext,
        lease: &WorkLease,
        spec: ResultSpec,
        clock: &'a dyn Fn() -> UnixMillis,
    ) -> Result<SqlResultSink<'a>, Failure> {
        self.check_lease_owner(context, lease)?;
        db::validate_media_type(&spec.media_type)?;
        if spec.max_bytes > self.limits.assets.max_asset_bytes {
            return Err(Failure::new(
                FailureCode::LimitExceeded,
                "result byte limit",
            ));
        }
        let now = clock();
        // Interior-borrow transaction permits authorized input readers and the
        // output writer to share this connection. No callback runs under it.
        let tx = Transaction::new_unchecked(&self.connection, TransactionBehavior::Immediate)
            .map_err(db::error)?;
        let job = db::job_info(&tx, context, &lease.id)?;
        db::authorize_job(context, &job)?;
        guard_job(&job, lease, now)?;
        assets::reap(&tx, &context.scope, now)?;
        storage::reap(&tx, &context.scope, now)?;
        let exists: bool = tx.query_row(
            "SELECT EXISTS(SELECT 1 FROM result_spools WHERE scope=?1 AND principal=?2 AND job_id=?3 AND fence=?4 AND name=?5)",
            params![context.scope.as_str(), context.principal.as_str(), lease.id.as_str(), lease.fence.get(), spec.name.as_str()], |r| r.get(0),
        ).map_err(db::error)?;
        if exists {
            return Err(Failure::new(
                FailureCode::ResourceConflict,
                "result name already reserved",
            ));
        }
        let (scope_count, job_count): (u32, u32) = tx.query_row(
            "SELECT count(*),coalesce(sum(principal=?2 AND job_id=?3),0) FROM result_spools WHERE scope=?1",
            params![context.scope.as_str(), context.principal.as_str(), lease.id.as_str()],
            |r| Ok((r.get(0)?, r.get(1)?)),
        ).map_err(db::error)?;
        if scope_count >= self.limits.results.max_outputs_per_scope
            || job_count >= self.limits.results.max_outputs_per_job
            || db::scope_reserved(&tx, &context.scope)?
                .checked_add(spec.max_bytes)
                .is_none_or(|n| n > self.limits.assets.max_scope_bytes)
        {
            tx.commit().map_err(db::error)?;
            return Err(Failure::new(
                FailureCode::LimitExceeded,
                "result scope reservation or receipt quota",
            ));
        }
        let record = Record {
            spec,
            state: State::Writing,
            received: 0,
            digest: None,
            updated_at: now,
        };
        tx.execute(
            "INSERT INTO result_spools(scope,principal,job_id,fence,name,request_digest,executor_digest,info,reserved_bytes,expires_at) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10)",
            params![context.scope.as_str(), context.principal.as_str(), lease.id.as_str(), lease.fence.get(), record.spec.name.as_str(), lease.request_digest.as_str(), lease.executor.as_str(), db::encode(&record)?, record.spec.max_bytes as i64, job.lease_until.ok_or_else(db::corrupt)?.get()],
        ).map_err(db::error)?;
        tx.commit().map_err(db::error)?;
        Ok(SqlResultSink::new(
            Owner {
                host: self,
                context: context.clone(),
                lease: lease.clone(),
                name: record.spec.name.clone(),
                clock,
                last_seen: Rc::new(Cell::new(now)),
            },
            record,
        ))
    }

    /// Reopens the same sealed candidate, still restricted to its live worker.
    /// Public ReadAssets cannot resolve these names. This is not package proof.
    pub fn open_result<'a>(
        &'a self,
        context: &CallContext,
        lease: &WorkLease,
        name: &RequestId,
        clock: &'a dyn Fn() -> UnixMillis,
    ) -> Result<SqlResultReader<'a>, Failure> {
        self.check_lease_owner(context, lease)?;
        let now = clock();
        let tx = Transaction::new_unchecked(&self.connection, TransactionBehavior::Deferred)
            .map_err(db::error)?;
        let job = db::job_info(&tx, context, &lease.id)?;
        db::authorize_job(context, &job)?;
        guard_job(&job, lease, now)?;
        let owner = Owner {
            host: self,
            context: context.clone(),
            lease: lease.clone(),
            name: name.clone(),
            clock,
            last_seen: Rc::new(Cell::new(now)),
        };
        let record = storage::load(&tx, &owner)?;
        storage::check_record(&record, State::Sealed, now)?;
        tx.commit().map_err(db::error)?;
        Ok(SqlResultReader::new(owner, record))
    }
}
