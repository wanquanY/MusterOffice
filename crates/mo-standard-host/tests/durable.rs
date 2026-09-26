mod support;

use mo_common::{Digest, DocumentId, OperationId, RequestId};
use mo_operation_service::*;
use mo_presentation_edit::{Operation, OperationEntry, SnapshotRecord};
use mo_presentation_model::Document;
use mo_standard_host::{HostLimits, StandardHost, WorkItem};
use rusqlite::Connection;
use std::{
    path::PathBuf,
    sync::{Arc, Barrier},
};

struct Database(PathBuf);
impl Database {
    fn new() -> Self {
        Self(support::temporary_directory("mo-host-test"))
    }
    fn path(&self) -> PathBuf {
        self.0.join("operations.sqlite")
    }
    fn host(&self) -> StandardHost {
        StandardHost::open(self.path(), executor(), HostLimits::default()).unwrap()
    }
    fn count(&self, table: &str) -> u32 {
        assert!(["jobs", "heads", "revisions"].contains(&table));
        Connection::open(self.path())
            .unwrap()
            .query_row(&format!("SELECT count(*) FROM {table}"), [], |r| r.get(0))
            .unwrap()
    }
}
impl Drop for Database {
    fn drop(&mut self) {
        std::fs::remove_dir_all(&self.0).unwrap();
    }
}
fn executor() -> Digest {
    Digest::from_sha256([1; 32])
}
fn time(t: i64) -> UnixMillis {
    UnixMillis::new(t).unwrap()
}
fn context() -> CallContext {
    CallContext {
        principal: PrincipalId::new("agent:one").unwrap(),
        scope: ScopeId::new("workspace:one").unwrap(),
        permissions: [
            Permission::Create,
            Permission::Edit,
            Permission::ReadDocument,
            Permission::ReadJob,
            Permission::CancelJob,
        ]
        .into_iter()
        .collect(),
    }
}
fn document() -> Document {
    let v: serde_json::Value = serde_json::from_str(include_str!(
        "../../../fixtures/presentations/playback/page.json"
    ))
    .unwrap();
    serde_json::from_value(v["page"]["document"].clone()).unwrap()
}
fn create(id: &str) -> OperationRequest {
    OperationRequest {
        contract_version: ContractVersion::V1,
        request_id: RequestId::new(id).unwrap(),
        profile_id: OperationProfile::AuthorModel,
        output_mode: OutputMode::Job,
        action: DocumentAction::Create {
            document: Box::new(document()),
        },
    }
}
fn apply(base: &SnapshotRecord, id: &str, title: &str) -> OperationRequest {
    OperationRequest {
        action: DocumentAction::Apply {
            document_id: base.document.id.clone(),
            base_revision: base.revision.clone(),
            operations: vec![OperationEntry {
                operation_id: OperationId::new("title").unwrap(),
                operation: Operation::SetTitle {
                    title: title.into(),
                },
            }],
        },
        ..create(id)
    }
}
fn complete(host: &mut StandardHost, request: OperationRequest) -> JobInfo {
    let job = host.submit(&context(), request, time(100)).unwrap();
    host.run_job(&context(), &job.id, time(101), &|| time(102), &|| false)
        .unwrap()
}
fn initial(host: &mut StandardHost) -> SnapshotRecord {
    let job = complete(host, create("create"));
    assert_eq!(job.state, JobState::Succeeded);
    host.read_document(&context(), &job.document_id, None)
        .unwrap()
}
fn code(job: &JobInfo) -> FailureCode {
    match job.result.as_ref().unwrap() {
        TerminalResult::Failed { error } => error.code,
        _ => panic!("expected failed job"),
    }
}
fn compute(work: &WorkItem) -> MutationCandidate {
    compute_mutation(&work.request, work.snapshot.clone(), &|| false).unwrap()
}
fn equal(a: &JobInfo, b: &JobInfo) {
    assert_eq!(
        serde_json::to_value(a).unwrap(),
        serde_json::to_value(b).unwrap()
    );
}

