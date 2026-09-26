mod support;

use mo_common::{ByteLength, Digest, RequestId};
use mo_opc::ReaderAt;
use mo_operation_service::*;
use mo_standard_host::{HostLimits, StandardHost};
use sha2::{Digest as _, Sha256};

struct Database(std::path::PathBuf);
impl Database {
    fn new() -> Self {
        Self(support::temporary_directory("mo-discovery"))
    }
    fn host(&self) -> StandardHost {
        StandardHost::open(
            self.0.join("host.sqlite"),
            Digest::from_sha256([1; 32]),
            HostLimits::default(),
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
        principal: PrincipalId::new("agent").unwrap(),
        scope: ScopeId::new("scope").unwrap(),
        permissions: Default::default(),
    }
}
fn time() -> UnixMillis {
    UnixMillis::new(100).unwrap()
}

#[test]
fn discovery_filters_permissions_and_does_not_claim_configured_rendering() {
    let db = Database::new();
    let mut host = db.host();
    let mut ctx = context();
    let text = host.dispatch_json(&ctx, "{\"operation\":\"capabilities\"}", &time, &|| false);
    let response: HostResponse = serde_json::from_str(&text).unwrap();
    let HostResponse::Succeeded {
        result: HostResult::Capabilities { capabilities: caps },
    } = response
    else {
        panic!("capabilities required")
    };
    assert_eq!(
        caps.operations
            .iter()
            .map(|o| o.operation)
            .collect::<Vec<_>>(),
        vec![ServiceOperation::Capabilities, ServiceOperation::Schema]
    );
    assert!(!caps.complete_feature_catalogue && !caps.full_presentation_acceptance);
    assert_eq!(caps.queued_execution, JobExecution::ExplicitRun);
    assert!(caps.renderer.is_none());
    ctx.permissions.extend([
        Permission::Export,
        Permission::ReadDocument,
        Permission::ReadAssets,
    ]);
    let caps = host.capabilities(&ctx);
    let export = caps
        .operations
        .iter()
        .find(|o| o.operation == ServiceOperation::Export)
        .unwrap();
    assert!(!export.available);
    assert_eq!(
        export.unavailable_reason,
        Some(UnavailableReason::PreviewRendererNotConfigured)
    );
    assert_eq!(
        export.revision_policy,
        Some(RevisionPolicy::ImmutableHistorical)
    );
    assert_eq!(
        caps.limits.asset_chunk_bytes.get(),
        ASSET_CHUNK_BYTES as u64
    );
    assert_eq!(caps.limits.export.asset_bytes.get(), 128 * 1024 * 1024);
    let conn = rusqlite::Connection::open(db.0.join("host.sqlite")).unwrap();
    assert_eq!(
        conn.query_row("SELECT count(*) FROM jobs", [], |r| r.get::<_, u32>(0))
            .unwrap(),
        0
    );
}

fn resource_roundtrip<H: OperationHost>(host: &mut H, ctx: &CallContext) {
    let bytes = b"actual bytes through the owner port";
    let request = HostRequest::BeginUpload {
        request: UploadRequest {
            request_id: RequestId::new("resource").unwrap(),
            descriptor: AssetDescriptor {
                sha256: Digest::from_sha256(Sha256::digest(bytes).into()),
                byte_length: ByteLength::new(bytes.len() as u64),
                media_type: "application/octet-stream".into(),
            },
        },
    };
    let denied = dispatch_host(host, ctx, request.clone(), &time, &|| false);
    assert!(matches!(
        denied,
        HostResponse::Failed {
            error: Failure {
                code: FailureCode::NotAuthorized,
                ..
            },
            job: None
        }
    ));
    let mut allowed = ctx.clone();
    allowed
        .permissions
        .extend([Permission::WriteAssets, Permission::ReadAssets]);
    let HostResponse::Succeeded {
        result: HostResult::Upload { upload },
    } = dispatch_host(host, &allowed, request, &time, &|| false)
    else {
        panic!("upload required")
    };
    if upload.state == UploadState::Uploading {
        host.append_upload(&allowed, &upload.id, ByteLength::new(0), bytes, time())
            .unwrap();
    } else {
        assert_eq!(upload.state, UploadState::Sealed);
    }
    let HostResponse::Succeeded {
        result: HostResult::Upload { upload },
    } = dispatch_host(
        host,
        &allowed,
        HostRequest::SealUpload {
            upload_id: upload.id,
        },
        &time,
        &|| false,
    )
    else {
        panic!("seal required")
    };
    let asset = upload.asset.unwrap();
    let reader = host.open_asset(&allowed, &asset.id).unwrap();
    assert_eq!(reader.info(), &asset);
    let mut copy = vec![0; bytes.len()];
    reader.read_exact_at(&mut copy, 0).unwrap();
    assert_eq!(copy, bytes);
}
#[test]
fn generic_owner_port_routes_real_binary_resources() {
    let db = Database::new();
    let mut host = db.host();
    resource_roundtrip(&mut host, &context());
    drop(host);
    let mut reopened = db.host();
    resource_roundtrip(&mut reopened, &context());
}

#[test]
fn schema_discovery_is_read_only_and_rejects_injected_authority() {
    let db = Database::new();
    let mut host = db.host();
    for id in SchemaId::ALL {
        let result = dispatch_host(
            &mut host,
            &context(),
            HostRequest::GetSchema { id },
            &time,
            &|| false,
        );
        let HostResponse::Succeeded {
            result: HostResult::Schema { document },
        } = result
        else {
            panic!("schema required")
        };
        assert_eq!(document.digest, id.document().unwrap().digest);
    }
    for input in [
        r#"{"operation":"capabilities","principal":"admin"}"#,
        r#"{"operation":"getSchema","id":"private-file"}"#,
        r#"{"operation":"getSchema","id":"document","permissions":["export"]}"#,
    ] {
        let result: HostResponse =
            serde_json::from_str(&host.dispatch_json(&context(), input, &time, &|| false)).unwrap();
        assert!(matches!(
            result,
            HostResponse::Failed {
                error: Failure {
                    code: FailureCode::InputInvalid,
                    ..
                },
                job: None
            }
        ));
    }
    let conn = rusqlite::Connection::open(db.0.join("host.sqlite")).unwrap();
    assert_eq!(
        conn.query_row("SELECT count(*) FROM jobs", [], |r| r.get::<_, u32>(0))
            .unwrap(),
        0
    );
}
