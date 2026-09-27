use mo_embedded_sdk::{
    Inputs, Presentation,
    common::{OperationId, RequestId},
    edit::{Operation, OperationEntry},
    execute,
    model::Document,
    operation::*,
};
use std::cell::Cell;

fn input() -> Invocation {
    let document: Document = serde_json::from_str(include_str!(
        "../../../fixtures/presentations/basic-shape.json"
    ))
    .unwrap();
    Invocation {
        request: OperationRequest {
            contract_version: ContractVersion::V1,
            request_id: RequestId::new("create").unwrap(),
            profile_id: OperationProfile::AuthorModel,
            action: DocumentAction::Create {
                document: Box::new(document),
            },
        },
        snapshot: None,
    }
}
#[test]
fn direct_dispatch_preserves_transaction_identity_and_cancelled_inputs() {
    let create = input();
    let created = execute(create.clone(), &Inputs::new(), None, &|| false).unwrap();
    assert_eq!(
        &create.request.digest().unwrap(),
        &created.receipt().request_digest
    );
    let (receipt, binary) = created.into_parts();
    assert!(binary.is_none());
    let ComputationResult::Mutated { snapshot, .. } = receipt.result else {
        panic!("mutation");
    };
    let mut expected = Presentation::from_snapshot(*snapshot.clone()).unwrap();
    let operations = vec![OperationEntry {
        operation_id: OperationId::new("title").unwrap(),
        operation: Operation::SetTitle {
            title: "new revision".into(),
        },
    }];
    let id = RequestId::new("edit").unwrap();
    expected
        .edit(id.clone(), operations.clone(), &|| false)
        .unwrap();
    let edit = Invocation {
        request: OperationRequest {
            request_id: id,
            action: DocumentAction::Apply {
                document_id: snapshot.document.id.clone(),
                base_revision: snapshot.revision.clone(),
                operations,
            },
            ..create.request
        },
        snapshot: Some(snapshot),
    };
    let calls = Cell::new(0);
    let success = execute(edit.clone(), &Inputs::new(), None, &|| {
        calls.set(calls.get() + 1);
        false
    })
    .unwrap();
    let ComputationResult::Mutated {
        snapshot: actual, ..
    } = &success.receipt().result
    else {
        panic!("mutation");
    };
    assert_eq!(actual.as_ref(), expected.snapshot());
    for stop in 1..=calls.get() {
        let count = Cell::new(0);
        let result = execute(edit.clone(), &Inputs::new(), None, &|| {
            count.set(count.get() + 1);
            count.get() == stop
        });
        assert_eq!(result.err().unwrap().code, FailureCode::Cancelled);
    }
    assert_ne!(edit.snapshot.unwrap().revision, actual.revision);
}

#[test]
fn strict_invocation_keeps_shape_and_component_budgets_without_host_fields() {
    let input = input();
    let raw = serde_json::to_string(&input).unwrap();
    assert!(decode_invocation(&raw).is_ok());
    let mut extra = serde_json::to_value(&input).unwrap();
    extra["jobId"] = "injected".into();
    assert_eq!(
        decode_invocation(&extra.to_string()).unwrap_err().code,
        FailureCode::InputInvalid
    );
    let DocumentAction::Create { document } = &input.request.action else {
        unreachable!()
    };
    let snapshot = Presentation::create(*document.clone(), &|| false)
        .unwrap()
        .into_snapshot();
    let mut invalid = input.clone();
    invalid.snapshot = Some(Box::new(snapshot));
    assert_eq!(
        invalid.validate().unwrap_err().code,
        FailureCode::InputInvalid
    );
    let mut too_large = input;
    let DocumentAction::Create { document } = &mut too_large.request.action else {
        unreachable!()
    };
    document.title = "x".repeat(MAX_OPERATION_BYTES);
    assert_eq!(
        too_large.validate().unwrap_err().code,
        FailureCode::LimitExceeded
    );
    let cancelled = Cell::new(0);
    assert_eq!(
        too_large
            .validate_cancellable(&|| {
                cancelled.set(cancelled.get() + 1);
                true
            })
            .unwrap_err()
            .code,
        FailureCode::Cancelled
    );
}