#[test]
fn durable_create_edit_history_and_retry_after_reopen() {
    let db = Database::new();
    let mut host = db.host();
    let old = initial(&mut host);
    let request = apply(&old, "edit", "Agent 编辑 ✓");
    let result = complete(&mut host, request.clone());
    assert_eq!(result.state, JobState::Succeeded);
    let changed = host
        .read_document(&context(), &old.document.id, None)
        .unwrap();
    assert_eq!(changed.document.title, "Agent 编辑 ✓");
    assert_ne!(changed.revision, old.revision);
    assert_eq!(
        host.read_document(&context(), &old.document.id, Some(&old.revision))
            .unwrap(),
        old
    );
    drop(host);
    let mut host = db.host();
    let mut retry = request;
    retry.output_mode = OutputMode::Sync;
    // Replay is resolved before checking the now-stale document revision.
    equal(&result, &host.submit(&context(), retry, time(200)).unwrap());
    equal(
        &result,
        &host.get_job(&context(), &result.id, time(0)).unwrap(),
    );
    assert_eq!(
        host.read_document(&context(), &old.document.id, None)
            .unwrap(),
        changed
    );
    assert_eq!(
        (db.count("jobs"), db.count("revisions"), db.count("heads")),
        (2, 2, 1)
    );
}

#[test]
fn duplicate_submission_is_serialized_across_connections() {
    let db = Database::new();
    let _init = db.host();
    let barrier = Arc::new(Barrier::new(4));
    let results = std::thread::scope(|scope| {
        let handles: Vec<_> = (0..4)
            .map(|_| {
                let mut host = db.host();
                let barrier = barrier.clone();
                scope.spawn(move || {
                    barrier.wait();
                    host.submit(&context(), create("same"), time(100)).unwrap()
                })
            })
            .collect();
        handles
            .into_iter()
            .map(|h| h.join().unwrap())
            .collect::<Vec<_>>()
    });
    for result in &results {
        equal(result, &results[0]);
    }
    assert_eq!(db.count("jobs"), 1);
    let mut host = db.host();
    let first = host
        .claim(&context(), &results[0].id, time(101))
        .unwrap()
        .unwrap();
    assert!(
        db.host()
            .claim(&context(), &results[0].id, time(101))
            .unwrap()
            .is_none()
    );
    host.finish(&context(), &first.lease, Ok(compute(&first)), time(102))
        .unwrap();
    assert_eq!(db.count("revisions"), 1);
}

#[test]
fn request_reuse_checks_arguments_and_permissions_before_replay() {
    let db = Database::new();
    let mut host = db.host();
    let job = complete(&mut host, create("key"));
    let mut changed = create("key");
    if let DocumentAction::Create { document } = &mut changed.action {
        document.title = "different".into();
    }
    assert_eq!(
        host.submit(&context(), changed, time(200))
            .unwrap_err()
            .code,
        FailureCode::RequestIdReused
    );
    let mut revoked = context();
    revoked.permissions.clear();
    assert_eq!(
        host.submit(&revoked, create("key"), time(200))
            .unwrap_err()
            .code,
        FailureCode::NotAuthorized
    );
    assert_eq!(
        host.get_job(&revoked, &job.id, time(200)).unwrap_err().code,
        FailureCode::NotAuthorized
    );
    assert_eq!(
        host.cancel_job(&revoked, &job.id, time(200))
            .unwrap_err()
            .code,
        FailureCode::NotAuthorized
    );
    assert_eq!(db.count("jobs"), 1);
}

