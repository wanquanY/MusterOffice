mod support;

use mo_common::{ByteLength, Digest, RequestId};
use mo_opc::ReaderAt;
use mo_operation_service::*;
use mo_standard_host::{AssetLimits, HostLimits, StandardHost};
use rusqlite::Connection;
use sha2::{Digest as _, Sha256};
use std::{
    cell::Cell,
    path::PathBuf,
    sync::{Arc, Barrier},
};

struct Database(PathBuf);
impl Database {
    fn new() -> Self {
        Self(support::temporary_directory("mo-assets-test"))
    }
    fn path(&self) -> PathBuf {
        self.0.join("host.sqlite")
    }
    fn host(&self) -> StandardHost {
        self.with_limits(AssetLimits::default())
    }
    fn with_limits(&self, assets: AssetLimits) -> StandardHost {
        StandardHost::open(
            self.path(),
            Digest::from_sha256([1; 32]),
            HostLimits {
                assets,
                ..HostLimits::default()
            },
        )
        .unwrap()
    }
    fn sql(&self) -> Connection {
        Connection::open(self.path()).unwrap()
    }
    fn count(&self, table: &str) -> i64 {
        assert!(["assets", "asset_chunks", "asset_uploads"].contains(&table));
        self.sql()
            .query_row(&format!("SELECT count(*) FROM {table}"), [], |r| r.get(0))
            .unwrap()
    }
    fn reserved(&self) -> i64 {
        self.sql()
            .query_row(
                "SELECT coalesce(sum(reserved_bytes),0) FROM asset_uploads",
                [],
                |r| r.get(0),
            )
            .unwrap()
    }
}
impl Drop for Database {
    fn drop(&mut self) {
        std::fs::remove_dir_all(&self.0).unwrap();
    }
}
fn context() -> CallContext {
    CallContext {
        principal: PrincipalId::new("agent:one").unwrap(),
        scope: ScopeId::new("scope:one").unwrap(),
        permissions: [Permission::WriteAssets, Permission::ReadAssets]
            .into_iter()
            .collect(),
    }
}
fn time(n: i64) -> UnixMillis {
    UnixMillis::new(n).unwrap()
}
fn bytes(n: usize) -> Vec<u8> {
    (0..n)
        .map(|i| ((i as u64 * 17 + i as u64 / 251) % 256) as u8)
        .collect()
}
fn request(id: &str, bytes: &[u8]) -> UploadRequest {
    UploadRequest {
        request_id: RequestId::new(id).unwrap(),
        descriptor: AssetDescriptor {
            sha256: Digest::from_sha256(Sha256::digest(bytes).into()),
            byte_length: ByteLength::new(bytes.len() as u64),
            media_type: "application/octet-stream".into(),
        },
    }
}
fn write(host: &mut StandardHost, request: UploadRequest, bytes: &[u8]) -> UploadInfo {
    let mut info = host.begin_upload(&context(), request, time(10)).unwrap();
    for (index, chunk) in bytes.chunks(ASSET_CHUNK_BYTES).enumerate() {
        info = host
            .append_upload(
                &context(),
                &info.id,
                ByteLength::new((index * ASSET_CHUNK_BYTES) as u64),
                chunk,
                time(20),
            )
            .unwrap();
    }
    info
}
fn sealed(host: &mut StandardHost, id: &str, bytes: &[u8]) -> UploadInfo {
    let upload = write(host, request(id, bytes), bytes);
    let sealed = host
        .seal_upload(&context(), &upload.id, &|| time(30), &|| false)
        .unwrap();
    assert_eq!(sealed.state, UploadState::Sealed);
    sealed
}

