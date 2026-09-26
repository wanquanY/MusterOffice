#[path = "scheduling/contention.rs"]
mod contention;
#[path = "scheduling/crash.rs"]
mod crash;
mod support;
use mo_common::{Digest, DocumentId, RequestId};
use mo_operation_service::*;
use mo_standard_host::{
    HostLimits, NativeRuntime, RuntimeOptions, StandardHost, StandardHostConfig,
};
use rusqlite::Connection;
use std::{
    path::PathBuf,
    sync::{Arc, Barrier},
    thread,
    time::{Duration, Instant},
};

struct Database(PathBuf);
impl Database {
    fn new() -> Self {
        Self(support::temporary_directory("mo-scheduling"))
    }
    fn path(&self) -> PathBuf {
        self.0.join("host.sqlite")
    }
    fn config(&self, executor: u8) -> StandardHostConfig {
        StandardHostConfig::new(
            self.path(),
            Digest::from_sha256([executor; 32]),
            HostLimits {
                lease_ms: 100,
                ..HostLimits::default()
            },
        )
    }
    fn host(&self) -> StandardHost {
        self.config(1).connect().unwrap()
    }
    fn sql(&self) -> Connection {
        Connection::open(self.path()).unwrap()
    }
}
impl Drop for Database {
    fn drop(&mut self) {
        std::fs::remove_dir_all(&self.0).unwrap();
    }
}
fn time(n: i64) -> UnixMillis {
    UnixMillis::new(n).unwrap()
}
fn context() -> CallContext {
    CallContext {
        principal: PrincipalId::new("agent").unwrap(),
        scope: ScopeId::new("scope").unwrap(),
        permissions: [
            Permission::Create,
            Permission::Edit,
            Permission::ReadJob,
            Permission::CancelJob,
            Permission::ReadDocument,
        ]
        .into_iter()
        .collect(),
    }
}
fn request(name: &str) -> OperationRequest {
    let value: serde_json::Value = serde_json::from_str(include_str!(
        "../../../fixtures/presentations/playback/page.json"
    ))
    .unwrap();
    let mut document: mo_presentation_model::Document =
        serde_json::from_value(value["page"]["document"].clone()).unwrap();
    document.id = DocumentId::new(name).unwrap();
    OperationRequest {
        contract_version: ContractVersion::V1,
        request_id: RequestId::new(name).unwrap(),
        profile_id: OperationProfile::AuthorModel,
        output_mode: OutputMode::Job,
        action: DocumentAction::Create {
            document: Box::new(document),
        },
    }
}
fn options() -> RuntimeOptions {
    RuntimeOptions {
        idle_poll: Duration::from_millis(10),
        ..RuntimeOptions::default()
    }
}
fn wait(mut check: impl FnMut() -> bool) {
    let deadline = Instant::now() + Duration::from_secs(10);
    while !check() {
        assert!(Instant::now() < deadline, "scheduler did not make progress");
        thread::sleep(Duration::from_millis(5));
    }
}
fn job(response: HostResponse) -> JobInfo {
    match response {
        HostResponse::Accepted { job, .. }
        | HostResponse::Succeeded {
            result: HostResult::Job { job },
        }
        | HostResponse::Failed { job: Some(job), .. } => *job,
        other => panic!("job response required: {other:?}"),
    }
}

#[test]
fn queue_claims_are_atomic_across_connections_and_preserve_fences() {
    let db = Database::new();
    let mut host = db.host();
    let ids = (0..12)
        .map(|n| {
            host.submit(&context(), request(&format!("doc-{n}")), time(10 + n))
                .unwrap()
                .id
        })
        .collect::<Vec<_>>();
    let barrier = Arc::new(Barrier::new(4));
    let threads = (0..4)
        .map(|_| {
            let config = db.config(1);
            let barrier = barrier.clone();
            thread::spawn(move || {
                let mut host = config.connect().unwrap();
                barrier.wait();
                let mut claims = Vec::new();
                while let Some(work) = host.claim_next(&context(), time(50)).unwrap() {
                    assert_eq!(work.lease.fence().get(), 1);
                    claims.push(work.lease.job_id().clone());
                }
                claims
            })
        })
        .collect::<Vec<_>>();
    let mut claimed = threads
        .into_iter()
        .flat_map(|t| t.join().unwrap())
        .collect::<Vec<_>>();
    claimed.sort();
    assert_eq!(claimed.len(), ids.len());
    claimed.dedup();
    assert_eq!(claimed.len(), ids.len());
    for id in ids {
        assert!(claimed.contains(&id));
        assert_eq!(
            host.get_job(&context(), &id, time(50)).unwrap().state,
            JobState::Running
        );
    }
}

