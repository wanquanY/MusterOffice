mod support;

use mo_common::{ByteLength, Digest, RequestId};
use mo_operation_service::*;
use mo_standard_host::{AssetLimits, HostLimits, StandardHost};
use rusqlite::Connection;
use sha2::{Digest as _, Sha256};
use std::{
    path::PathBuf,
    process::{Command, Stdio},
    time::{Duration, Instant},
};

fn context() -> CallContext {
    CallContext {
        scope: ScopeId::new("scope").unwrap(),
        principal: PrincipalId::new("principal").unwrap(),
        permissions: [Permission::WriteAssets, Permission::ReadAssets]
            .into_iter()
            .collect(),
    }
}
fn time(n: i64) -> UnixMillis {
    UnixMillis::new(n).unwrap()
}
fn host(path: PathBuf) -> StandardHost {
    StandardHost::open(
        path,
        Digest::from_sha256([0; 32]),
        HostLimits {
            assets: AssetLimits {
                upload_ttl_ms: 100,
                ..AssetLimits::default()
            },
            ..HostLimits::default()
        },
    )
    .unwrap()
}
struct Directory(PathBuf);
impl Directory {
    fn new() -> Self {
        Self(support::temporary_directory("mo-resource-recovery"))
    }
    fn db(&self) -> PathBuf {
        self.0.join("host.sqlite")
    }
}
impl Drop for Directory {
    fn drop(&mut self) {
        std::fs::remove_dir_all(&self.0).unwrap();
    }
}

