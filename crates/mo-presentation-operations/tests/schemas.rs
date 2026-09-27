use mo_presentation_operations::{FailureCode, SchemaId, computation_schema_json, decode_request};

#[test]
fn discovery_matches_the_generated_public_schemas_and_rejects_host_schema_ids() {
    for id in SchemaId::ALL {
        let name = serde_json::to_string(&id).unwrap();
        let document: mo_presentation_operations::SchemaDocument =
            serde_json::from_str(&computation_schema_json(&name).unwrap()).unwrap();
        assert_eq!(document.id, id);
        assert_eq!(document.digest, id.document().unwrap().digest);
        let file = format!(
            "../../contracts/generated/{}.schema.json",
            name.trim_matches('"')
        );
        let generated: serde_json::Value =
            serde_json::from_slice(&std::fs::read(file).unwrap()).unwrap();
        assert_eq!(document.schema, generated);
    }
    for legacy in [
        "host-request",
        "operation-job",
        "upload-request",
        "host-capabilities",
    ] {
        assert_eq!(
            computation_schema_json(&serde_json::to_string(legacy).unwrap())
                .unwrap_err()
                .code,
            FailureCode::InputInvalid
        );
    }
    assert_eq!(
        computation_schema_json(&"x".repeat(129)).unwrap_err().code,
        FailureCode::LimitExceeded
    );
}

#[test]
fn bounded_request_decode_rejects_duplicate_keys_before_evaluation() {
    assert_eq!(decode_request(r#"{"contractVersion":"musteroffice.computation/1-draft","requestId":"a","requestId":"b"}"#).unwrap_err().code, FailureCode::InputInvalid);
    let oversized = " ".repeat(mo_presentation_operations::MAX_OPERATION_BYTES + 1);
    assert_eq!(
        decode_request(&oversized).unwrap_err().code,
        FailureCode::LimitExceeded
    );
}