#[test]
fn persisted_chunks_are_immutable_range_readable_and_replayed_after_restart() {
    let db = Database::new();
    let mut host = db.host();
    let input = bytes(ASSET_CHUNK_BYTES * 3 + 19);
    let q = request("one", &input);
    let upload = write(&mut host, q.clone(), &input);
    assert!(upload.asset.is_none());
    assert_eq!(db.count("assets"), 0);
    assert_eq!(db.count("asset_chunks"), 4);
    drop(host);
    let mut host = db.host();
    assert_eq!(
        host.begin_upload(&context(), q.clone(), time(21)).unwrap(),
        upload
    );
    let result = host
        .seal_upload(&context(), &upload.id, &|| time(30), &|| false)
        .unwrap();
    let asset = result.asset.as_ref().unwrap();
    assert_eq!(result.state, UploadState::Sealed);
    assert_eq!(asset.descriptor, q.descriptor);
    let reader = host.open_asset(&context(), &asset.id).unwrap();
    for (offset, n) in [
        (0, 1),
        (255, 101),
        (ASSET_CHUNK_BYTES - 7, 35),
        (ASSET_CHUNK_BYTES * 3, 19),
        (0, input.len()),
    ] {
        let mut out = vec![0; n];
        reader.read_exact_at(&mut out, offset as u64).unwrap();
        assert_eq!(out, input[offset..offset + n]);
    }
    assert_eq!(reader.read_at(&mut [0; 1], u64::MAX).unwrap(), 0);
    assert!(
        reader
            .read_exact_at(&mut [0; 20], (input.len() - 19) as u64)
            .is_err()
    );
    assert_eq!(db.reserved(), input.len() as i64);
    assert_eq!(
        host.cancel_upload(&context(), &upload.id, time(31))
            .unwrap(),
        result
    );
    assert_eq!(
        host.seal_upload(&context(), &upload.id, &|| time(0), &|| false)
            .unwrap(),
        result
    );
    assert_eq!(host.begin_upload(&context(), q, time(0)).unwrap(), result);
}

#[test]
fn chunks_retry_exactly_but_gaps_partial_blocks_and_content_changes_fail() {
    let db = Database::new();
    let mut host = db.host();
    let input = bytes(ASSET_CHUNK_BYTES + 13);
    let q = request("one", &input);
    let upload = host.begin_upload(&context(), q.clone(), time(10)).unwrap();
    assert_eq!(
        host.append_upload(
            &context(),
            &upload.id,
            ByteLength::new(ASSET_CHUNK_BYTES as u64),
            &input[ASSET_CHUNK_BYTES..],
            time(11)
        )
        .unwrap_err()
        .code,
        FailureCode::ResourceConflict
    );
    assert_eq!(
        host.append_upload(
            &context(),
            &upload.id,
            ByteLength::new(0),
            &input[..8],
            time(11)
        )
        .unwrap_err()
        .code,
        FailureCode::ResourceConflict
    );
    assert_eq!(
        host.seal_upload(&context(), &upload.id, &|| time(11), &|| false)
            .unwrap_err()
            .code,
        FailureCode::ResourceIncomplete
    );
    let first = host
        .append_upload(
            &context(),
            &upload.id,
            ByteLength::new(0),
            &input[..ASSET_CHUNK_BYTES],
            time(12),
        )
        .unwrap();
    assert_eq!(
        host.append_upload(
            &context(),
            &upload.id,
            ByteLength::new(0),
            &input[..ASSET_CHUNK_BYTES],
            time(13)
        )
        .unwrap(),
        first
    );
    let mut wrong = input[..ASSET_CHUNK_BYTES].to_vec();
    wrong[0] ^= 1;
    assert_eq!(
        host.append_upload(&context(), &upload.id, ByteLength::new(0), &wrong, time(13))
            .unwrap_err()
            .code,
        FailureCode::ResourceConflict
    );
    host.append_upload(
        &context(),
        &upload.id,
        ByteLength::new(ASSET_CHUNK_BYTES as u64),
        &input[ASSET_CHUNK_BYTES..],
        time(14),
    )
    .unwrap();
    let final_info = host
        .seal_upload(&context(), &upload.id, &|| time(15), &|| false)
        .unwrap();
    assert_eq!(
        host.append_upload(
            &context(),
            &upload.id,
            ByteLength::new(0),
            &input[..ASSET_CHUNK_BYTES],
            time(16)
        )
        .unwrap(),
        final_info
    );
    assert_eq!(
        host.append_upload(&context(), &upload.id, ByteLength::new(0), &wrong, time(16))
            .unwrap_err()
            .code,
        FailureCode::ResourceConflict
    );
    let mut changed = q;
    changed.descriptor.media_type = "image/png".into();
    assert_eq!(
        host.begin_upload(&context(), changed, time(17))
            .unwrap_err()
            .code,
        FailureCode::RequestIdReused
    );
    assert_eq!(db.count("asset_chunks"), 2);
}

