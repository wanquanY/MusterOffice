mod support;

use mo_common::{ByteLength, Digest, RequestId};
use mo_opc::{ReaderAt, ResultSink};
use mo_operation_service::*;
use mo_standard_host::{HostLimits, ResultSpec, StandardHost, WorkItem};
use rusqlite::Connection;
use sha2::{Digest as _, Sha256};
use std::{
    cell::Cell,
    io::Write,
    path::PathBuf,
    sync::{Arc, Barrier},
};

struct Database(PathBuf);
impl Database {
    fn new() -> Self {
        Self(support::temporary_directory("mo-results"))
    }
    fn path(&self) -> PathBuf {
        self.0.join("host.sqlite")
    }
    fn host(&self) -> StandardHost {
        host(self.path(), HostLimits::default())
    }
    fn sql(&self) -> Connection {
        Connection::open(self.path()).unwrap()
    }
    fn value(&self, sql: &str) -> i64 {
        self.sql().query_row(sql, [], |r| r.get(0)).unwrap()
    }
    fn reserved(&self) -> i64 {
        self.value("SELECT coalesce(sum(reserved_bytes),0) FROM result_spools")
    }
    fn chunks(&self) -> i64 {
        self.value("SELECT count(*) FROM result_chunks")
    }
}
impl Drop for Database {
    fn drop(&mut self) {
        std::fs::remove_dir_all(&self.0).unwrap();
    }
}
fn host(path: PathBuf, limits: HostLimits) -> StandardHost {
    StandardHost::open(path, Digest::from_sha256([1; 32]), limits).unwrap()
}
fn context() -> CallContext {
    CallContext {
        principal: PrincipalId::new("worker").unwrap(),
        scope: ScopeId::new("workspace").unwrap(),
        permissions: [
            Permission::Create,
            Permission::ReadJob,
            Permission::CancelJob,
            Permission::WriteAssets,
            Permission::ReadAssets,
        ]
        .into_iter()
        .collect(),
    }
}
fn time(n: i64) -> UnixMillis {
    UnixMillis::new(n).unwrap()
}
fn clock() -> UnixMillis {
    time(20)
}
fn spec(name: &str, max_bytes: u64) -> ResultSpec {
    ResultSpec {
        name: RequestId::new(name).unwrap(),
        media_type: "application/octet-stream".into(),
        max_bytes,
    }
}
fn work(host: &mut StandardHost) -> WorkItem {
    let v: serde_json::Value = serde_json::from_str(include_str!(
        "../../../fixtures/presentations/native-export/request.json"
    ))
    .unwrap();
    let job = host
        .submit(
            &context(),
            OperationRequest {
                contract_version: ContractVersion::V1,
                request_id: RequestId::new("create").unwrap(),
                profile_id: OperationProfile::AuthorModel,
                output_mode: OutputMode::Job,
                action: DocumentAction::Create {
                    document: Box::new(serde_json::from_value(v["document"].clone()).unwrap()),
                },
            },
            time(10),
        )
        .unwrap();
    host.claim(&context(), &job.id, time(11)).unwrap().unwrap()
}
fn upload_request(name: &str, n: u64) -> UploadRequest {
    UploadRequest {
        request_id: RequestId::new(name).unwrap(),
        descriptor: AssetDescriptor {
            sha256: Digest::from_sha256([0; 32]),
            byte_length: ByteLength::new(n),
            media_type: "application/octet-stream".into(),
        },
    }
}
fn error_code(error: std::io::Error) -> FailureCode {
    error
        .into_inner()
        .unwrap()
        .downcast::<Failure>()
        .unwrap()
        .code
}

