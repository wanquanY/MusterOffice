use mo_common::{Digest, DocumentId, Emu, ObjectId, OperationId, RequestId};
use mo_kernel_api::{KernelRequest, KernelResponse};
use mo_operation_service::*;
use mo_presentation_edit::{Operation, OperationEntry, Snapshot, Transaction};
use mo_presentation_model::{Document, Size, ValidationLimits};

fn document() -> Document {
    Document::empty(
        DocumentId::new("diagnostics").unwrap(),
        Size {
            width: Emu::new(914400),
            height: Emu::new(914400),
        },
    )
}
fn request(action: DocumentAction) -> OperationRequest {
    OperationRequest {
        contract_version: ContractVersion::V1,
        request_id: RequestId::new("request").unwrap(),
        profile_id: OperationProfile::AuthorModel,
        output_mode: OutputMode::Sync,
        action,
    }
}
fn equal_diagnostic(kernel: KernelResponse, operation: Failure) -> serde_json::Value {
    let KernelResponse::Error { error } = kernel else {
        panic!("expected a diagnostic")
    };
    let expected = serde_json::to_value(error).unwrap();
    assert_eq!(operation.detail.as_deref(), Some(&expected));
    assert_eq!(
        serde_json::to_value(operation.code).unwrap(),
        expected["code"]
    );
    assert_eq!(operation.message, expected["message"]);
    expected
}

#[test]
fn invalid_document_paths_survive_both_public_envelopes() {
    let mut document = document();
    document.page_size.width = Emu::new(0);
    let kernel = mo_kernel_api::dispatch(
        KernelRequest::Initialize {
            document: document.clone(),
        },
        ValidationLimits::default(),
    );
    let service = compute_mutation(
        &request(DocumentAction::Create {
            document: Box::new(document),
        }),
        None,
        &|| false,
    )
    .err()
    .unwrap();
    let diagnostic = equal_diagnostic(kernel, service);
    assert_eq!(diagnostic["report"]["issues"][0]["path"], "/pageSize");
}

#[test]
fn conflict_revision_and_operation_identity_survive_both_envelopes() {
    let snapshot = Snapshot::new(document(), ValidationLimits::default())
        .unwrap()
        .into_record();
    for conflict in [false, true] {
        let base_revision = if conflict {
            Digest::from_sha256([3; 32])
        } else {
            snapshot.revision.clone()
        };
        let operations = vec![OperationEntry {
            operation_id: OperationId::new("edit-missing").unwrap(),
            operation: Operation::DeleteObject {
                object: ObjectId::new("missing").unwrap(),
                policy: mo_presentation_edit::DeletePolicy::RejectDependencies,
            },
        }];
        let action = DocumentAction::Apply {
            document_id: snapshot.document.id.clone(),
            base_revision: base_revision.clone(),
            operations: operations.clone(),
        };
        let transaction = Transaction {
            document_id: snapshot.document.id.clone(),
            request_id: RequestId::new("request").unwrap(),
            base_revision,
            operations,
        };
        let kernel = mo_kernel_api::dispatch(
            KernelRequest::Prepare {
                snapshot: snapshot.clone(),
                transaction,
            },
            ValidationLimits::default(),
        );
        let service = compute_mutation(&request(action), Some(snapshot.clone()), &|| false)
            .err()
            .unwrap();
        let diagnostic = equal_diagnostic(kernel, service);
        if conflict {
            assert_eq!(diagnostic["currentRevision"], snapshot.revision.as_str());
        } else {
            assert_eq!(diagnostic["operationIds"][0], "edit-missing");
        }
    }
}