#[test]
fn digest_mismatch_and_every_cancellation_checkpoint_release_staging() {
    let input = bytes(ASSET_CHUNK_BYTES * 2 + 9);
    let db = Database::new();
    let mut host = db.host();
    let mut q = request("bad", &input);
    q.descriptor.sha256 = Digest::from_sha256([0; 32]);
    let upload = write(&mut host, q.clone(), &input);
    let result = host
        .seal_upload(&context(), &upload.id, &|| time(30), &|| false)
        .unwrap();
    assert_eq!(result.state, UploadState::Failed);
    assert_eq!(
        result.error.as_ref().unwrap().code,
        FailureCode::ResourceConflict
    );
    assert_eq!(
        (db.count("assets"), db.count("asset_chunks"), db.reserved()),
        (0, 0, 0)
    );
    assert_eq!(host.begin_upload(&context(), q, time(31)).unwrap(), result);
    for stop in 0..4 {
        let upload = write(
            &mut host,
            request(&format!("cancel-{stop}"), &input),
            &input,
        );
        let calls = Cell::new(0);
        let result = host
            .seal_upload(&context(), &upload.id, &|| time(30), &|| {
                let i = calls.get();
                calls.set(i + 1);
                i == stop
            })
            .unwrap();
        assert_eq!(result.state, UploadState::Cancelled);
        assert_eq!(db.reserved(), 0);
        assert_eq!(db.count("asset_chunks"), 0);
    }
    assert_eq!(
        sealed(&mut host, "recovery", &input).state,
        UploadState::Sealed
    );
}

#[test]
fn permission_and_scope_are_not_inferred_from_a_digest_or_handle() {
    let db = Database::new();
    let mut host = db.host();
    let result = sealed(&mut host, "one", b"payload");
    let asset = result.asset.unwrap();
    let mut denied = context();
    denied.permissions.clear();
    assert_eq!(
        host.begin_upload(&denied, request("one", b"payload"), time(40))
            .unwrap_err()
            .code,
        FailureCode::NotAuthorized
    );
    assert!(matches!(
        host.open_asset(&denied, &asset.id),
        Err(Failure {
            code: FailureCode::NotAuthorized,
            ..
        })
    ));
    let mut other = context();
    other.principal = PrincipalId::new("another-agent").unwrap();
    assert_eq!(
        host.get_upload(&other, &result.id, time(40))
            .unwrap_err()
            .code,
        FailureCode::NotFound
    );
    assert_eq!(host.asset_info(&other, &asset.id).unwrap(), asset);
    other.scope = ScopeId::new("another-scope").unwrap();
    assert_eq!(
        host.asset_info(&other, &asset.id).unwrap_err().code,
        FailureCode::NotFound
    );
    let guessed = AssetId::new(asset.descriptor.sha256.as_str()).unwrap();
    assert_eq!(
        host.asset_info(&context(), &guessed).unwrap_err().code,
        FailureCode::NotFound
    );
}