#[test]
fn partial_flush_extends_only_tail_and_seal_reads_persisted_chunks() {
    let db = Database::new();
    let mut host = db.host();
    let work = work(&mut host);
    let mut sink = host
        .create_result(&context(), &work.lease, spec("pptx", 1_000_000), &clock)
        .unwrap();
    let bytes: Vec<u8> = (0..ASSET_CHUNK_BYTES * 2 + 31)
        .map(|i| (i % 251) as u8)
        .collect();
    sink.write_all(&bytes[..37]).unwrap();
    sink.flush().unwrap();
    assert_eq!(db.chunks(), 1);
    assert_eq!(db.value("SELECT sum(length(data)) FROM result_chunks"), 37);
    sink.write_all(&bytes[37..ASSET_CHUNK_BYTES + 11]).unwrap();
    sink.flush().unwrap();
    sink.write_all(&bytes[ASSET_CHUNK_BYTES + 11..]).unwrap();
    let sealed = sink.seal().unwrap();
    assert_eq!(sealed.byte_length, bytes.len() as u64);
    assert_eq!(
        sealed.reader.sha256(),
        &Digest::from_sha256(Sha256::digest(&bytes).into())
    );
    assert_eq!(db.reserved(), bytes.len() as i64);
    assert_eq!(db.chunks(), 3);
    let mut actual = vec![0; bytes.len()];
    sealed.reader.read_exact_at(&mut actual, 0).unwrap();
    assert_eq!(actual, bytes);
    assert_eq!(sealed.reader.read_at(&mut [0; 1], u64::MAX).unwrap(), 0);
    drop(sealed);
    drop(host);
    let reopened = db.host();
    let reader = reopened
        .open_result(
            &context(),
            &work.lease,
            &RequestId::new("pptx").unwrap(),
            &clock,
        )
        .unwrap();
    reader.read_exact_at(&mut actual, 0).unwrap();
    assert_eq!(actual, bytes);
    assert!(matches!(
        reopened.asset_info(&context(), &AssetId::new("pptx").unwrap()),
        Err(Failure {
            code: FailureCode::NotFound,
            ..
        })
    ));
    assert_eq!(db.value("SELECT count(*) FROM assets"), 0);
}

#[test]
fn cancel_revokes_existing_writers_readers_and_reservations_atomically() {
    let db = Database::new();
    let mut host = db.host();
    let work = work(&mut host);
    let mut a = host
        .create_result(&context(), &work.lease, spec("a", 100), &clock)
        .unwrap();
    a.write_all(b"sealed bytes").unwrap();
    let reader = a.seal().unwrap().reader;
    let mut b = host
        .create_result(&context(), &work.lease, spec("b", 100), &clock)
        .unwrap();
    b.write_all(b"partial").unwrap();
    b.flush().unwrap();
    let mut other = db.host();
    let cancelled = other
        .cancel_job(&context(), work.lease.job_id(), time(21))
        .unwrap();
    assert!(cancelled.cancel_requested);
    assert_eq!(db.reserved(), 0);
    assert_eq!(db.chunks(), 0);
    assert_eq!(
        error_code(reader.read_at(&mut [0; 1], 0).unwrap_err()),
        FailureCode::Cancelled
    );
    assert_eq!(
        error_code(b.write(b"x").unwrap_err()),
        FailureCode::Cancelled
    );
    assert!(b.seal().is_err());
    let ended = other
        .finish(
            &context(),
            &work.lease,
            Err(Failure::new(FailureCode::InputInvalid, "unused")),
            time(22),
        )
        .unwrap();
    assert_eq!(ended.state, JobState::Cancelled);
}

