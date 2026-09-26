//! Durable scheduling indexes and bounded work selection. Jobs remain the only
//! source of truth; indexes are derived from their existing persisted receipts.
use crate::{StandardHost, WorkItem, db, jobs::claim_record};
use mo_operation_service::*;
use rusqlite::{Connection, OptionalExtension, TransactionBehavior, params};

const NEXT_JOB: &str = "SELECT id FROM jobs WHERE scope=?1 AND principal=?2
    AND json_extract(info,'$.state')='queued' AND json_extract(info,'$.executorDigest')=?3
    AND ((operation='presentations.create' AND ?4) OR (operation='presentations.apply' AND ?5) OR (operation='presentations.export' AND ?6) OR (operation='presentations.import' AND ?7))
    ORDER BY CAST(json_extract(info,'$.createdAt') AS INTEGER),id LIMIT 1";
const EXPIRED_JOBS: &str = "SELECT id FROM jobs WHERE scope=?1 AND principal=?2
    AND json_extract(info,'$.state')='running' AND CAST(json_extract(info,'$.leaseUntil') AS INTEGER)<=?3
    AND ((operation='presentations.create' AND ?4) OR (operation='presentations.apply' AND ?5) OR (operation='presentations.export' AND ?6) OR (operation='presentations.import' AND ?7))
    ORDER BY CAST(json_extract(info,'$.leaseUntil') AS INTEGER),id LIMIT ?8";

pub(crate) fn migrate(connection: &Connection) -> Result<(), Failure> {
    connection.execute_batch(
        "CREATE INDEX jobs_queued ON jobs(scope,principal,json_extract(info,'$.executorDigest'),CAST(json_extract(info,'$.createdAt') AS INTEGER),id) WHERE json_extract(info,'$.state')='queued';
         CREATE INDEX jobs_expiring ON jobs(scope,principal,CAST(json_extract(info,'$.leaseUntil') AS INTEGER),id) WHERE json_extract(info,'$.state')='running';"
    ).map_err(db::error)
}

fn allowed(context: &CallContext) -> [bool; 4] {
    [
        ServiceOperation::Create,
        ServiceOperation::Apply,
        ServiceOperation::Export,
        ServiceOperation::Import,
    ]
    .map(|operation| operation.authorize(context).is_ok())
}

impl StandardHost {
    /// Claims at most one authorized job for this exact principal, scope and
    /// pinned executor. Selection and lease publication share the write lock.
    /// Jobs pinned to a different executor stay available to that executor.
    pub fn claim_next(
        &mut self,
        context: &CallContext,
        now: UnixMillis,
    ) -> Result<Option<WorkItem>, Failure> {
        let [create, apply, export, import] = allowed(context);
        let tx = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(db::error)?;
        let id: Option<String> = tx
            .query_row(
                NEXT_JOB,
                params![
                    context.scope.as_str(),
                    context.principal.as_str(),
                    self.executor.as_str(),
                    create,
                    apply,
                    export,
                    import
                ],
                |row| row.get(0),
            )
            .optional()
            .map_err(db::error)?;
        let claimed = id
            .map(|id| {
                let id = JobId::new(id).map_err(|_| db::corrupt())?;
                claim_record(&tx, context, &id, &self.executor, self.limits.lease_ms, now)
            })
            .transpose()?
            .flatten();
        tx.commit().map_err(db::error)?;
        claimed.map(|job| self.load_work(context, job)).transpose()
    }

    pub fn run_next(
        &mut self,
        context: &CallContext,
        now: UnixMillis,
        clock: &dyn Fn() -> UnixMillis,
        check: &dyn Fn() -> bool,
    ) -> Result<Option<JobInfo>, Failure> {
        self.claim_next(context, now)?
            .map(|work| self.execute_work(context, work, clock, check))
            .transpose()
    }

    /// Expire at most `limit` abandoned leases, across executor versions but
    /// only within the injected owner's authority. The existing terminal path
    /// also reclaims private output/reservations in this same transaction.
    pub fn recover_expired(
        &mut self,
        context: &CallContext,
        now: UnixMillis,
        limit: u32,
    ) -> Result<u32, Failure> {
        if !(1..=256).contains(&limit) {
            return Err(Failure::new(
                FailureCode::InputInvalid,
                "recovery batch must be 1..=256",
            ));
        }
        let [create, apply, export, import] = allowed(context);
        let tx = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(db::error)?;
        let ids = {
            let mut query = tx.prepare(EXPIRED_JOBS).map_err(db::error)?;
            query
                .query_map(
                    params![
                        context.scope.as_str(),
                        context.principal.as_str(),
                        now.get(),
                        create,
                        apply,
                        export,
                        import,
                        limit
                    ],
                    |r| r.get::<_, String>(0),
                )
                .map_err(db::error)?
                .collect::<Result<Vec<_>, _>>()
                .map_err(db::error)?
        };
        let mut count = 0;
        for id in ids {
            let id = JobId::new(id).map_err(|_| db::corrupt())?;
            let mut info = db::job_info(&tx, context, &id)?;
            db::authorize_job(context, &info)?;
            if db::expire(&mut info, now)? {
                db::save(&tx, context, &info)?;
                count += 1;
            }
        }
        tx.commit().map_err(db::error)?;
        Ok(count)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn actual_queue_queries_use_bounded_index_ranges_without_sorting() {
        let mut connection = Connection::open_in_memory().unwrap();
        db::initialize(&mut connection).unwrap();
        for (sql, index) in [(NEXT_JOB, "jobs_queued"), (EXPIRED_JOBS, "jobs_expiring")] {
            let mut query = connection
                .prepare(&format!("EXPLAIN QUERY PLAN {sql}"))
                .unwrap();
            let values = (0..query.parameter_count()).map(|_| rusqlite::types::Value::Integer(1));
            let steps = query
                .query_map(rusqlite::params_from_iter(values), |r| {
                    r.get::<_, String>(3)
                })
                .unwrap()
                .collect::<Result<Vec<_>, _>>()
                .unwrap();
            assert!(
                steps
                    .iter()
                    .any(|s| s.contains(&format!("SEARCH jobs USING INDEX {index}"))),
                "{steps:?}"
            );
            assert!(
                steps
                    .iter()
                    .all(|s| !s.contains("TEMP B-TREE") && !s.starts_with("SCAN jobs")),
                "{steps:?}"
            );
        }
    }
}
