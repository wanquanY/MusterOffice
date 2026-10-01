use mo_common::*;
use mo_operation_service::*;
use mo_presentation_edit::Snapshot;
use mo_presentation_model::{Document, Size};

#[test]
fn new_template_computation_does_not_expand_the_retired_persistent_host_contract() {
    let source = Snapshot::new(
        Document::empty(
            DocumentId::new("source").unwrap(),
            Size {
                width: Emu::new(100),
                height: Emu::new(100),
            },
        ),
        Default::default(),
    )
    .unwrap();
    // Use the same public definition type without adding a production legacy
    // dependency on the template engine or interpreting it inside the host.
    let action:DocumentAction=serde_json::from_value(serde_json::json!({
        "kind":"instantiateTemplate","documentId":"instance","templateDigest":"a".repeat(64),"bindings":{},
        "definition":{"format":"musteroffice.presentation-template/1-draft","source":{"documentId":source.document().id,"revision":source.revision(),"semanticDigest":source.semantic_digest()},"parameters":{}}
    })).unwrap();
    let DocumentAction::InstantiateTemplate { definition, .. } = &action else {
        panic!()
    };
    let describe = DocumentAction::DescribeTemplate {
        definition: definition.clone(),
    };
    for action in [action, describe] {
        assert!(ServiceOperation::for_action(&action).is_err());
        let request = OperationRequest {
            contract_version: ContractVersion::V1,
            request_id: RequestId::new("new-action").unwrap(),
            profile_id: OperationProfile::AuthorModel,
            output_mode: OutputMode::Auto,
            action,
        };
        assert!(request.validate_profile().is_err());
        assert!(
            serde_json::from_str::<OperationRequest>(&serde_json::to_string(&request).unwrap())
                .is_err()
        );
        assert!(compute_mutation(&request, Some(source.clone().into_record()), &|| false).is_err());
    }
    let schema = SchemaId::OperationRequest.schema();
    assert!(
        !serde_json::to_string(&schema)
            .unwrap()
            .contains("describeTemplate")
    );
    assert!(
        !serde_json::to_string(&schema)
            .unwrap()
            .contains("instantiateTemplate")
    );
}

#[test]
fn compact_creation_does_not_expand_the_frozen_persistent_host_contract() {
    let invocation: serde_json::Value = serde_json::from_str(include_str!(
        "../../../fixtures/presentations/compose/invocation.json"
    ))
    .unwrap();
    let action: DocumentAction =
        serde_json::from_value(invocation["request"]["action"].clone()).unwrap();
    assert!(ServiceOperation::for_action(&action).is_err());
    let mut request = invocation["request"].clone();
    request["outputMode"] = serde_json::json!("auto");
    assert!(serde_json::from_value::<OperationRequest>(request).is_err());
    assert!(
        !serde_json::to_string(&SchemaId::OperationRequest.schema())
            .unwrap()
            .contains("PresentationContent")
    );
}

#[test]
fn compact_append_remains_outside_the_frozen_persistent_host_contract() {
    let action = DocumentAction::Append {
        document_id: DocumentId::new("existing").unwrap(),
        base_revision: Digest::from_sha256([0xaa; 32]),
        slides: vec![],
        resources: vec![],
    };
    assert!(ServiceOperation::for_action(&action).is_err());
    let request = OperationRequest {
        contract_version: ContractVersion::V1,
        request_id: RequestId::new("append").unwrap(),
        profile_id: OperationProfile::AuthorModel,
        output_mode: OutputMode::Auto,
        action,
    };
    assert!(request.validate_profile().is_err());
    assert!(
        serde_json::from_str::<OperationRequest>(&serde_json::to_string(&request).unwrap())
            .is_err()
    );
    assert!(compute_mutation(&request, None, &|| false).is_err());
}