#[test]
fn byte_limit_failure_is_sticky_and_drop_abandons_without_flushing() {
    let db = Database::new();
    let mut host = db.host();
    let work = work(&mut host);
    let mut sink = host
        .create_result(&context(), &work.lease, spec("limited", 5), &clock)
        .unwrap();
    sink.write_all(b"four").unwrap();
    assert_eq!(
        error_code(sink.write(b"xx").unwrap_err()),
        FailureCode::LimitExceeded
    );
    assert_eq!(
        error_code(sink.write(b"x").unwrap_err()),
        FailureCode::LimitExceeded
    );
    assert!(sink.seal().is_err());
    assert_eq!(db.reserved(), 0);
    assert_eq!(db.chunks(), 0);
    assert!(matches!(
        host.create_result(&context(), &work.lease, spec("limited", 5), &clock),
        Err(Failure {
            code: FailureCode::ResourceConflict,
            ..
        })
    ));
    let mut dropped = host
        .create_result(&context(), &work.lease, spec("dropped", 100), &clock)
        .unwrap();
    dropped.write_all(b"unflushed").unwrap();
    drop(dropped);
    assert_eq!(db.reserved(), 0);
    assert_eq!(db.chunks(), 0);
}

#[test]
fn persisted_corruption_and_missing_chunks_cannot_seal_or_read() {
    let db = Database::new();
    let mut host = db.host();
    let work = work(&mut host);
    let mut sink = host
        .create_result(&context(), &work.lease, spec("bad", 100), &clock)
        .unwrap();
    sink.write_all(b"abcd").unwrap();
    sink.flush().unwrap();
    db.sql()
        .execute_batch("UPDATE result_chunks SET data=x'01020304';")
        .unwrap();
    assert!(sink.seal().is_err());
    assert_eq!(db.chunks(), 0);
    let mut sink = host
        .create_result(&context(), &work.lease, spec("good", 100), &clock)
        .unwrap();
    sink.write_all(b"abcd").unwrap();
    let reader = sink.seal().unwrap().reader;
    db.sql()
        .execute_batch("UPDATE result_chunks SET data=x'01020304';")
        .unwrap();
    assert!(reader.read_at(&mut [0; 4], 0).is_err());
    db.sql()
        .execute_batch("DELETE FROM result_chunks;")
        .unwrap();
    assert!(reader.read_at(&mut [0; 4], 0).is_err());
}

#[test]
fn scope_reservation_is_shared_with_uploads_and_racing_outputs() {
    let db = Database::new();
    let mut limits = HostLimits::default();
    limits.assets.max_asset_bytes = 1000;
    limits.assets.max_scope_bytes = 1000;
    let mut host = host(db.path(), limits);
    let work = work(&mut host);
    let mut other = crate::host(db.path(), limits);
    let sink = host
        .create_result(&context(), &work.lease, spec("out", 600), &clock)
        .unwrap();
    assert_eq!(
        other
            .begin_upload(&context(), upload_request("blocked", 401), time(21))
            .unwrap_err()
            .code,
        FailureCode::LimitExceeded
    );
    let upload = other
        .begin_upload(&context(), upload_request("input", 400), time(21))
        .unwrap();
    assert!(matches!(
        host.create_result(&context(), &work.lease, spec("more", 1), &clock),
        Err(Failure {
            code: FailureCode::LimitExceeded,
            ..
        })
    ));
    drop(sink);
    other
        .cancel_upload(&context(), &upload.id, time(22))
        .unwrap();
    let barrier = Arc::new(Barrier::new(2));
    let mut threads = Vec::new();
    for name in ["race-a", "race-b"] {
        let path = db.path();
        let lease = work.lease.clone();
        let barrier = barrier.clone();
        threads.push(std::thread::spawn(move || {
            let host = crate::host(path, limits);
            barrier.wait();
            let result = host.create_result(&context(), &lease, spec(name, 600), &clock);
            let won = result.is_ok();
            barrier.wait();
            drop(result);
            won
        }));
    }
    assert_eq!(
        threads
            .into_iter()
            .filter_map(|t| t.join().unwrap().then_some(()))
            .count(),
        1
    );
    assert_eq!(db.reserved(), 0);
}