#[test]
fn queue_filters_authority_executor_and_orders_numeric_times() {
    let db = Database::new();
    let mut host = db.host();
    let older = host.submit(&context(), request("older"), time(9)).unwrap();
    let newer = host.submit(&context(), request("newer"), time(10)).unwrap();
    let foreign = db
        .config(2)
        .connect()
        .unwrap()
        .submit(&context(), request("foreign"), time(1))
        .unwrap();
    let mut other = context();
    other.principal = PrincipalId::new("different").unwrap();
    let unrelated = host.submit(&other, request("other"), time(1)).unwrap();
    let mut apply = request("edit");
    apply.action = DocumentAction::Apply {
        document_id: DocumentId::new("older").unwrap(),
        base_revision: Digest::from_sha256([3; 32]),
        operations: vec![],
    };
    let edit = host.submit(&context(), apply, time(2)).unwrap();
    let mut create_only = context();
    create_only.permissions = [Permission::Create].into_iter().collect();
    assert_eq!(
        host.claim_next(&create_only, time(10))
            .unwrap()
            .unwrap()
            .lease
            .job_id(),
        &older.id
    );
    assert_eq!(
        host.claim_next(&create_only, time(10))
            .unwrap()
            .unwrap()
            .lease
            .job_id(),
        &newer.id
    );
    assert!(host.claim_next(&create_only, time(10)).unwrap().is_none());
    assert_eq!(
        host.get_job(&context(), &edit.id, time(10)).unwrap().state,
        JobState::Queued
    );
    assert_eq!(
        host.claim(&context(), &foreign.id, time(10))
            .err()
            .unwrap()
            .code,
        FailureCode::ExecutorMismatch
    );
    assert_eq!(
        host.get_job(&other, &unrelated.id, time(10)).unwrap().state,
        JobState::Queued
    );
    let mut no_rights = context();
    no_rights.permissions.clear();
    assert!(host.claim_next(&no_rights, time(10)).unwrap().is_none());
}

#[test]
fn bounded_recovery_finishes_expired_cancelled_and_old_executor_work() {
    let db = Database::new();
    let mut old = db.config(2).connect().unwrap();
    let cancelled = old
        .submit(&context(), request("cancelled"), time(9))
        .unwrap();
    let expired = old
        .submit(&context(), request("expired"), time(10))
        .unwrap();
    old.claim(&context(), &cancelled.id, time(9))
        .unwrap()
        .unwrap();
    old.claim(&context(), &expired.id, time(10))
        .unwrap()
        .unwrap();
    old.cancel_job(&context(), &cancelled.id, time(20)).unwrap();
    let mut host = db.host();
    assert_eq!(host.recover_expired(&context(), time(108), 1).unwrap(), 0);
    assert_eq!(host.recover_expired(&context(), time(109), 1).unwrap(), 1);
    assert_eq!(
        host.get_job(&context(), &cancelled.id, time(109))
            .unwrap()
            .state,
        JobState::Cancelled
    );
    assert_eq!(
        host.get_job(&context(), &expired.id, time(109))
            .unwrap()
            .state,
        JobState::Running
    );
    assert_eq!(host.recover_expired(&context(), time(110), 1).unwrap(), 1);
    let recovered = host.get_job(&context(), &expired.id, time(110)).unwrap();
    let Some(TerminalResult::Failed { error }) = recovered.result else {
        panic!("failed receipt required")
    };
    assert_eq!(error.code, FailureCode::ExecutionInterrupted);
    assert_eq!(host.recover_expired(&context(), time(111), 1).unwrap(), 0);
    assert_eq!(
        host.recover_expired(&context(), time(111), 257)
            .unwrap_err()
            .code,
        FailureCode::InputInvalid
    );
}

