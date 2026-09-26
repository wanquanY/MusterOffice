//! Explicit integration with a real pinned worker, never a fabricated preview.
#[path = "exports/runtime.rs"]
mod runtime;
mod support;

use mo_common::{ByteLength, Digest, OperationId, RequestId};
use mo_native_render::NativePreviewRenderer;
use mo_opc::ReaderAt;
use mo_operation_service::*;
use mo_presentation_delivery::PreviewRenderer;
use mo_presentation_edit::{Operation, OperationEntry, SnapshotRecord};
use mo_standard_host::{HostLimits, StandardHost, WorkItem};
use rusqlite::Connection;
use sha2::{Digest as _, Sha256};
use std::{path::PathBuf, time::Duration};

struct Database(PathBuf);
impl Database {
    fn new() -> Self {
        Self(support::temporary_directory("mo-exports"))
    }
    fn path(&self) -> PathBuf {
        self.0.join("host.sqlite")
    }
    fn host(&self) -> StandardHost {
        StandardHost::open(
            self.path(),
            Digest::from_sha256([1; 32]),
            HostLimits::default(),
        )
        .unwrap()
    }
    fn sql(&self) -> Connection {
        Connection::open(self.path()).unwrap()
    }
    fn count(&self, query: &str) -> i64 {
        self.sql().query_row(query, [], |r| r.get(0)).unwrap()
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
fn clock() -> UnixMillis {
    time(100)
}
fn context() -> CallContext {
    CallContext {
        principal: PrincipalId::new("agent").unwrap(),
        scope: ScopeId::new("workspace").unwrap(),
        permissions: [
            Permission::Create,
            Permission::Edit,
            Permission::Export,
            Permission::ReadDocument,
            Permission::ReadJob,
            Permission::CancelJob,
            Permission::WriteAssets,
            Permission::ReadAssets,
        ]
        .into_iter()
        .collect(),
    }
}
fn renderer() -> NativePreviewRenderer {
    NativePreviewRenderer::new(
        std::env::var_os("MO_DELIVERY_WORKER")
            .expect("explicit worker")
            .into(),
        Digest::try_from(std::env::var("MO_DELIVERY_WORKER_SHA256").expect("explicit digest"))
            .unwrap(),
        Duration::from_secs(60),
    )
    .unwrap()
}
fn upload(host: &mut StandardHost, name: &str, bytes: &[u8], mime: &str) -> AssetId {
    let u = host
        .begin_upload(
            &context(),
            UploadRequest {
                request_id: RequestId::new(name).unwrap(),
                descriptor: AssetDescriptor {
                    sha256: Digest::from_sha256(Sha256::digest(bytes).into()),
                    byte_length: ByteLength::new(bytes.len() as u64),
                    media_type: mime.into(),
                },
            },
            time(1),
        )
        .unwrap();
    for (i, chunk) in bytes.chunks(ASSET_CHUNK_BYTES).enumerate() {
        host.append_upload(
            &context(),
            &u.id,
            ByteLength::new((i * ASSET_CHUNK_BYTES) as u64),
            chunk,
            time(2),
        )
        .unwrap();
    }
    host.seal_upload(&context(), &u.id, &|| time(3), &|| false)
        .unwrap()
        .asset
        .unwrap()
        .id
}
fn setup(host: &mut StandardHost) -> (SnapshotRecord, ExportSettings) {
    let input: serde_json::Value = serde_json::from_str(include_str!(
        "../../../fixtures/presentations/delivery/input.json"
    ))
    .unwrap();
    let request = OperationRequest {
        contract_version: ContractVersion::V1,
        request_id: RequestId::new("create").unwrap(),
        profile_id: OperationProfile::AuthorModel,
        output_mode: OutputMode::Sync,
        action: DocumentAction::Create {
            document: Box::new(serde_json::from_value(input["document"].clone()).unwrap()),
        },
    };
    let id = request.action.document_id().clone();
    let job = host.submit(&context(), request, time(4)).unwrap();
    assert_eq!(
        host.run_job(&context(), &job.id, time(5), &|| time(6), &|| false)
            .unwrap()
            .state,
        JobState::Succeeded
    );
    let image = upload(
        host,
        "image",
        include_bytes!("../../../fixtures/presentations/native-export/resources.bin"),
        "image/png",
    );
    let font = upload(
        host,
        "font",
        include_bytes!("../../../fixtures/fonts/owned.ttf"),
        "application/octet-stream",
    );
    (
        host.read_document(&context(), &id, None).unwrap(),
        ExportSettings {
            delivery: serde_json::from_value(input["settings"].clone()).unwrap(),
            resources: vec![AssetBinding {
                resource_id: mo_common::ResourceId::new("resource:checker").unwrap(),
                asset_id: image,
            }],
            font_asset_id: Some(font),
            renderer: renderer().identity(),
        },
    )
}
fn request(snapshot: &SnapshotRecord, settings: &ExportSettings, id: &str) -> OperationRequest {
    OperationRequest {
        contract_version: ContractVersion::V1,
        request_id: RequestId::new(id).unwrap(),
        profile_id: OperationProfile::ResourceDelivery,
        output_mode: OutputMode::Job,
        action: DocumentAction::Export {
            document_id: snapshot.document.id.clone(),
            base_revision: snapshot.revision.clone(),
            settings: Box::new(settings.clone()),
        },
    }
}
fn claim(host: &mut StandardHost, request: OperationRequest) -> WorkItem {
    let job = host.submit(&context(), request, time(20)).unwrap();
    assert_eq!(job.state, JobState::Queued);
    host.claim(&context(), &job.id, time(21)).unwrap().unwrap()
}
fn receipt(job: &JobInfo) -> &ExportReceipt {
    assert_eq!(job.state, JobState::Succeeded);
    match job.result.as_ref().unwrap() {
        TerminalResult::Succeeded { receipt } => match receipt.as_ref() {
            OperationReceipt::Export(r) => r,
            _ => panic!("wrong receipt"),
        },
        _ => panic!("failed export"),
    }
}
fn verify_assets(host: &StandardHost, r: &ExportReceipt) {
    assert_eq!(r.bundle.assets.len(), 12);
    for asset in &r.bundle.assets {
        let reader = host
            .open_asset(&context(), &AssetId::new(asset.id.as_str()).unwrap())
            .unwrap();
        assert_eq!(reader.info().descriptor.sha256, asset.sha256);
        let mut bytes = vec![0; asset.byte_length.get() as usize];
        reader.read_exact_at(&mut bytes, 0).unwrap();
        assert_eq!(
            Digest::from_sha256(Sha256::digest(bytes).into()),
            asset.sha256
        );
    }
}

#[test]
#[ignore = "requires pinned MO_DELIVERY_WORKER and SHA256"]
fn real_export_publishes_all_assets_atomically_and_reopens_without_renderer() {
    let db = Database::new();
    let mut host = db.host();
    let (snapshot, settings) = setup(&mut host);
    let query = request(&snapshot, &settings, "export");
    let work = claim(&mut host, query.clone());
    let mut renderer = renderer();
    let candidate = host
        .prepare_export(&context(), &work, &mut renderer, &clock, &|| false)
        .unwrap();
    assert_eq!(db.count("SELECT count(*) FROM result_assets"), 0);
    let id = AssetId::new(candidate.delivery().bundle().pptx_asset_id.as_str()).unwrap();
    assert!(host.open_asset(&context(), &id).is_err());
    let job = host
        .finish_export(&context(), &work.lease, Ok(candidate), time(101))
        .unwrap();
    verify_assets(&host, receipt(&job));
    assert_eq!(db.count("SELECT count(*) FROM result_assets"), 12);
    assert_eq!(
        db.count("SELECT count(*) FROM result_spools WHERE expires_at IS NOT NULL"),
        0
    );
    assert_eq!(db.count("SELECT count(*) FROM revisions"), 1);
    assert_eq!(
        db.count("SELECT sum(reserved_bytes) FROM result_spools"),
        receipt(&job)
            .bundle
            .assets
            .iter()
            .map(|a| a.byte_length.get() as i64)
            .sum::<i64>()
    );
    let retry = host.submit(&context(), query, time(102)).unwrap();
    assert_eq!(
        serde_json::to_value(&retry).unwrap(),
        serde_json::to_value(&job).unwrap()
    );
    drop(host);
    let mut reopened = db.host();
    verify_assets(&reopened, receipt(&job));
    assert_eq!(
        reopened
            .cancel_job(&context(), &job.id, time(1_000_000))
            .unwrap()
            .state,
        JobState::Succeeded
    );
    verify_assets(&reopened, receipt(&job));
    let mut denied = context();
    denied.scope = ScopeId::new("elsewhere").unwrap();
    assert_eq!(
        reopened.asset_info(&denied, &id).unwrap_err().code,
        FailureCode::NotFound
    );
}

#[test]
#[ignore = "requires pinned MO_DELIVERY_WORKER and SHA256"]
fn historical_revision_and_repeat_exports_preserve_head_and_reuse_public_bytes() {
    let db = Database::new();
    let mut host = db.host();
    let (snapshot, settings) = setup(&mut host);
    let work = claim(&mut host, request(&snapshot, &settings, "old"));
    let mut r = renderer();
    let candidate = host
        .prepare_export(&context(), &work, &mut r, &clock, &|| false)
        .unwrap();
    let mut other = db.host();
    let edit = OperationRequest {
        contract_version: ContractVersion::V1,
        request_id: RequestId::new("edit").unwrap(),
        profile_id: OperationProfile::AuthorModel,
        output_mode: OutputMode::Sync,
        action: DocumentAction::Apply {
            document_id: snapshot.document.id.clone(),
            base_revision: snapshot.revision.clone(),
            operations: vec![OperationEntry {
                operation_id: OperationId::new("title").unwrap(),
                operation: Operation::SetTitle {
                    title: "New head".into(),
                },
            }],
        },
    };
    let edit = other.submit(&context(), edit, time(110)).unwrap();
    assert_eq!(
        other
            .run_job(&context(), &edit.id, time(111), &|| time(112), &|| false)
            .unwrap()
            .state,
        JobState::Succeeded
    );
    let first = host
        .finish_export(&context(), &work.lease, Ok(candidate), time(120))
        .unwrap();
    assert_eq!(receipt(&first).revision, snapshot.revision);
    assert_eq!(
        host.read_document(&context(), &snapshot.document.id, None)
            .unwrap()
            .document
            .title,
        "New head"
    );
    let bytes = db.count("SELECT sum(reserved_bytes) FROM result_spools");
    let second_query = request(&snapshot, &settings, "repeat");
    let second = host.submit(&context(), second_query, time(130)).unwrap();
    host.set_preview_renderer(Box::new(renderer()));
    let second = host
        .run_job(&context(), &second.id, time(131), &|| time(140), &|| false)
        .unwrap();
    assert_eq!(
        serde_json::to_value(receipt(&first)).unwrap(),
        serde_json::to_value(receipt(&second)).unwrap()
    );
    assert_eq!(db.count("SELECT count(*) FROM result_assets"), 12);
    assert_eq!(
        db.count("SELECT sum(reserved_bytes) FROM result_spools"),
        bytes
    );
    assert_eq!(
        db.count(
            "SELECT count(*) FROM result_spools WHERE json_extract(info,'$.state')='Discarded'"
        ),
        12
    );
    verify_assets(&host, receipt(&second));
}

#[test]
#[ignore = "requires pinned MO_DELIVERY_WORKER and SHA256"]
fn cancellation_and_expiry_win_before_publication_and_revoke_private_readers() {
    for expired in [false, true] {
        let db = Database::new();
        let mut host = db.host();
        let (snapshot, settings) = setup(&mut host);
        let work = claim(&mut host, request(&snapshot, &settings, "cancel"));
        let mut r = renderer();
        let candidate = host
            .prepare_export(&context(), &work, &mut r, &clock, &|| false)
            .unwrap();
        let now = if expired { 40_000 } else { 101 };
        let mut other = db.host();
        if expired {
            other
                .get_job(&context(), work.lease.job_id(), time(now))
                .unwrap();
        } else {
            other
                .cancel_job(&context(), work.lease.job_id(), time(now))
                .unwrap();
        }
        assert!(
            candidate.delivery().artifacts()[0]
                .reader()
                .read_at(&mut [0; 1], 0)
                .is_err()
        );
        let job = host
            .finish_export(&context(), &work.lease, Ok(candidate), time(now + 1))
            .unwrap();
        assert_eq!(
            job.state,
            if expired {
                JobState::Failed
            } else {
                JobState::Cancelled
            }
        );
        assert_eq!(db.count("SELECT count(*) FROM result_assets"), 0);
        assert_eq!(db.count("SELECT count(*) FROM result_chunks"), 0);
        assert_eq!(db.count("SELECT sum(reserved_bytes) FROM result_spools"), 0);
    }
}

#[test]
#[ignore = "requires pinned MO_DELIVERY_WORKER and SHA256"]
fn publication_sql_failure_rolls_back_assets_state_and_receipt_together() {
    let db = Database::new();
    let mut host = db.host();
    let (snapshot, settings) = setup(&mut host);
    let work = claim(&mut host, request(&snapshot, &settings, "rollback"));
    let mut r = renderer();
    let candidate = host
        .prepare_export(&context(), &work, &mut r, &clock, &|| false)
        .unwrap();
    let before = db.count("SELECT sum(reserved_bytes) FROM result_spools");
    db.sql().execute_batch("CREATE TRIGGER fail_second_asset BEFORE INSERT ON result_assets WHEN (SELECT count(*) FROM result_assets)>=1 BEGIN SELECT RAISE(ABORT,'injected publish failure'); END;").unwrap();
    let error = host
        .finish_export(&context(), &work.lease, Ok(candidate), time(101))
        .unwrap_err();
    assert_eq!(error.code, FailureCode::StorageFailure);
    assert_eq!(db.count("SELECT count(*) FROM result_assets"), 0);
    assert_eq!(
        db.count(
            "SELECT count(*) FROM result_spools WHERE json_extract(info,'$.state')='Published'"
        ),
        0
    );
    assert_eq!(
        db.count("SELECT sum(reserved_bytes) FROM result_spools"),
        before
    );
    assert_eq!(
        host.get_job(&context(), work.lease.job_id(), time(102))
            .unwrap()
            .state,
        JobState::Running
    );
    host.cancel_job(&context(), work.lease.job_id(), time(103))
        .unwrap();
    let job = host
        .finish_export(
            &context(),
            &work.lease,
            Err(Failure::new(FailureCode::Cancelled, "cancel")),
            time(104),
        )
        .unwrap();
    assert_eq!(job.state, JobState::Cancelled);
    assert_eq!(db.count("SELECT count(*) FROM result_chunks"), 0);
}

#[test]
#[ignore = "requires pinned MO_DELIVERY_WORKER and SHA256"]
fn wrong_request_candidate_and_changed_permission_never_publish() {
    let db = Database::new();
    let mut host = db.host();
    let (snapshot, settings) = setup(&mut host);
    let a = claim(&mut host, request(&snapshot, &settings, "a"));
    let b = claim(&mut host, request(&snapshot, &settings, "b"));
    let mut r = renderer();
    let candidate = host
        .prepare_export(&context(), &a, &mut r, &clock, &|| false)
        .unwrap();
    let job = host
        .finish_export(&context(), &b.lease, Ok(candidate), time(101))
        .unwrap();
    assert_eq!(job.state, JobState::Failed);
    assert!(matches!(
        job.result,
        Some(TerminalResult::Failed {
            error: Failure {
                code: FailureCode::StaleExecution,
                ..
            }
        })
    ));
    assert_eq!(db.count("SELECT count(*) FROM result_assets"), 0);
    let c = claim(&mut host, request(&snapshot, &settings, "c"));
    let candidate = host
        .prepare_export(&context(), &c, &mut r, &clock, &|| false)
        .unwrap();
    let mut denied = context();
    denied.permissions.remove(&Permission::Export);
    assert_eq!(
        host.finish_export(&denied, &c.lease, Ok(candidate), time(101))
            .unwrap_err()
            .code,
        FailureCode::NotAuthorized
    );
    assert_eq!(db.count("SELECT count(*) FROM result_assets"), 0);
    for work in [&a, &c] {
        host.cancel_job(&context(), work.lease.job_id(), time(102))
            .unwrap();
    }
    assert_eq!(db.count("SELECT count(*) FROM result_chunks"), 0);
}

#[test]
#[ignore = "requires pinned MO_DELIVERY_WORKER and SHA256"]
fn invalid_bindings_and_real_page_failure_produce_no_public_assets() {
    for scenario in 0..3 {
        let db = Database::new();
        let mut host = db.host();
        let (snapshot, mut settings) = setup(&mut host);
        let code = match scenario {
            0 => {
                settings.renderer.implementation_sha256 = Digest::from_sha256([0; 32]);
                FailureCode::ExecutorMismatch
            }
            1 => {
                settings.resources.clear();
                FailureCode::ResourceIncomplete
            }
            2 => {
                settings.font_asset_id = None;
                FailureCode::RenderFailure
            }
            _ => unreachable!(),
        };
        let query = request(&snapshot, &settings, "invalid");
        let job = host.submit(&context(), query, time(20)).unwrap();
        host.set_preview_renderer(Box::new(renderer()));
        let failed = host
            .run_job(&context(), &job.id, time(21), &clock, &|| false)
            .unwrap();
        assert_eq!(failed.state, JobState::Failed);
        let Some(TerminalResult::Failed { error }) = failed.result else {
            panic!("missing failure");
        };
        assert_eq!(error.code, code);
        if scenario == 2 {
            assert!(error.detail.is_some());
        }
        assert_eq!(db.count("SELECT count(*) FROM result_assets"), 0);
        assert_eq!(db.count("SELECT count(*) FROM result_chunks"), 0);
        assert_eq!(
            db.count("SELECT coalesce(sum(reserved_bytes),0) FROM result_spools"),
            0
        );
    }
}

#[test]
#[ignore = "requires pinned MO_DELIVERY_WORKER and SHA256"]
fn renewal_during_computation_preserves_outputs_without_reviving_cancelled_or_expired_work() {
    use std::cell::{Cell, RefCell};
    for scenario in 0..4 {
        let db = Database::new();
        let mut host = db.host();
        let (snapshot, settings) = setup(&mut host);
        let work = claim(&mut host, request(&snapshot, &settings, "renew"));
        let observer = RefCell::new(db.host());
        let now = Cell::new(100);
        let checks = Cell::new(0);
        let triggered = Cell::new(false);
        let clock = || time(now.get());
        let check = || {
            checks.set(checks.get() + 1);
            // Advance an explicit host clock across the original lease while
            // exercising the real pipeline. No wall-clock sleep or mock page.
            if checks.get() <= 10 {
                now.set(now.get() + 6_000);
            }
            if scenario != 0
                && !triggered.get()
                && db.count("SELECT count(*) FROM result_chunks") > 0
            {
                triggered.set(true);
                match scenario {
                    1 => {
                        observer
                            .borrow_mut()
                            .cancel_job(&context(), work.lease.job_id(), clock())
                            .unwrap();
                    }
                    2 => now.set(now.get() + HostLimits::default().lease_ms),
                    3 => return true,
                    _ => unreachable!(),
                }
            }
            scenario == 3 && triggered.get()
        };
        let candidate = host.prepare_export(&context(), &work, &mut renderer(), &clock, &check);
        if scenario == 0 {
            assert!(candidate.is_ok());
            assert!(now.get() > 21 + HostLimits::default().lease_ms);
            let observed = observer
                .borrow_mut()
                .get_job(&context(), work.lease.job_id(), clock())
                .unwrap();
            assert_eq!(observed.state, JobState::Running);
            assert!(observed.lease_until.unwrap() > clock());
            assert_eq!(
                db.count("SELECT count(DISTINCT expires_at) FROM result_spools"),
                1
            );
        } else {
            assert!(triggered.get());
            assert!(candidate.is_err());
        }
        let job = host
            .finish_export(&context(), &work.lease, candidate, clock())
            .unwrap();
        if scenario == 0 {
            verify_assets(&host, receipt(&job));
        } else {
            let expected = if scenario == 2 {
                FailureCode::ExecutionInterrupted
            } else {
                FailureCode::Cancelled
            };
            let Some(TerminalResult::Failed { ref error }) = job.result else {
                panic!("missing terminal failure");
            };
            assert_eq!(error.code, expected);
            assert_eq!(db.count("SELECT count(*) FROM result_assets"), 0);
            assert_eq!(db.count("SELECT count(*) FROM result_chunks"), 0);
            assert_eq!(
                db.count("SELECT coalesce(sum(reserved_bytes),0) FROM result_spools"),
                0
            );
            let renewed = host.renew(&context(), &work.lease, clock()).unwrap();
            assert_eq!(
                serde_json::to_value(&renewed).unwrap(),
                serde_json::to_value(&job).unwrap()
            );
        }
    }
}

#[test]
#[ignore = "requires pinned MO_DELIVERY_WORKER and SHA256"]
fn smaller_host_asset_budget_is_shared_by_discovery_and_actual_export() {
    let db = Database::new();
    let mut limits = HostLimits::default();
    limits.assets.max_asset_bytes = 96 * 1024;
    let mut host = StandardHost::open(db.path(), Digest::from_sha256([1; 32]), limits).unwrap();
    let (snapshot, settings) = setup(&mut host);
    host.set_preview_renderer(Box::new(renderer()));
    let caps = host.capabilities(&context());
    let descriptor = caps
        .operations
        .iter()
        .find(|d| d.operation == ServiceOperation::Export)
        .unwrap();
    assert!(descriptor.available && descriptor.unavailable_reason.is_none());
    assert_eq!(caps.renderer.unwrap(), settings.renderer);
    assert_eq!(caps.limits.export.asset_bytes.get(), 96 * 1024);
    assert_eq!(caps.limits.export.model_bytes.get(), 96 * 1024);
    assert_eq!(caps.limits.export.font_bytes.get(), 96 * 1024);
    let job = host
        .submit(
            &context(),
            request(&snapshot, &settings, "small-budget"),
            time(20),
        )
        .unwrap();
    let done = host
        .run_job(&context(), &job.id, time(21), &clock, &|| false)
        .unwrap();
    verify_assets(&host, receipt(&done));
    assert!(
        receipt(&done)
            .bundle
            .assets
            .iter()
            .all(|a| a.byte_length.get() <= 96 * 1024)
    );
}