#[test]
fn killed_verifier_is_never_published_and_expiry_releases_staging() {
    let dir = Directory::new();
    let mut host = host(dir.db());
    let bytes = b"sealed bytes must come after verification";
    let request = UploadRequest {
        request_id: RequestId::new("resource").unwrap(),
        descriptor: AssetDescriptor {
            sha256: Digest::from_sha256(Sha256::digest(bytes).into()),
            byte_length: ByteLength::new(bytes.len() as u64),
            media_type: "application/octet-stream".into(),
        },
    };
    let upload = host
        .begin_upload(&context(), request.clone(), time(10))
        .unwrap();
    host.append_upload(&context(), &upload.id, ByteLength::new(0), bytes, time(20))
        .unwrap();
    let ready = dir.0.join("ready");
    let mut child = Command::new(std::env::current_exe().unwrap())
        .args(["--exact", "verifier_child", "--ignored", "--nocapture"])
        .env("MO_RESOURCE_TEST_DB", dir.db())
        .env("MO_RESOURCE_TEST_READY", &ready)
        .env("MO_RESOURCE_TEST_UPLOAD", upload.id.as_str())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    let deadline = Instant::now() + Duration::from_secs(10);
    while !ready.exists() && Instant::now() < deadline {
        if child.try_wait().unwrap().is_some() {
            panic!("verifier exited before durable freeze");
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    let ready = ready.exists();
    child.kill().unwrap();
    child.wait().unwrap();
    assert!(ready);
    assert_eq!(
        host.get_upload(&context(), &upload.id, time(51))
            .unwrap()
            .state,
        UploadState::Verifying
    );
    assert_eq!(
        host.seal_upload(&context(), &upload.id, &|| time(52), &|| false)
            .unwrap_err()
            .code,
        FailureCode::ResourceBusy
    );
    assert_eq!(host.reap_uploads(&context(), time(150)).unwrap(), 1);
    let failed = host.get_upload(&context(), &upload.id, time(151)).unwrap();
    assert_eq!(
        failed.error.as_ref().unwrap().code,
        FailureCode::ResourceExpired
    );
    assert_eq!(
        host.begin_upload(&context(), request, time(152)).unwrap(),
        failed
    );
    let db = Connection::open(dir.db()).unwrap();
    for table in ["assets", "asset_chunks"] {
        assert_eq!(
            db.query_row(&format!("SELECT count(*) FROM {table}"), [], |r| r
                .get::<_, i64>(0))
                .unwrap(),
            0
        );
    }
    assert_eq!(
        db.query_row("SELECT sum(reserved_bytes) FROM asset_uploads", [], |r| r
            .get::<_, i64>(
            0
        ))
        .unwrap(),
        0
    );
}

#[test]
#[ignore = "test-only subprocess entry, executed and killed by parent test"]
fn verifier_child() {
    let mut host = host(PathBuf::from(
        std::env::var_os("MO_RESOURCE_TEST_DB").unwrap(),
    ));
    let id = UploadId::new(std::env::var("MO_RESOURCE_TEST_UPLOAD").unwrap()).unwrap();
    let ready = std::env::var_os("MO_RESOURCE_TEST_READY").unwrap();
    host.seal_upload(&context(), &id, &|| time(50), &|| {
        std::fs::write(&ready, b"frozen").unwrap();
        loop {
            std::thread::park();
        }
    })
    .unwrap();
}

#[test]
fn v1_migration_is_atomic_idempotent_and_preserves_existing_rows() {
    let dir = Directory::new();
    let db = Connection::open(dir.db()).unwrap();
    // Preserve a real serialized receipt in the exact three-table v1 shape.
    // Queue expression indexes require valid JSON; arbitrary text is corruption.
    let source = Directory::new();
    let mut source_host = host(source.db());
    let value: serde_json::Value = serde_json::from_str(include_str!(
        "../../../fixtures/presentations/native-export/request.json"
    ))
    .unwrap();
    let mut writer = context();
    writer.permissions.insert(Permission::Create);
    source_host
        .submit(
            &writer,
            OperationRequest {
                contract_version: ContractVersion::V1,
                request_id: RequestId::new("preserve").unwrap(),
                profile_id: OperationProfile::AuthorModel,
                output_mode: OutputMode::Job,
                action: DocumentAction::Create {
                    document: Box::new(serde_json::from_value(value["document"].clone()).unwrap()),
                },
            },
            time(10),
        )
        .unwrap();
    let source_db = Connection::open(source.db()).unwrap();
    let row: [String; 7] = source_db
        .query_row(
            "SELECT scope,principal,id,operation,request_id,request,info FROM jobs",
            [],
            |r| {
                Ok([
                    r.get(0)?,
                    r.get(1)?,
                    r.get(2)?,
                    r.get(3)?,
                    r.get(4)?,
                    r.get(5)?,
                    r.get(6)?,
                ])
            },
        )
        .unwrap();
    db.execute_batch("CREATE TABLE jobs (scope TEXT NOT NULL, principal TEXT NOT NULL, id TEXT NOT NULL, operation TEXT NOT NULL, request_id TEXT NOT NULL, request TEXT NOT NULL, info TEXT NOT NULL, PRIMARY KEY(scope,principal,id), UNIQUE(scope,principal,operation,request_id)) STRICT;
CREATE TABLE revisions(scope TEXT NOT NULL, document_id TEXT NOT NULL, revision TEXT NOT NULL, semantic_digest TEXT NOT NULL, snapshot TEXT NOT NULL, PRIMARY KEY(scope,document_id,revision)) STRICT;
CREATE TABLE heads(scope TEXT NOT NULL, document_id TEXT NOT NULL, revision TEXT NOT NULL, PRIMARY KEY(scope,document_id), FOREIGN KEY(scope,document_id,revision) REFERENCES revisions(scope,document_id,revision)) STRICT;
PRAGMA application_id=1297041478; PRAGMA user_version=1;").unwrap();
    db.execute(
        "INSERT INTO jobs VALUES(?1,?2,?3,?4,?5,?6,?7)",
        rusqlite::params_from_iter(row.iter()),
    )
    .unwrap();
    let _first = host(dir.db());
    let _second = host(dir.db());
    assert_eq!(
        db.query_row("PRAGMA user_version", [], |r| r.get::<_, i64>(0))
            .unwrap(),
        5
    );
    let pair: (String, String) = db
        .query_row("SELECT request,info FROM jobs", [], |r| {
            Ok((r.get(0)?, r.get(1)?))
        })
        .unwrap();
    assert_eq!(pair, (row[5].clone(), row[6].clone()));
    let tables: i64 = db
        .query_row(
            "SELECT count(*) FROM sqlite_schema WHERE type='table'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(tables, 9);
}