#[test]
fn runtime_executes_preexisting_and_disconnected_jobs_without_another_owner() {
    let db = Database::new();
    let mut host = db.host();
    let prior = host
        .submit(&context(), request("before-start"), time(10))
        .unwrap();
    let runtime =
        NativeRuntime::start(db.config(1), context(), options(), Arc::new(|| time(50))).unwrap();
    let mut control = runtime.connect().unwrap();
    assert_eq!(
        control.capabilities().queued_execution,
        JobExecution::HostScheduled
    );
    let submitted = job(control.dispatch(HostRequest::Submit {
        request: Box::new(request("new-job")),
    }));
    drop(control);
    wait(|| {
        host.get_job(&context(), &prior.id, time(50)).unwrap().state == JobState::Succeeded
            && host
                .get_job(&context(), &submitted.id, time(50))
                .unwrap()
                .state
                == JobState::Succeeded
    });
    runtime.shutdown().unwrap();
    assert_eq!(
        db.sql()
            .query_row("SELECT count(*) FROM jobs", [], |r| r.get::<_, i64>(0))
            .unwrap(),
        2
    );
    assert_eq!(
        db.sql()
            .query_row("SELECT count(*) FROM revisions", [], |r| r.get::<_, i64>(0))
            .unwrap(),
        2
    );
}

#[test]
fn sync_wait_uses_same_workers_and_execution_authority_without_read_job() {
    let db = Database::new();
    let mut ctx = context();
    ctx.permissions = [Permission::Create].into_iter().collect();
    let runtime =
        NativeRuntime::start(db.config(1), ctx, options(), Arc::new(|| time(50))).unwrap();
    let mut control = runtime.connect().unwrap();
    let mut query = request("sync");
    query.output_mode = OutputMode::Sync;
    let completed = job(control.dispatch(HostRequest::Submit {
        request: Box::new(query.clone()),
    }));
    assert_eq!(completed.state, JobState::Succeeded);
    query.output_mode = OutputMode::Job;
    let replay = job(control.dispatch(HostRequest::Submit {
        request: Box::new(query),
    }));
    assert_eq!(
        serde_json::to_value(replay).unwrap(),
        serde_json::to_value(&completed).unwrap()
    );
    let HostResponse::Failed { error, .. } = control.dispatch(HostRequest::GetJob {
        job_id: completed.id,
    }) else {
        panic!("separate query grant required")
    };
    assert_eq!(error.code, FailureCode::NotAuthorized);
    drop(control);
    runtime.shutdown().unwrap();
}

#[test]
fn failed_worker_stops_new_admission_but_keeps_control_and_durable_data() {
    let db = Database::new();
    let mut host = db.host();
    let prior = host
        .submit(&context(), request("existing"), time(10))
        .unwrap();
    let runtime = NativeRuntime::start(
        db.config(1),
        context(),
        options(),
        Arc::new(|| {
            assert!(
                !thread::current()
                    .name()
                    .is_some_and(|n| n.starts_with("mo-executor-")),
                "injected worker clock failure"
            );
            time(50)
        }),
    )
    .unwrap();
    wait(|| runtime.health().is_err());
    let mut control = runtime.connect().unwrap();
    assert!(
        control
            .capabilities()
            .operations
            .iter()
            .filter(|o| o.profile_id.is_some())
            .all(|o| !o.available
                && o.unavailable_reason == Some(UnavailableReason::ExecutionUnavailable))
    );
    let HostResponse::Failed { error, .. } = control.dispatch(HostRequest::Submit {
        request: Box::new(request("denied")),
    }) else {
        panic!("failed pool must reject admission")
    };
    assert_eq!(error.code, FailureCode::ExecutionInterrupted);
    // Control remains usable after execution has failed, including cancellation.
    let recovered = job(control.dispatch(HostRequest::Submit {
        request: Box::new(request("existing")),
    }));
    assert_eq!(recovered.id, prior.id);
    assert_eq!(recovered.state, JobState::Queued);
    assert!(matches!(
        control.dispatch(HostRequest::GetSchema {
            id: SchemaId::OperationJob
        }),
        HostResponse::Succeeded { .. }
    ));
    assert_eq!(
        job(control.dispatch(HostRequest::GetJob {
            job_id: prior.id.clone()
        }))
        .state,
        JobState::Queued
    );
    assert_eq!(
        job(control.dispatch(HostRequest::CancelJob {
            job_id: prior.id.clone()
        }))
        .state,
        JobState::Cancelled
    );
    assert_eq!(
        host.get_job(&context(), &prior.id, time(50)).unwrap().state,
        JobState::Cancelled
    );
    drop(control);
    assert!(runtime.shutdown().is_err());
    assert_eq!(
        db.sql()
            .query_row("SELECT count(*) FROM jobs", [], |r| r.get::<_, i64>(0))
            .unwrap(),
        1
    );
}