#[test]
fn quota_reservations_are_atomic_and_expiry_or_cancel_releases_bytes() {
    let db = Database::new();
    let limits = AssetLimits {
        max_asset_bytes: 100,
        max_scope_bytes: 100,
        max_uploads_per_scope: 4,
        upload_ttl_ms: 100,
    };
    let mut host = db.with_limits(limits);
    let first = host
        .begin_upload(&context(), request("one", &bytes(70)), time(10))
        .unwrap();
    assert_eq!(db.reserved(), 70);
    assert_eq!(
        host.begin_upload(&context(), request("too-many", &bytes(40)), time(11))
            .unwrap_err()
            .code,
        FailureCode::LimitExceeded
    );
    host.cancel_upload(&context(), &first.id, time(12)).unwrap();
    assert_eq!(db.reserved(), 0);
    let abandoned = host
        .begin_upload(&context(), request("abandoned", &bytes(100)), time(20))
        .unwrap();
    host.append_upload(
        &context(),
        &abandoned.id,
        ByteLength::new(0),
        &bytes(100),
        time(21),
    )
    .unwrap();
    let next = host
        .begin_upload(&context(), request("next", &bytes(80)), time(121))
        .unwrap();
    assert_eq!(next.state, UploadState::Uploading);
    assert_eq!(db.reserved(), 80);
    assert_eq!(db.count("asset_chunks"), 0);
    let expired = host
        .get_upload(&context(), &abandoned.id, time(122))
        .unwrap();
    assert_eq!(expired.error.unwrap().code, FailureCode::ResourceExpired);
    assert_eq!(host.reap_uploads(&context(), time(221)).unwrap(), 1);
    assert_eq!(db.reserved(), 0);
    host.begin_upload(&context(), request("fourth", b""), time(222))
        .unwrap();
    assert_eq!(
        host.begin_upload(&context(), request("fifth", b""), time(223))
            .unwrap_err()
            .code,
        FailureCode::LimitExceeded
    );
}

#[test]
fn concurrent_admission_and_same_request_retry_do_not_overreserve() {
    let db = Database::new();
    let limits = AssetLimits {
        max_asset_bytes: 100,
        max_scope_bytes: 100,
        ..AssetLimits::default()
    };
    let _host = db.with_limits(limits);
    let barrier = Arc::new(Barrier::new(2));
    let results = std::thread::scope(|scope| {
        let handles: Vec<_> = ["one", "two"]
            .into_iter()
            .map(|id| {
                let mut host = db.with_limits(limits);
                let b = barrier.clone();
                scope.spawn(move || {
                    b.wait();
                    host.begin_upload(&context(), request(id, &bytes(60)), time(10))
                })
            })
            .collect();
        handles
            .into_iter()
            .map(|h| h.join().unwrap())
            .collect::<Vec<_>>()
    });
    assert_eq!(results.iter().filter(|r| r.is_ok()).count(), 1);
    assert_eq!(db.reserved(), 60);
    assert_eq!(db.count("asset_uploads"), 1);
    let winner = results.into_iter().find_map(Result::ok).unwrap();
    let mut host = db.with_limits(limits);
    assert_eq!(
        host.begin_upload(
            &context(),
            UploadRequest {
                request_id: winner.request_id.clone(),
                descriptor: winner.descriptor.clone()
            },
            time(11)
        )
        .unwrap(),
        winner
    );
}

#[test]
fn hashing_releases_write_lock_and_cancel_wins_before_publication() {
    let db = Database::new();
    let mut host = db.host();
    let upload = write(&mut host, request("one", b"bytes"), b"bytes");
    let invoked = Cell::new(false);
    let result = host
        .seal_upload(&context(), &upload.id, &|| time(30), &|| {
            if !invoked.replace(true) {
                let mut other = db.host();
                let cancelled = other
                    .cancel_upload(&context(), &upload.id, time(30))
                    .unwrap();
                assert_eq!(cancelled.state, UploadState::Cancelled);
                // A write from a different connection while hashing would time out
                // if the verifier retained the database write lock.
                assert!(
                    other
                        .begin_upload(&context(), request("another", b""), time(30))
                        .is_ok()
                );
            }
            false
        })
        .unwrap();
    assert_eq!(result.state, UploadState::Cancelled);
    assert_eq!(db.count("assets"), 0);
    assert_eq!(db.count("asset_chunks"), 0);
}