#[test]
fn mutation_permission_allows_execution_without_granting_other_query_permissions() {
    let db = Database::new();
    let mut host = db.host();
    let mut authority = context();
    authority.permissions = [Permission::Create].into_iter().collect();
    let job = host
        .submit(&authority, create("minimal"), time(100))
        .unwrap();
    let result = host
        .run_job(&authority, &job.id, time(101), &|| time(102), &|| false)
        .unwrap();
    assert_eq!(result.state, JobState::Succeeded);
    equal(
        &result,
        &host
            .run_job(&authority, &job.id, time(103), &|| time(104), &|| false)
            .unwrap(),
    );
    assert_eq!(
        host.get_job(&authority, &job.id, time(105))
            .unwrap_err()
            .code,
        FailureCode::NotAuthorized
    );
    assert_eq!(
        host.read_document(&authority, &result.document_id, None)
            .unwrap_err()
            .code,
        FailureCode::NotAuthorized
    );
}

#[test]
fn scope_and_principal_isolation_with_explicit_shared_document_scope() {
    let db = Database::new();
    let mut host = db.host();
    let job = complete(&mut host, create("key"));
    let mut another = context();
    another.principal = PrincipalId::new("agent:two").unwrap();
    assert_eq!(
        host.get_job(&another, &job.id, time(200)).unwrap_err().code,
        FailureCode::NotFound
    );
    // Members explicitly authorized for this scope can read the shared document.
    assert!(host.read_document(&another, &job.document_id, None).is_ok());
    another.scope = ScopeId::new("workspace:two").unwrap();
    assert_eq!(
        host.read_document(&another, &job.document_id, None)
            .unwrap_err()
            .code,
        FailureCode::NotFound
    );
    let separate = host.submit(&another, create("key"), time(200)).unwrap();
    assert_ne!(separate.id, job.id);
    assert_eq!(
        host.run_job(&another, &separate.id, time(201), &|| time(202), &|| false)
            .unwrap()
            .state,
        JobState::Succeeded
    );
    assert_eq!((db.count("heads"), db.count("revisions")), (2, 2));
}

#[test]
fn simultaneous_edits_commit_exactly_one_revision() {
    let db = Database::new();
    let mut host = db.host();
    let base = initial(&mut host);
    let mut work = Vec::new();
    for id in ["left", "right"] {
        let job = host
            .submit(&context(), apply(&base, id, id), time(200))
            .unwrap();
        work.push(host.claim(&context(), &job.id, time(201)).unwrap().unwrap());
    }
    let barrier = Arc::new(Barrier::new(2));
    let results = std::thread::scope(|scope| {
        let handles: Vec<_> = work
            .into_iter()
            .map(|work| {
                let mut host = db.host();
                let barrier = barrier.clone();
                let candidate = compute(&work);
                scope.spawn(move || {
                    barrier.wait();
                    host.finish(&context(), &work.lease, Ok(candidate), time(202))
                        .unwrap()
                })
            })
            .collect();
        handles
            .into_iter()
            .map(|h| h.join().unwrap())
            .collect::<Vec<_>>()
    });
    assert_eq!(
        results
            .iter()
            .filter(|j| j.state == JobState::Succeeded)
            .count(),
        1
    );
    assert_eq!(
        code(
            results
                .iter()
                .find(|j| j.state == JobState::Failed)
                .unwrap()
        ),
        FailureCode::RevisionConflict
    );
    assert_eq!(db.count("revisions"), 2);
    let current = host
        .read_document(&context(), &base.document.id, None)
        .unwrap();
    let winner = results
        .iter()
        .find(|j| j.state == JobState::Succeeded)
        .unwrap();
    assert_eq!(current.document.title, winner.request_id.as_str());
}