#[test]
fn expiry_reaps_orphans_and_renewal_keeps_live_output_reserved() {
    let db = Database::new();
    let mut host = db.host();
    let work = work(&mut host);
    let now = Cell::new(time(20));
    let clock = || now.get();
    let mut sink = host
        .create_result(&context(), &work.lease, spec("lease", 100), &clock)
        .unwrap();
    sink.write_all(b"active").unwrap();
    let reader = sink.seal().unwrap().reader;
    let mut other = db.host();
    other.renew(&context(), &work.lease, time(30000)).unwrap();
    now.set(time(30012));
    reader.read_exact_at(&mut [0; 6], 0).unwrap();
    other
        .begin_upload(&context(), upload_request("admission", 0), time(30012))
        .unwrap();
    assert_eq!(db.reserved(), 6);
    now.set(time(60000));
    assert_eq!(
        error_code(reader.read_at(&mut [0; 1], 0).unwrap_err()),
        FailureCode::ExecutionInterrupted
    );
    other
        .begin_upload(&context(), upload_request("reap", 0), time(60000))
        .unwrap();
    assert_eq!(db.reserved(), 0);
    assert_eq!(db.chunks(), 0);
    assert_eq!(
        other
            .get_job(&context(), work.lease.job_id(), time(60000))
            .unwrap()
            .state,
        JobState::Failed
    );
}

#[test]
fn authorization_executor_request_and_fence_are_bound() {
    let db = Database::new();
    let mut host = db.host();
    let work = work(&mut host);
    let mut wrong = context();
    wrong.principal = PrincipalId::new("other").unwrap();
    assert!(matches!(
        host.create_result(&wrong, &work.lease, spec("a", 10), &clock),
        Err(Failure {
            code: FailureCode::NotFound,
            ..
        })
    ));
    wrong = context();
    wrong.permissions.clear();
    assert!(matches!(
        host.create_result(&wrong, &work.lease, spec("a", 10), &clock),
        Err(Failure {
            code: FailureCode::NotAuthorized,
            ..
        })
    ));
    let mismatch = StandardHost::open(
        db.path(),
        Digest::from_sha256([9; 32]),
        HostLimits::default(),
    )
    .unwrap();
    assert!(matches!(
        mismatch.create_result(&context(), &work.lease, spec("a", 10), &clock),
        Err(Failure {
            code: FailureCode::ExecutorMismatch,
            ..
        })
    ));
    let original: String = db
        .sql()
        .query_row("SELECT info FROM jobs", [], |r| r.get(0))
        .unwrap();
    for field in ["fence", "requestDigest", "executorDigest"] {
        let mut sink = host
            .create_result(&context(), &work.lease, spec(field, 10), &clock)
            .unwrap();
        let mut changed: serde_json::Value = serde_json::from_str(&original).unwrap();
        changed[field] = serde_json::Value::String(if field == "fence" {
            "2".into()
        } else {
            Digest::from_sha256([7; 32]).to_string()
        });
        db.sql()
            .execute("UPDATE jobs SET info=?1", [changed.to_string()])
            .unwrap();
        assert_eq!(
            error_code(sink.write(b"x").unwrap_err()),
            FailureCode::StaleExecution
        );
        db.sql()
            .execute("UPDATE jobs SET info=?1", [&original])
            .unwrap();
        assert_eq!(
            error_code(sink.write(b"x").unwrap_err()),
            FailureCode::StaleExecution
        );
    }
}