#[test]
fn failure_on_final_receipt_update_cannot_publish_an_asset() {
    let db = Database::new();
    let mut host = db.host();
    let upload = write(&mut host, request("one", b"bytes"), b"bytes");
    let hooked = Cell::new(false);
    let failure=host.seal_upload(&context(),&upload.id,&||time(30),&|| {
        if !hooked.replace(true){db.sql().execute_batch("CREATE TRIGGER failing_seal BEFORE UPDATE ON asset_uploads BEGIN SELECT RAISE(ABORT,'failed last write'); END;").unwrap();}false
    }).unwrap_err();
    assert_eq!(failure.code, FailureCode::StorageFailure);
    assert_eq!(db.count("assets"), 0);
    db.sql()
        .execute_batch("DROP TRIGGER failing_seal;")
        .unwrap();
    assert_eq!(
        host.get_upload(&context(), &upload.id, time(31))
            .unwrap()
            .state,
        UploadState::Verifying
    );
    assert_eq!(
        host.seal_upload(&context(), &upload.id, &|| time(31), &|| false)
            .unwrap_err()
            .code,
        FailureCode::ResourceBusy
    );
    assert_eq!(
        host.get_upload(&context(), &upload.id, time(900030))
            .unwrap()
            .state,
        UploadState::Failed
    );
    assert_eq!(db.reserved(), 0);
    assert_eq!(db.count("asset_chunks"), 0);
}

#[test]
fn range_reader_detects_changed_or_missing_persisted_chunks() {
    let db = Database::new();
    let mut host = db.host();
    let result = sealed(&mut host, "one", b"bytes");
    let asset = result.asset.unwrap();
    db.sql()
        .execute_batch("UPDATE asset_chunks SET data=x'0102030405';")
        .unwrap();
    let reader = host.open_asset(&context(), &asset.id).unwrap();
    assert!(reader.read_exact_at(&mut [0; 5], 0).is_err());
    db.sql().execute_batch("DELETE FROM asset_chunks;").unwrap();
    assert!(reader.read_exact_at(&mut [0; 1], 0).is_err());
}

#[test]
fn zero_length_clock_regression_overflow_and_invalid_metadata_are_explicit() {
    let db = Database::new();
    let mut host = db.host();
    let upload = host
        .begin_upload(&context(), request("empty", b""), time(10))
        .unwrap();
    assert_eq!(
        host.get_upload(&context(), &upload.id, time(9))
            .unwrap_err()
            .code,
        FailureCode::InputInvalid
    );
    let result = host
        .seal_upload(&context(), &upload.id, &|| time(11), &|| false)
        .unwrap();
    assert_eq!(result.state, UploadState::Sealed);
    assert_eq!(
        host.open_asset(&context(), &result.asset.unwrap().id)
            .unwrap()
            .read_at(&mut [0; 1], 0)
            .unwrap(),
        0
    );
    assert_eq!(db.count("asset_chunks"), 0);
    let mut q = request("overflow", b"");
    q.descriptor.byte_length = ByteLength::new(u64::MAX);
    assert_eq!(
        host.begin_upload(&context(), q, time(12)).unwrap_err().code,
        FailureCode::LimitExceeded
    );
    for mime in [
        "no-slash",
        "image/",
        "/png",
        "image/png extra",
        "image/\n",
        "image/png/extra",
        "图像/png",
    ] {
        let mut q = request("mime", b"");
        q.descriptor.media_type = mime.into();
        assert_eq!(
            host.begin_upload(&context(), q, time(12)).unwrap_err().code,
            FailureCode::InputInvalid
        );
    }
    assert_eq!(
        host.begin_upload(&context(), request("clock", b""), time(i64::MAX))
            .unwrap_err()
            .code,
        FailureCode::LimitExceeded
    );
}

