use super::*;

#[test]
fn transient_storage_contention_recovers_without_restarting_or_replaying_work() {
    let db = Database::new();
    let mut host = db.host();
    let runtime =
        NativeRuntime::start(db.config(1), context(), options(), Arc::new(|| time(50))).unwrap();
    let sql = db.sql();
    sql.busy_timeout(Duration::from_secs(2)).unwrap();
    sql.execute_batch("BEGIN IMMEDIATE").unwrap();
    wait(|| matches!(runtime.health(), Err(e) if e.code == FailureCode::StorageBusy));
    // A connection and read-only capability discovery need no writer reservation.
    let session = runtime.connect().unwrap();
    assert!(
        session
            .capabilities()
            .operations
            .iter()
            .filter(|o| o.profile_id.is_some())
            .all(|o| !o.available)
    );
    drop(session);
    sql.execute_batch("ROLLBACK").unwrap();
    wait(|| runtime.health().is_ok());
    let accepted = host
        .submit(&context(), request("after-contention"), time(50))
        .unwrap();
    wait(|| {
        host.get_job(&context(), &accepted.id, time(50))
            .unwrap()
            .state
            .terminal()
    });
    let receipt = host.get_job(&context(), &accepted.id, time(50)).unwrap();
    assert_eq!(receipt.state, JobState::Succeeded);
    assert_eq!(receipt.fence.get(), 1);
    runtime.shutdown().unwrap();
    assert_eq!(
        sql.query_row("SELECT count(*) FROM revisions", [], |r| r.get::<_, i64>(0))
            .unwrap(),
        1
    );
}

#[test]
fn queued_running_and_terminal_status_reads_do_not_reserve_the_writer() {
    let db = Database::new();
    let mut host = db.host();
    let queued = host
        .submit(&context(), request("queued"), time(10))
        .unwrap();
    let running = host
        .submit(&context(), request("running"), time(10))
        .unwrap();
    host.claim(&context(), &running.id, time(20))
        .unwrap()
        .unwrap();
    let terminal = host
        .submit(&context(), request("terminal"), time(10))
        .unwrap();
    host.run_job(&context(), &terminal.id, time(20), &|| time(20), &|| false)
        .unwrap();
    let sql = db.sql();
    sql.execute_batch("BEGIN IMMEDIATE").unwrap();
    // Also exercise acquiring a fresh control connection while a writer exists.
    let mut reader = db.config(1).connect().unwrap();
    for (id, state) in [
        (&queued.id, JobState::Queued),
        (&running.id, JobState::Running),
        (&terminal.id, JobState::Succeeded),
    ] {
        assert_eq!(
            reader.get_job(&context(), id, time(30)).unwrap().state,
            state
        );
    }
    assert_eq!(
        reader
            .cancel_job(&context(), &terminal.id, time(30))
            .unwrap()
            .state,
        JobState::Succeeded
    );
    sql.execute_batch("ROLLBACK").unwrap();
    // Expiry still durably transitions and blocks stale work from publishing.
    assert_eq!(
        reader
            .get_job(&context(), &running.id, time(120))
            .unwrap()
            .state,
        JobState::Failed
    );
}

#[test]
fn control_binding_corruption_is_rejected_and_body_corruption_is_rejected_at_execution() {
    let db = Database::new();
    let mut host = db.host();
    let accepted = host
        .submit(&context(), request("binding"), time(10))
        .unwrap();
    let sql = db.sql();
    sql.execute("UPDATE job_bindings SET document_id='different'", [])
        .unwrap();
    assert_eq!(
        host.get_job(&context(), &accepted.id, time(20))
            .unwrap_err()
            .code,
        FailureCode::StorageFailure
    );
    sql.execute("UPDATE job_bindings SET document_id='binding'", [])
        .unwrap();
    sql.execute(
        "UPDATE jobs SET request=json_set(request,'$.action.document.title','tampered')",
        [],
    )
    .unwrap();
    assert_eq!(
        host.get_job(&context(), &accepted.id, time(20))
            .unwrap()
            .state,
        JobState::Queued
    );
    assert_eq!(
        host.claim(&context(), &accepted.id, time(20))
            .err()
            .unwrap()
            .code,
        FailureCode::StorageFailure
    );
    assert_eq!(
        sql.query_row("SELECT count(*) FROM revisions", [], |r| r.get::<_, i64>(0))
            .unwrap(),
        0
    );
}

#[test]
fn v5_projection_migration_is_atomic_and_preserves_existing_requests_and_receipts() {
    for corrupt in [false, true] {
        let db = Database::new();
        let mut host = db.host();
        let accepted = host
            .submit(&context(), request("migration"), time(10))
            .unwrap();
        host.run_job(&context(), &accepted.id, time(20), &|| time(20), &|| false)
            .unwrap();
        drop(host);
        let sql = db.sql();
        sql.execute_batch("DROP TABLE job_bindings; PRAGMA user_version=5;")
            .unwrap();
        if corrupt {
            sql.execute("UPDATE jobs SET request='{}'", []).unwrap();
        }
        let before: (String, String) = sql
            .query_row("SELECT request,info FROM jobs", [], |r| {
                Ok((r.get(0)?, r.get(1)?))
            })
            .unwrap();
        let opened = db.config(1).connect();
        if corrupt {
            assert_eq!(opened.err().unwrap().code, FailureCode::StorageFailure);
            assert_eq!(
                sql.query_row(
                    "SELECT count(*) FROM sqlite_schema WHERE name='job_bindings'",
                    [],
                    |r| r.get::<_, i64>(0)
                )
                .unwrap(),
                0
            );
        } else {
            assert_eq!(
                opened
                    .unwrap()
                    .get_job(&context(), &accepted.id, time(20))
                    .unwrap()
                    .state,
                JobState::Succeeded
            );
        }
        let after: (String, String) = sql
            .query_row("SELECT request,info FROM jobs", [], |r| {
                Ok((r.get(0)?, r.get(1)?))
            })
            .unwrap();
        assert_eq!(before, after);
        assert_eq!(
            sql.query_row("PRAGMA user_version", [], |r| r.get::<_, i64>(0))
                .unwrap(),
            if corrupt { 5 } else { 6 }
        );
    }
}