#[test]
fn queued_and_running_cancellation_never_publish_candidates() {
    let db = Database::new();
    let mut host = db.host();
    let queued = host
        .submit(&context(), create("queued"), time(100))
        .unwrap();
    let cancelled = host.cancel_job(&context(), &queued.id, time(101)).unwrap();
    assert_eq!(cancelled.state, JobState::Cancelled);
    assert!(
        host.claim(&context(), &queued.id, time(102))
            .unwrap()
            .is_none()
    );
    let job = host
        .submit(&context(), create("running"), time(100))
        .unwrap();
    let work = host.claim(&context(), &job.id, time(101)).unwrap().unwrap();
    let candidate = compute(&work);
    let requested = db
        .host()
        .cancel_job(&context(), &job.id, time(102))
        .unwrap();
    assert_eq!(requested.state, JobState::Running);
    assert!(requested.cancel_requested);
    // Cancellation does not claim that the computing process has stopped.
    assert_eq!(
        host.renew(&context(), &work.lease, time(103))
            .unwrap()
            .lease_until,
        requested.lease_until
    );
    let terminal = host
        .finish(&context(), &work.lease, Ok(candidate), time(104))
        .unwrap();
    assert_eq!(terminal.state, JobState::Cancelled);
    equal(
        &terminal,
        &host.cancel_job(&context(), &job.id, time(105)).unwrap(),
    );
    assert_eq!(db.count("heads"), 0);
    assert_eq!(db.count("revisions"), 0);
}

#[test]
fn cancellation_and_commit_race_has_one_durable_winner() {
    for _ in 0..8 {
        let db = Database::new();
        let mut host = db.host();
        let job = host.submit(&context(), create("race"), time(100)).unwrap();
        let work = host.claim(&context(), &job.id, time(101)).unwrap().unwrap();
        let candidate = compute(&work);
        let mut cancelling = db.host();
        let barrier = Arc::new(Barrier::new(2));
        let other = barrier.clone();
        let (finished, cancelled) = std::thread::scope(|scope| {
            let cancel = scope.spawn(|| {
                other.wait();
                cancelling
                    .cancel_job(&context(), &job.id, time(102))
                    .unwrap()
            });
            barrier.wait();
            let result = host
                .finish(&context(), &work.lease, Ok(candidate), time(102))
                .unwrap();
            (result, cancel.join().unwrap())
        });
        match finished.state {
            JobState::Succeeded => {
                assert_eq!(cancelled.state, JobState::Succeeded);
                assert_eq!(db.count("heads"), 1);
            }
            JobState::Cancelled => {
                assert!(cancelled.cancel_requested);
                assert_eq!(db.count("heads"), 0);
            }
            _ => panic!("unexpected race result"),
        }
        equal(
            &finished,
            &db.host().get_job(&context(), &job.id, time(200)).unwrap(),
        );
    }
}

#[test]
fn lease_expiry_and_executor_changes_cannot_publish_late_results() {
    let db = Database::new();
    let mut host = db.host();
    let job = host
        .submit(&context(), create("expired"), time(100))
        .unwrap();
    let work = host.claim(&context(), &job.id, time(101)).unwrap().unwrap();
    let renewed = host.renew(&context(), &work.lease, time(200)).unwrap();
    assert_eq!(renewed.lease_until, Some(time(30200)));
    let mut wrong = StandardHost::open(
        db.path(),
        Digest::from_sha256([2; 32]),
        HostLimits::default(),
    )
    .unwrap();
    assert_eq!(
        wrong
            .finish(&context(), &work.lease, Ok(compute(&work)), time(201))
            .unwrap_err()
            .code,
        FailureCode::ExecutorMismatch
    );
    let expired = db.host().get_job(&context(), &job.id, time(30200)).unwrap();
    assert_eq!(code(&expired), FailureCode::ExecutionInterrupted);
    equal(
        &expired,
        &host
            .finish(&context(), &work.lease, Ok(compute(&work)), time(30201))
            .unwrap(),
    );
    assert_eq!(db.count("revisions"), 0);
    let queued = host
        .submit(&context(), create("queued"), time(40000))
        .unwrap();
    assert!(matches!(
        wrong.claim(&context(), &queued.id, time(40001)),
        Err(Failure {
            code: FailureCode::ExecutorMismatch,
            ..
        })
    ));
    assert_eq!(
        host.get_job(&context(), &queued.id, time(40002))
            .unwrap()
            .state,
        JobState::Queued
    );
}