#[test]
fn metadata_query_preserves_failed_upload_state_without_claiming_usable_bytes() {
    let db = Database::new();
    let mut host = db.host();
    let info = host
        .begin_upload(&context(), request("cancel", b"bytes"), time(10))
        .unwrap();
    let cancelled = host.cancel_upload(&context(), &info.id, time(11)).unwrap();
    let response = host.dispatch(
        &context(),
        HostRequest::GetUpload { upload_id: info.id },
        &|| time(12),
        &|| false,
    );
    let HostResponse::Succeeded {
        result: HostResult::Upload { upload },
    } = response
    else {
        panic!("metadata query failed");
    };
    assert_eq!(*upload, cancelled);
    assert_eq!(upload.state, UploadState::Cancelled);
    assert!(upload.asset.is_none());
    assert_eq!(upload.error.as_ref().unwrap().code, FailureCode::Cancelled);
}

#[test]
fn native_pptx_writer_consumes_scope_authorized_range_readers() {
    use mo_pptx::{ExportDefaults, PptxLimits};
    use mo_presentation_model::Document;
    let db = Database::new();
    let mut host = db.host();
    let fixture: serde_json::Value = serde_json::from_str(include_str!(
        "../../../fixtures/presentations/native-export/request.json"
    ))
    .unwrap();
    let document: Document = serde_json::from_value(fixture["document"].clone()).unwrap();
    let defaults: ExportDefaults = serde_json::from_value(fixture["defaults"].clone()).unwrap();
    let bundle = include_bytes!("../../../fixtures/presentations/native-export/resources.bin");
    let mut bindings = Vec::new();
    for value in fixture["resourceBindings"].as_array().unwrap() {
        let id = mo_common::ResourceId::new(value["resourceId"].as_str().unwrap()).unwrap();
        let offset: usize = value["byteOffset"].as_str().unwrap().parse().unwrap();
        let n: usize = value["byteLength"].as_str().unwrap().parse().unwrap();
        let bytes = &bundle[offset..offset + n];
        let mut q = request(id.as_str(), bytes);
        q.descriptor.media_type = document.resources[&id].media_type.clone();
        let upload = write(&mut host, q, bytes);
        let asset = host
            .seal_upload(&context(), &upload.id, &|| time(30), &|| false)
            .unwrap()
            .asset
            .unwrap();
        bindings.push(AssetBinding {
            resource_id: id,
            asset_id: asset.id,
        });
    }
    drop(host);
    let host = db.host();
    let resources = host
        .bind_resources(&context(), &document, &bindings)
        .unwrap();
    let output = mo_pptx::export(
        &document,
        &defaults,
        &resources,
        PptxLimits::default(),
        &|| false,
    )
    .unwrap();
    let package = mo_opc::Package::open(
        output.as_slice(),
        output.len() as u64,
        mo_opc::PackageLimits::default(),
        &|| false,
    )
    .unwrap();
    let images: Vec<_> = package
        .parts()
        .values()
        .filter(|p| p.content_type.starts_with("image/"))
        .collect();
    assert!(!images.is_empty());
    for part in images {
        assert!(document.resources.values().any(|r| r.sha256 == part.sha256));
    }
    let spool_root = db.0.join("output-staging");
    std::fs::create_dir(&spool_root).unwrap();
    let streamed = mo_pptx::export_to(
        &document,
        &defaults,
        &resources,
        mo_standard_host::FileSpool::create(&spool_root, 1_000_000).unwrap(),
        PptxLimits::default(),
        &|| false,
    )
    .unwrap();
    assert_eq!(streamed.receipt().sha256, *package.sha256());
    assert_eq!(streamed.receipt().byte_length, output.len() as u64);
    let mut actual = vec![0; output.len()];
    streamed.reader().read_exact_at(&mut actual, 0).unwrap();
    assert_eq!(actual, output);
    drop(streamed);
    assert_eq!(std::fs::read_dir(&spool_root).unwrap().count(), 0);
    let mut wrong = document.clone();
    wrong.resources.values_mut().next().unwrap().sha256 = Digest::from_sha256([0; 32]);
    assert!(matches!(
        host.bind_resources(&context(), &wrong, &bindings),
        Err(Failure {
            code: FailureCode::ResourceConflict,
            ..
        })
    ));
    assert!(matches!(
        host.bind_resources(&context(), &document, &[]),
        Err(Failure {
            code: FailureCode::ResourceIncomplete,
            ..
        })
    ));
}