#[test]
fn storage_failure_rolls_back_job_and_output_cleanup_together() {
    let db = Database::new();
    let mut host = db.host();
    let work = work(&mut host);
    let mut sink = host
        .create_result(&context(), &work.lease, spec("a", 100), &clock)
        .unwrap();
    sink.write_all(b"pending").unwrap();
    let reader = sink.seal().unwrap().reader;
    let mut other = db.host();
    db.sql().execute_batch("CREATE TRIGGER fail_result_delete BEFORE DELETE ON result_chunks BEGIN SELECT RAISE(ABORT,'injected'); END;").unwrap();
    assert_eq!(
        other
            .cancel_job(&context(), work.lease.job_id(), time(21))
            .unwrap_err()
            .code,
        FailureCode::StorageFailure
    );
    assert!(
        !other
            .get_job(&context(), work.lease.job_id(), time(21))
            .unwrap()
            .cancel_requested
    );
    assert_eq!(db.reserved(), 7);
    reader.read_exact_at(&mut [0; 7], 0).unwrap();
    // Advance time because reading with a regressed trusted clock is rejected.
    let reader = other
        .open_result(
            &context(),
            &work.lease,
            &RequestId::new("a").unwrap(),
            &|| time(22),
        )
        .unwrap();
    reader.read_exact_at(&mut [0; 7], 0).unwrap();
    drop(reader);
    db.sql()
        .execute_batch("DROP TRIGGER fail_result_delete;")
        .unwrap();
    other
        .cancel_job(&context(), work.lease.job_id(), time(23))
        .unwrap();
    assert_eq!(db.chunks(), 0);
    assert_eq!(db.reserved(), 0);
}

#[test]
fn successful_mutation_discards_unpublished_candidates_in_same_commit() {
    let db = Database::new();
    let mut host = db.host();
    let work = work(&mut host);
    let mut sink = host
        .create_result(&context(), &work.lease, spec("scratch", 10), &clock)
        .unwrap();
    sink.write_all(b"temp").unwrap();
    let sealed = sink.seal().unwrap();
    drop(sealed);
    let candidate = compute_mutation(&work.request, work.snapshot, &|| false).unwrap();
    let finished = host
        .finish(&context(), &work.lease, Ok(candidate), time(21))
        .unwrap();
    assert_eq!(finished.state, JobState::Succeeded);
    assert_eq!(db.chunks(), 0);
    assert_eq!(db.reserved(), 0);
    assert!(
        host.open_result(
            &context(),
            &work.lease,
            &RequestId::new("scratch").unwrap(),
            &|| time(22)
        )
        .is_err()
    );
}

#[test]
fn zero_length_clock_regression_and_count_quota_are_enforced() {
    let db = Database::new();
    let mut limits = HostLimits::default();
    limits.results.max_outputs_per_job = 2;
    let mut host = host(db.path(), limits);
    let work = work(&mut host);
    let empty = host
        .create_result(&context(), &work.lease, spec("empty", 0), &clock)
        .unwrap()
        .seal()
        .unwrap();
    assert_eq!(empty.byte_length, 0);
    assert_eq!(
        empty.reader.sha256(),
        &Digest::from_sha256(Sha256::digest([]).into())
    );
    assert_eq!(empty.reader.read_at(&mut [0; 1], 0).unwrap(), 0);
    let now = Cell::new(time(25));
    let clock = || now.get();
    let mut sink = host
        .create_result(&context(), &work.lease, spec("clock", 5), &clock)
        .unwrap();
    sink.write_all(b"x").unwrap();
    now.set(time(24));
    assert_eq!(
        error_code(sink.flush().unwrap_err()),
        FailureCode::InputInvalid
    );
    drop(sink);
    assert!(matches!(
        host.create_result(&context(), &work.lease, spec("third", 0), &|| time(26)),
        Err(Failure {
            code: FailureCode::LimitExceeded,
            ..
        })
    ));
}