#[test]
fn cancellation_survives_expiry_and_clock_regression_cannot_extend_a_lease() {
    let db = Database::new();
    let mut host = db.host();
    let job = host
        .submit(&context(), create("cancel"), time(100))
        .unwrap();
    let work = host.claim(&context(), &job.id, time(101)).unwrap().unwrap();
    assert_eq!(
        host.renew(&context(), &work.lease, time(99))
            .unwrap_err()
            .code,
        FailureCode::InputInvalid
    );
    host.cancel_job(&context(), &job.id, time(102)).unwrap();
    let terminal = db.host().get_job(&context(), &job.id, time(30101)).unwrap();
    assert_eq!(code(&terminal), FailureCode::Cancelled);
    assert_eq!(db.count("heads"), 0);
}

#[test]
fn revoked_commit_and_cross_operation_candidate_do_not_modify_document() {
    let db = Database::new();
    let mut host = db.host();
    let job = host.submit(&context(), create("one"), time(100)).unwrap();
    let work = host.claim(&context(), &job.id, time(101)).unwrap().unwrap();
    let mut revoked = context();
    revoked.permissions.remove(&Permission::Create);
    assert_eq!(
        host.finish(&revoked, &work.lease, Ok(compute(&work)), time(102))
            .unwrap_err()
            .code,
        FailureCode::NotAuthorized
    );
    let mut other = context();
    other.principal = PrincipalId::new("other").unwrap();
    assert_eq!(
        host.finish(&other, &work.lease, Ok(compute(&work)), time(102))
            .unwrap_err()
            .code,
        FailureCode::NotFound
    );
    let wrong = compute_mutation(&create("two"), None, &|| false).unwrap();
    let result = host
        .finish(&context(), &work.lease, Ok(wrong), time(102))
        .unwrap();
    assert_eq!(code(&result), FailureCode::StaleExecution);
    assert_eq!(db.count("heads"), 0);
}

#[test]
fn write_failure_rolls_back_snapshot_head_and_terminal_receipt_together() {
    let db = Database::new();
    let mut host = db.host();
    let job = host
        .submit(&context(), create("atomic"), time(100))
        .unwrap();
    let work = host.claim(&context(), &job.id, time(101)).unwrap().unwrap();
    let admin = Connection::open(db.path()).unwrap();
    // Fail the last write, after both revision insertion and head update occurred.
    admin.execute_batch("CREATE TRIGGER test_disk_failure BEFORE UPDATE ON jobs BEGIN SELECT RAISE(ABORT,'synthetic write failure'); END;").unwrap();
    assert_eq!(
        host.finish(&context(), &work.lease, Ok(compute(&work)), time(102))
            .unwrap_err()
            .code,
        FailureCode::StorageFailure
    );
    assert_eq!((db.count("revisions"), db.count("heads")), (0, 0));
    assert_eq!(
        db.host()
            .get_job(&context(), &job.id, time(103))
            .unwrap()
            .state,
        JobState::Running
    );
    admin
        .execute_batch("DROP TRIGGER test_disk_failure;")
        .unwrap();
    let result = host
        .finish(&context(), &work.lease, Ok(compute(&work)), time(104))
        .unwrap();
    assert_eq!(result.state, JobState::Succeeded);
    equal(
        &result,
        &host
            .finish(&context(), &work.lease, Ok(compute(&work)), time(105))
            .unwrap(),
    );
    assert_eq!((db.count("revisions"), db.count("heads")), (1, 1));
}

