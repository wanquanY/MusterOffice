use mo_embedded_sdk::{Inputs, Presentation, common::*, edit::*, model::Document, operation::*};
use std::cell::Cell;

fn document() -> Document {
    let value: serde_json::Value = serde_json::from_str(include_str!(
        "../../../fixtures/presentations/playback/page.json"
    ))
    .unwrap();
    serde_json::from_value(value["page"]["document"].clone()).unwrap()
}
fn operations() -> Vec<OperationEntry> {
    (0..3)
        .map(|i| OperationEntry {
            operation_id: OperationId::new(format!("edit:{i}")).unwrap(),
            operation: Operation::SetTitle {
                title: format!("title {i}"),
            },
        })
        .collect()
}
#[test]
fn in_memory_edit_is_atomic_at_each_cancellation_point_and_restores_without_a_host() {
    let original = Presentation::create(document(), &|| false)
        .unwrap()
        .into_snapshot();
    let mut expected = Presentation::from_snapshot(original.clone()).unwrap();
    let calls = Cell::new(0);
    let receipt = expected
        .edit(RequestId::new("edit").unwrap(), operations(), &|| {
            calls.set(calls.get() + 1);
            false
        })
        .unwrap();
    assert_eq!(expected.document().title, "title 2");
    assert_eq!(receipt.revision, expected.snapshot().revision);
    for stop in 1..=calls.get() {
        let mut deck = Presentation::from_snapshot(original.clone()).unwrap();
        let count = Cell::new(0);
        let result = deck.edit(RequestId::new("edit").unwrap(), operations(), &|| {
            count.set(count.get() + 1);
            count.get() == stop
        });
        assert_eq!(result.unwrap_err().code, FailureCode::Cancelled);
        assert_eq!(deck.snapshot(), &original);
        deck.edit(RequestId::new("edit").unwrap(), operations(), &|| false)
            .unwrap();
        assert_eq!(deck.snapshot(), expected.snapshot());
    }
    let mut corrupt = original;
    corrupt.document.title.push('!');
    assert!(Presentation::from_snapshot(corrupt).is_err());
}

#[test]
fn borrowed_input_registration_is_explicit_and_does_not_replace_existing_data() {
    let mut inputs = Inputs::new();
    let id = AssetId::new("image:1").unwrap();
    inputs
        .insert_bytes(id.clone(), "image/png", b"12345", &|| false)
        .unwrap();
    let input = inputs.get(&id).unwrap();
    let mut part = [0; 2];
    input.reader.read_exact_at(&mut part, 2).unwrap();
    assert_eq!(&part, b"34");
    assert_eq!(input.info.descriptor.byte_length.get(), 5);
    assert_eq!(
        inputs
            .insert_bytes(id.clone(), "image/png", b"other", &|| false)
            .unwrap_err()
            .code,
        FailureCode::ResourceConflict
    );
    let cancelled_id = AssetId::new("cancelled").unwrap();
    assert_eq!(
        inputs
            .insert_bytes(cancelled_id.clone(), "image/png", b"bytes", &|| true)
            .unwrap_err()
            .code,
        FailureCode::Cancelled
    );
    assert!(inputs.get(&cancelled_id).is_err());
    let mut data = [0; 5];
    inputs
        .get(&id)
        .unwrap()
        .reader
        .read_exact_at(&mut data, 0)
        .unwrap();
    assert_eq!(&data, b"12345");
}

#[test]
fn computation_envelope_rejects_legacy_jobs_and_injected_authority() {
    let request = OperationRequest {
        contract_version: ContractVersion::V1,
        request_id: RequestId::new("create").unwrap(),
        profile_id: OperationProfile::AuthorModel,
        action: DocumentAction::Create {
            document: Box::new(document()),
        },
    };
    let wire = serde_json::to_value(&request).unwrap();
    assert_eq!(wire["contractVersion"], "musteroffice.computation/1-draft");
    for field in [
        "principal",
        "scope",
        "permissions",
        "jobId",
        "fence",
        "outputMode",
    ] {
        let mut injected = wire.clone();
        injected[field] = "forged".into();
        assert!(
            mo_embedded_sdk::common::from_json_str::<OperationRequest>(&injected.to_string())
                .is_err(),
            "{field}"
        );
    }
    let mut old = wire;
    old["contractVersion"] = "musteroffice.operations/1-draft".into();
    assert!(serde_json::from_value::<OperationRequest>(old).is_err());
}