#[test]
fn invalid_worker_budget_never_opens_the_database() {
    let db = Database::new();
    assert_eq!(
        NativeRuntime::start(
            db.config(1),
            context(),
            RuntimeOptions {
                workers: 0,
                ..options()
            },
            Arc::new(|| time(50))
        )
        .err()
        .unwrap()
        .code,
        FailureCode::InputInvalid
    );
    assert!(!db.path().exists());
}

#[test]
fn v4_upgrade_preserves_receipts_and_rejects_corruption_atomically() {
    for corrupt in [false, true] {
        let db = Database::new();
        let mut host = db.host();
        let prior = host
            .submit(&context(), request("existing"), time(10))
            .unwrap();
        drop(host);
        let sql = db.sql();
        sql.execute_batch(
            "DROP INDEX jobs_queued; DROP INDEX jobs_expiring; DROP TABLE job_bindings; PRAGMA user_version=4;",
        )
        .unwrap();
        if corrupt {
            sql.execute("UPDATE jobs SET info='invalid json'", [])
                .unwrap();
        }
        let row = || {
            sql.query_row("SELECT request,info FROM jobs", [], |r| {
                Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?))
            })
            .unwrap()
        };
        let before = row();
        let opened = db.config(1).connect();
        if corrupt {
            assert_eq!(opened.err().unwrap().code, FailureCode::StorageFailure);
        } else {
            assert_eq!(
                serde_json::to_value(
                    opened
                        .unwrap()
                        .get_job(&context(), &prior.id, time(10))
                        .unwrap()
                )
                .unwrap(),
                serde_json::to_value(prior).unwrap()
            );
        }
        assert_eq!(row(), before);
        assert_eq!(
            sql.query_row("PRAGMA user_version", [], |r| r.get::<_, i64>(0))
                .unwrap(),
            if corrupt { 4 } else { 6 }
        );
        assert_eq!(
            sql.query_row(
                "SELECT count(*) FROM sqlite_schema WHERE name IN ('jobs_queued','jobs_expiring')",
                [],
                |r| r.get::<_, i64>(0)
            )
            .unwrap(),
            if corrupt { 0 } else { 2 }
        );
    }
}

#[test]
fn mutation_checkpoints_renew_and_observe_durable_cancellation() {
    use std::cell::{Cell, RefCell};
    for (cancelled, first_check) in [(false, 10), (true, 10), (false, 90), (true, 90)] {
        let db = Database::new();
        let mut host = db.host();
        let prior = host
            .submit(&context(), request("mutation"), time(10))
            .unwrap();
        let other = RefCell::new(db.host());
        // Also cover snapshot preparation consuming most of the initial lease.
        let current = Cell::new(first_check);
        let checkpoints = Cell::new(0);
        let finished = host
            .run_job(
                &context(),
                &prior.id,
                time(10),
                &|| time(current.get()),
                &|| {
                    checkpoints.set(checkpoints.get() + 1);
                    current.set(current.get() + if first_check == 10 { 40 } else { 10 });
                    if cancelled && checkpoints.get() == 2 {
                        other
                            .borrow_mut()
                            .cancel_job(&context(), &prior.id, time(current.get()))
                            .unwrap();
                    }
                    false // The durable cancellation must be observed by the owner.
                },
            )
            .unwrap();
        assert_eq!(
            finished.state,
            if cancelled {
                JobState::Cancelled
            } else {
                JobState::Succeeded
            }
        );
        if cancelled {
            assert_eq!(checkpoints.get(), 2);
            assert_eq!(
                db.sql()
                    .query_row("SELECT count(*) FROM revisions", [], |r| r.get::<_, i64>(0))
                    .unwrap(),
                0
            );
        } else {
            assert!(
                current.get() > 110,
                "computation must exceed the original lease"
            );
            assert_eq!(
                db.sql()
                    .query_row("SELECT count(*) FROM revisions", [], |r| r.get::<_, i64>(0))
                    .unwrap(),
                1
            );
        }
    }
}