#[test]
fn semantic_failure_and_cooperative_cancel_are_durable_without_partial_edit() {
    let db = Database::new();
    let mut host = db.host();
    let base = initial(&mut host);
    let mut request = apply(&base, "invalid", "must not leak");
    if let DocumentAction::Apply { operations, .. } = &mut request.action {
        operations.push(OperationEntry {
            operation_id: OperationId::new("missing").unwrap(),
            operation: Operation::MoveSlide {
                slide: mo_common::SlideId::new("missing").unwrap(),
                index: 0,
            },
        });
    }
    let failed = complete(&mut host, request);
    assert_eq!(failed.state, JobState::Failed);
    let job = host
        .submit(
            &context(),
            apply(&base, "cancel-compute", "cancelled"),
            time(200),
        )
        .unwrap();
    let cancelled = host
        .run_job(&context(), &job.id, time(201), &|| time(202), &|| true)
        .unwrap();
    assert_eq!(code(&cancelled), FailureCode::Cancelled);
    assert_eq!(
        host.read_document(&context(), &base.document.id, None)
            .unwrap(),
        base
    );
    assert_eq!(db.count("revisions"), 1);
}

#[test]
fn quotas_preserve_replays_and_do_not_leave_partial_documents() {
    let db = Database::new();
    let mut host = StandardHost::open(
        db.path(),
        executor(),
        HostLimits {
            max_jobs_per_principal: 2,
            max_documents_per_scope: 1,
            ..HostLimits::default()
        },
    )
    .unwrap();
    let first = complete(&mut host, create("first"));
    assert_eq!(first.state, JobState::Succeeded);
    let mut request = create("second");
    if let DocumentAction::Create { document } = &mut request.action {
        document.id = DocumentId::new("document:two").unwrap();
    }
    assert_eq!(
        code(&complete(&mut host, request)),
        FailureCode::LimitExceeded
    );
    assert_eq!(
        host.submit(&context(), create("third"), time(200))
            .unwrap_err()
            .code,
        FailureCode::LimitExceeded
    );
    equal(
        &first,
        &host.submit(&context(), create("first"), time(200)).unwrap(),
    );
    assert_eq!(
        (db.count("jobs"), db.count("heads"), db.count("revisions")),
        (2, 1, 1)
    );
}

#[test]
fn unrelated_database_is_rejected_without_schema_or_journal_changes() {
    let db = Database::new();
    let c = Connection::open(db.path()).unwrap();
    c.execute_batch(
        "CREATE TABLE private_data(value TEXT); INSERT INTO private_data VALUES ('keep');",
    )
    .unwrap();
    assert!(matches!(
        StandardHost::open(db.path(), executor(), HostLimits::default()),
        Err(Failure {
            code: FailureCode::StorageFailure,
            ..
        })
    ));
    assert_eq!(
        c.query_row("PRAGMA journal_mode", [], |r| r.get::<_, String>(0))
            .unwrap(),
        "delete"
    );
    assert_eq!(
        c.query_row("SELECT value FROM private_data", [], |r| r
            .get::<_, String>(0))
            .unwrap(),
        "keep"
    );
    assert_eq!(
        c.query_row(
            "SELECT count(*) FROM sqlite_schema WHERE type='table'",
            [],
            |r| r.get::<_, u32>(0)
        )
        .unwrap(),
        1
    );
}

#[test]
fn corrupted_state_and_snapshot_are_rejected_on_read() {
    let db = Database::new();
    let mut host = db.host();
    let job = complete(&mut host, create("one"));
    let admin = Connection::open(db.path()).unwrap();
    admin
        .execute_batch("UPDATE jobs SET info=json_set(info,'$.state','failed');")
        .unwrap();
    assert_eq!(
        host.get_job(&context(), &job.id, time(200))
            .unwrap_err()
            .code,
        FailureCode::StorageFailure
    );
    admin
        .execute_batch(
            "UPDATE revisions SET snapshot=json_set(snapshot,'$.document.title','tampered');",
        )
        .unwrap();
    assert_eq!(
        host.read_document(&context(), &job.document_id, None)
            .unwrap_err()
            .code,
        FailureCode::StorageFailure
    );
}