#[test]
fn real_pptx_streams_from_persistent_inputs_into_verified_private_output() {
    let db = Database::new();
    let mut host = db.host();
    let fixture: serde_json::Value = serde_json::from_str(include_str!(
        "../../../fixtures/presentations/native-export/request.json"
    ))
    .unwrap();
    let document: mo_presentation_model::Document =
        serde_json::from_value(fixture["document"].clone()).unwrap();
    let defaults = serde_json::from_value(fixture["defaults"].clone()).unwrap();
    let bytes = include_bytes!("../../../fixtures/presentations/native-export/resources.bin");
    let mut bindings = Vec::new();
    for item in fixture["resourceBindings"].as_array().unwrap() {
        let id = mo_common::ResourceId::new(item["resourceId"].as_str().unwrap()).unwrap();
        let start: usize = item["byteOffset"].as_str().unwrap().parse().unwrap();
        let len: usize = item["byteLength"].as_str().unwrap().parse().unwrap();
        let data = &bytes[start..start + len];
        let mut request = upload_request(id.as_str(), len as u64);
        request.descriptor.sha256 = Digest::from_sha256(Sha256::digest(data).into());
        request.descriptor.media_type = document.resources[&id].media_type.clone();
        let upload = host.begin_upload(&context(), request, time(1)).unwrap();
        for (i, chunk) in data.chunks(ASSET_CHUNK_BYTES).enumerate() {
            host.append_upload(
                &context(),
                &upload.id,
                ByteLength::new((i * ASSET_CHUNK_BYTES) as u64),
                chunk,
                time(2),
            )
            .unwrap();
        }
        let asset = host
            .seal_upload(&context(), &upload.id, &|| time(3), &|| false)
            .unwrap()
            .asset
            .unwrap();
        bindings.push(AssetBinding {
            resource_id: id,
            asset_id: asset.id,
        });
    }
    let work = work(&mut host);
    let resources = host
        .bind_resources(&context(), &document, &bindings)
        .unwrap();
    let expected = mo_pptx::export(
        &document,
        &defaults,
        &resources,
        mo_pptx::PptxLimits::default(),
        &|| false,
    )
    .unwrap();
    let sink = host
        .create_result(&context(), &work.lease, spec("deck", 1_000_000), &clock)
        .unwrap();
    let verified = mo_pptx::export_to(
        &document,
        &defaults,
        &resources,
        sink,
        mo_pptx::PptxLimits::default(),
        &|| false,
    )
    .unwrap();
    assert_eq!(verified.receipt().byte_length, expected.len() as u64);
    assert_eq!(&verified.receipt().sha256, verified.reader().sha256());
    let mut actual = vec![0; expected.len()];
    verified.reader().read_exact_at(&mut actual, 0).unwrap();
    assert_eq!(actual, expected);
    assert_eq!(db.reserved(), expected.len() as i64);
    assert_eq!(
        db.value("SELECT count(*) FROM assets"),
        bindings.len() as i64
    );
    assert_eq!(
        db.host()
            .get_job(&context(), work.lease.job_id(), time(21))
            .unwrap()
            .state,
        JobState::Running
    );
}

#[test]
fn seal_releases_write_lock_and_cancellation_wins_during_hashing() {
    let db = Database::new();
    let mut host = db.host();
    let work = work(&mut host);
    let cancelled = Cell::new(false);
    let clock = || {
        let sealing:bool=db.sql().query_row("SELECT EXISTS(SELECT 1 FROM result_spools WHERE json_extract(info,'$.state')='Sealing')",[],|r|r.get(0)).unwrap();
        if sealing && !cancelled.replace(true) {
            db.host()
                .cancel_job(&context(), work.lease.job_id(), time(21))
                .unwrap();
        }
        time(if cancelled.get() { 21 } else { 20 })
    };
    let mut sink = host
        .create_result(
            &context(),
            &work.lease,
            spec("during-hash", 1_000_000),
            &clock,
        )
        .unwrap();
    sink.write_all(&vec![7; ASSET_CHUNK_BYTES + 1]).unwrap();
    assert_eq!(
        error_code(sink.seal().err().unwrap()),
        FailureCode::Cancelled
    );
    assert!(cancelled.get());
    assert_eq!(db.chunks(), 0);
    assert_eq!(db.reserved(), 0);
}

