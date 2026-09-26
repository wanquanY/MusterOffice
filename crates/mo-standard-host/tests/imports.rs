//! Imported documents use the same durable mutation owner and authorization.
mod support;
use mo_common::*;
use mo_operation_service::*;
use mo_presentation_edit::{Operation, OperationEntry};
use mo_presentation_model::ObjectContent;
use mo_standard_host::{HostLimits, StandardHost};
fn clock() -> UnixMillis {
    UnixMillis::new(100).unwrap()
}
fn context() -> CallContext {
    CallContext {
        principal: PrincipalId::new("agent").unwrap(),
        scope: ScopeId::new("scope").unwrap(),
        permissions: [
            Permission::Create,
            Permission::Edit,
            Permission::ReadAssets,
            Permission::WriteAssets,
            Permission::ReadDocument,
            Permission::ReadJob,
            Permission::CancelJob,
        ]
        .into_iter()
        .collect(),
    }
}
struct Image;
impl mo_pptx::Resources for Image {
    fn open(&self, _: &ResourceId) -> Result<mo_pptx::ResourceData<'_>, mo_pptx::PptxError> {
        static BYTES: &[u8] =
            include_bytes!("../../../fixtures/presentations/native-export/resources.bin");
        Ok(mo_pptx::ResourceData {
            reader: &BYTES,
            byte_length: BYTES.len() as u64,
        })
    }
}
fn source() -> Vec<u8> {
    let v: serde_json::Value = serde_json::from_str(include_str!(
        "../../../fixtures/presentations/native-export/request.json"
    ))
    .unwrap();
    mo_pptx::export(
        &serde_json::from_value(v["document"].clone()).unwrap(),
        &serde_json::from_value(v["defaults"].clone()).unwrap(),
        &Image,
        Default::default(),
        &|| false,
    )
    .unwrap()
}
fn import(host: &mut StandardHost) -> OperationRequest {
    let bytes = source();
    let p = mo_opc::Package::open(
        bytes.as_slice(),
        bytes.len() as u64,
        Default::default(),
        &|| false,
    )
    .unwrap();
    let upload = host
        .begin_upload(
            &context(),
            UploadRequest {
                request_id: RequestId::new("upload").unwrap(),
                descriptor: AssetDescriptor {
                    sha256: p.sha256().clone(),
                    byte_length: ByteLength::new(bytes.len() as u64),
                    media_type: mo_presentation_delivery::PPTX_MIME.into(),
                },
            },
            clock(),
        )
        .unwrap();
    host.append_upload(&context(), &upload.id, ByteLength::new(0), &bytes, clock())
        .unwrap();
    let asset = host
        .seal_upload(&context(), &upload.id, &clock, &|| false)
        .unwrap()
        .asset
        .unwrap();
    OperationRequest {
        contract_version: ContractVersion::V1,
        request_id: RequestId::new("import").unwrap(),
        profile_id: OperationProfile::AuthorModel,
        output_mode: OutputMode::Job,
        action: DocumentAction::Import {
            document_id: DocumentId::new("imported").unwrap(),
            source: AssetBinding {
                resource_id: ResourceId::new("original").unwrap(),
                asset_id: asset.id,
            },
        },
    }
}
#[test]
fn imported_revision_survives_restart_replays_and_rejects_stale_edit() {
    let dir = support::temporary_directory("mo-import");
    let path = dir.join("host.sqlite");
    let mut host =
        StandardHost::open(&path, Digest::from_sha256([1; 32]), HostLimits::default()).unwrap();
    let request = import(&mut host);
    let mut denied = context();
    denied.permissions.remove(&Permission::ReadAssets);
    assert_eq!(
        host.submit(&denied, request.clone(), clock())
            .unwrap_err()
            .code,
        FailureCode::NotAuthorized
    );
    let job = host.submit(&context(), request.clone(), clock()).unwrap();
    let done = host
        .run_next(&context(), clock(), &clock, &|| false)
        .unwrap()
        .unwrap();
    assert_eq!(done.id, job.id);
    assert_eq!(done.state, JobState::Succeeded);
    assert_eq!(
        host.submit(&context(), request.clone(), clock())
            .unwrap()
            .id,
        job.id
    );
    let snapshot = host
        .read_document(&context(), request.action.document_id(), None)
        .unwrap();
    let object = snapshot.document.objects.values().find(|o| matches!(&o.content, ObjectContent::RetainedSource { paragraphs, .. } if !paragraphs.is_empty() && !paragraphs[0].runs.is_empty())).unwrap();
    let ObjectContent::RetainedSource { paragraphs, .. } = &object.content else {
        unreachable!()
    };
    let edit = OperationRequest {
        contract_version: ContractVersion::V1,
        request_id: RequestId::new("edit").unwrap(),
        profile_id: OperationProfile::AuthorModel,
        output_mode: OutputMode::Sync,
        action: DocumentAction::Apply {
            document_id: snapshot.document.id.clone(),
            base_revision: snapshot.revision.clone(),
            operations: vec![OperationEntry {
                operation_id: OperationId::new("splice").unwrap(),
                operation: Operation::SpliceText {
                    object: object.id.clone(),
                    paragraph: paragraphs[0].id.clone(),
                    run: paragraphs[0].runs[0].id.clone(),
                    start: 0,
                    delete: 0,
                    insert: "Imported ".into(),
                },
            }],
        },
    };
    let job = host.submit(&context(), edit.clone(), clock()).unwrap();
    assert_eq!(
        host.run_job(&context(), &job.id, clock(), &clock, &|| false)
            .unwrap()
            .state,
        JobState::Succeeded
    );
    let changed = host
        .read_document(&context(), &snapshot.document.id, None)
        .unwrap();
    assert_ne!(changed.revision, snapshot.revision);
    assert_eq!(
        changed.document.source_bindings,
        snapshot.document.source_bindings
    );
    let mut stale = edit.clone();
    stale.request_id = RequestId::new("stale").unwrap();
    let job = host.submit(&context(), stale, clock()).unwrap();
    let failed = host
        .run_job(&context(), &job.id, clock(), &clock, &|| false)
        .unwrap();
    assert!(matches!(
        failed.result,
        Some(TerminalResult::Failed {
            error: Failure {
                code: FailureCode::RevisionConflict,
                ..
            }
        })
    ));
    drop(host);
    let mut host =
        StandardHost::open(&path, Digest::from_sha256([1; 32]), HostLimits::default()).unwrap();
    assert_eq!(
        host.read_document(&context(), &snapshot.document.id, None)
            .unwrap(),
        changed
    );
    assert_eq!(
        host.read_document(&context(), &snapshot.document.id, Some(&snapshot.revision))
            .unwrap(),
        snapshot
    );
    assert_eq!(
        host.submit(&context(), edit, clock()).unwrap().state,
        JobState::Succeeded
    );
    drop(host);
    std::fs::remove_dir_all(dir).unwrap();
}