#[test]
fn relabelled_base_content_cannot_be_committed_under_a_known_revision() {
    let db = Database::new();
    let mut host = db.host();
    let original = initial(&mut host);
    let job = host
        .submit(&context(), apply(&original, "edit", "new title"), time(200))
        .unwrap();
    let work = host.claim(&context(), &job.id, time(201)).unwrap().unwrap();
    let mut forged = original.clone();
    // This remains a semantically valid document but was never stored under
    // that revision. The pure computation cannot verify host provenance.
    forged
        .document
        .objects
        .get_mut(&mo_common::ObjectId::new("shape:1").unwrap())
        .unwrap()
        .accessibility
        .title = "unauthorized prior content".into();
    forged.semantic_digest = forged.document.semantic_digest().unwrap();
    let candidate = compute_mutation(&work.request, Some(forged), &|| false).unwrap();
    assert_eq!(
        code(
            &host
                .finish(&context(), &work.lease, Ok(candidate), time(202))
                .unwrap()
        ),
        FailureCode::StaleExecution
    );
    assert_eq!(
        host.read_document(&context(), &original.document.id, None)
            .unwrap(),
        original
    );
    assert_eq!(db.count("revisions"), 1);
}

#[test]
fn killed_worker_leaves_a_recoverable_durable_lease_and_no_revision() {
    use std::{
        process::{Command, Stdio},
        time::{Duration, Instant},
    };
    let db = Database::new();
    let mut host = db.host();
    let job = host
        .submit(&context(), create("process"), time(100))
        .unwrap();
    let ready = db.0.join("ready");
    let mut child = Command::new(std::env::current_exe().unwrap())
        .args(["--exact", "process_worker", "--ignored", "--nocapture"])
        .env("MO_HOST_TEST_DATABASE", db.path())
        .env("MO_HOST_TEST_READY", &ready)
        .env("MO_HOST_TEST_JOB", job.id.as_str())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    let deadline = Instant::now() + Duration::from_secs(10);
    while !ready.exists() && Instant::now() < deadline {
        if child.try_wait().unwrap().is_some() {
            panic!("worker exited before claiming");
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    let claimed = ready.exists();
    child.kill().unwrap();
    child.wait().unwrap();
    assert!(claimed, "worker failed to signal its committed lease");
    drop(host);
    let mut reopened = db.host();
    assert_eq!(
        reopened
            .get_job(&context(), &job.id, time(102))
            .unwrap()
            .state,
        JobState::Running
    );
    let interrupted = reopened.get_job(&context(), &job.id, time(30101)).unwrap();
    assert_eq!(code(&interrupted), FailureCode::ExecutionInterrupted);
    equal(
        &interrupted,
        &reopened
            .submit(&context(), create("process"), time(30102))
            .unwrap(),
    );
    assert_eq!((db.count("heads"), db.count("revisions")), (0, 0));
    assert!(
        reopened
            .claim(&context(), &job.id, time(30103))
            .unwrap()
            .is_none()
    );
}

// Test-only subprocess entry. No production checkpoint, environment branch or
// crash hook is added to the library or CLI.
#[test]
#[ignore = "invoked as a disposable child by killed_worker test"]
fn process_worker() {
    let db = std::env::var_os("MO_HOST_TEST_DATABASE").unwrap();
    let ready = std::env::var_os("MO_HOST_TEST_READY").unwrap();
    let id = JobId::new(std::env::var("MO_HOST_TEST_JOB").unwrap()).unwrap();
    let mut host =
        StandardHost::open(PathBuf::from(db), executor(), HostLimits::default()).unwrap();
    let work = host.claim(&context(), &id, time(101)).unwrap().unwrap();
    let _unpublished = compute(&work);
    std::fs::write(ready, b"claimed and computed").unwrap();
    loop {
        std::thread::park();
    }
}