#[test]
fn final_seal_storage_failure_does_not_leave_a_readable_candidate() {
    let db = Database::new();
    let mut host = db.host();
    let work = work(&mut host);
    let mut sink = host
        .create_result(&context(), &work.lease, spec("commit", 100), &clock)
        .unwrap();
    sink.write_all(b"bytes").unwrap();
    db.sql().execute_batch("CREATE TRIGGER fail_seal BEFORE UPDATE ON result_spools WHEN json_extract(NEW.info,'$.state')='Sealed' BEGIN SELECT RAISE(ABORT,'injected'); END;").unwrap();
    assert_eq!(
        error_code(sink.seal().err().unwrap()),
        FailureCode::StorageFailure
    );
    assert_eq!(db.chunks(), 0);
    assert_eq!(db.reserved(), 0);
    assert!(
        host.open_result(
            &context(),
            &work.lease,
            &RequestId::new("commit").unwrap(),
            &clock
        )
        .is_err()
    );
}

#[test]
fn killed_writer_and_sealer_leave_private_recoverable_outputs() {
    use std::{
        process::{Command, Stdio},
        time::{Duration, Instant},
    };
    for phase in ["Writing", "Sealing"] {
        let db = Database::new();
        let ready = db.0.join("ready");
        let mut child = Command::new(std::env::current_exe().unwrap())
            .args(["--exact", "result_child", "--ignored", "--nocapture"])
            .env("MO_RESULT_TEST_DB", db.path())
            .env("MO_RESULT_TEST_READY", &ready)
            .env("MO_RESULT_TEST_PHASE", phase)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .unwrap();
        let deadline = Instant::now() + Duration::from_secs(10);
        while !ready.exists() {
            if let Some(status) = child.try_wait().unwrap() {
                panic!("child exited early: {status}");
            }
            if Instant::now() >= deadline {
                child.kill().unwrap();
                child.wait().unwrap();
                panic!("child never reached {phase}");
            }
            std::thread::sleep(Duration::from_millis(10));
        }
        child.kill().unwrap();
        assert!(!child.wait().unwrap().success());
        let state: String = db
            .sql()
            .query_row(
                "SELECT json_extract(info,'$.state') FROM result_spools",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(state, phase);
        assert!(db.chunks() > 0);
        assert!(db.reserved() > 0);
        assert_eq!(db.value("SELECT count(*) FROM assets"), 0);
        let id: String = db
            .sql()
            .query_row("SELECT id FROM jobs", [], |r| r.get(0))
            .unwrap();
        let job = db
            .host()
            .get_job(&context(), &JobId::new(id).unwrap(), time(40000))
            .unwrap();
        assert_eq!(job.state, JobState::Failed);
        assert!(matches!(
            job.result,
            Some(TerminalResult::Failed {
                error: Failure {
                    code: FailureCode::ExecutionInterrupted,
                    ..
                }
            })
        ));
        assert_eq!(db.chunks(), 0);
        assert_eq!(db.reserved(), 0);
    }
}

#[test]
#[ignore = "subprocess entry, executed and killed in both phases by parent test"]
fn result_child() {
    let path = PathBuf::from(std::env::var_os("MO_RESULT_TEST_DB").unwrap());
    let ready = PathBuf::from(std::env::var_os("MO_RESULT_TEST_READY").unwrap());
    let phase = std::env::var("MO_RESULT_TEST_PHASE").unwrap();
    let mut host = host(path.clone(), HostLimits::default());
    let work = work(&mut host);
    let stop = || {
        std::fs::write(&ready, b"ready").unwrap();
        loop {
            std::thread::park();
        }
    };
    let clock = || {
        if phase == "Sealing" {
            let sealing:bool=Connection::open(&path).unwrap().query_row("SELECT EXISTS(SELECT 1 FROM result_spools WHERE json_extract(info,'$.state')='Sealing')",[],|r|r.get(0)).unwrap();
            if sealing {
                stop();
            }
        }
        time(20)
    };
    let mut sink = host
        .create_result(&context(), &work.lease, spec("killed", 1_000_000), &clock)
        .unwrap();
    sink.write_all(&vec![11; ASSET_CHUNK_BYTES + 1]).unwrap();
    sink.flush().unwrap();
    if phase == "Writing" {
        stop();
    }
    sink.seal().unwrap();
    panic!("expected process kill");
}
