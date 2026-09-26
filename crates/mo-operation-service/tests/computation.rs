use mo_common::{Digest, OperationId, RequestId};
use mo_operation_service::*;
use mo_presentation_edit::{Operation, OperationEntry};
use mo_presentation_model::Document;
use std::cell::Cell;

fn request() -> OperationRequest {
    let v: serde_json::Value = serde_json::from_str(include_str!(
        "../../../fixtures/presentations/playback/page.json"
    ))
    .unwrap();
    let document: Document = serde_json::from_value(v["page"]["document"].clone()).unwrap();
    OperationRequest {
        contract_version: ContractVersion::V1,
        request_id: RequestId::new("create").unwrap(),
        profile_id: OperationProfile::AuthorModel,
        output_mode: OutputMode::Job,
        action: DocumentAction::Create {
            document: Box::new(document),
        },
    }
}

#[test]
fn create_and_multi_operation_edit_are_atomic_at_every_cancellation_checkpoint() {
    let create = request();
    let points = Cell::new(0);
    let original = compute_mutation(&create, None, &|| {
        points.set(points.get() + 1);
        false
    })
    .unwrap();
    let record = original.snapshot().clone();
    for stop in 1..=points.get() {
        let count = Cell::new(0);
        assert!(matches!(
            compute_mutation(&create, None, &|| {
                count.set(count.get() + 1);
                count.get() == stop
            }),
            Err(Failure {
                code: FailureCode::Cancelled,
                ..
            })
        ));
        assert_eq!(
            compute_mutation(&create, None, &|| false)
                .unwrap()
                .snapshot(),
            &record
        );
    }
    let edit = OperationRequest {
        request_id: RequestId::new("edit").unwrap(),
        action: DocumentAction::Apply {
            document_id: record.document.id.clone(),
            base_revision: record.revision.clone(),
            operations: (0..4)
                .map(|i| OperationEntry {
                    operation_id: OperationId::new(format!("op:{i}")).unwrap(),
                    operation: Operation::SetTitle {
                        title: format!("title {i}"),
                    },
                })
                .collect(),
        },
        ..create
    };
    points.set(0);
    let expected = compute_mutation(&edit, Some(record.clone()), &|| {
        points.set(points.get() + 1);
        false
    })
    .unwrap();
    assert_eq!(expected.snapshot().document.title, "title 3");
    assert_eq!(
        expected.base_semantic_digest(),
        Some(&record.semantic_digest)
    );
    assert_eq!(
        serde_json::from_str::<mo_presentation_edit::SnapshotRecord>(expected.snapshot_json())
            .unwrap(),
        *expected.snapshot()
    );
    assert!(points.get() > 10);
    for stop in 1..=points.get() {
        let count = Cell::new(0);
        assert!(matches!(
            compute_mutation(&edit, Some(record.clone()), &|| {
                count.set(count.get() + 1);
                count.get() == stop
            }),
            Err(Failure {
                code: FailureCode::Cancelled,
                ..
            })
        ));
        let recovered = compute_mutation(&edit, Some(record.clone()), &|| false).unwrap();
        assert_eq!(expected.snapshot(), recovered.snapshot());
        assert_eq!(
            expected.receipt().transaction,
            recovered.receipt().transaction
        );
    }
}

#[test]
fn request_identity_excludes_transport_preference_but_binds_all_operation_arguments() {
    let mut a = request();
    let original = a.digest().unwrap();
    for mode in [OutputMode::Auto, OutputMode::Sync, OutputMode::Job] {
        a.output_mode = mode;
        assert_eq!(a.digest().unwrap(), original);
    }
    if let DocumentAction::Create { document } = &mut a.action {
        document.title.push('!');
    }
    assert_ne!(a.digest().unwrap(), original);
    let mut v = serde_json::to_value(request()).unwrap();
    v["principal"] = serde_json::json!("forged");
    assert!(mo_common::from_json_str::<OperationRequest>(&v.to_string()).is_err());
    v.as_object_mut().unwrap().remove("principal");
    v["action"]["path"] = serde_json::json!("/arbitrary.pptx");
    assert!(mo_common::from_json_str::<OperationRequest>(&v.to_string()).is_err());
    assert!(
        mo_common::from_json_str::<HostRequest>(
            r#"{"operation":"getJob","jobId":"one","jobId":"two"}"#
        )
        .is_err()
    );
}

#[test]
fn counters_are_canonical_and_preserve_values_above_javascript_integer_precision() {
    for value in [0, 1, 9_007_199_254_740_993, i64::MAX] {
        let value = UnixMillis::new(value).unwrap();
        let json = serde_json::to_string(&value).unwrap();
        assert_eq!(json, format!("\"{}\"", value.get()));
        assert_eq!(serde_json::from_str::<UnixMillis>(&json).unwrap(), value);
    }
    for invalid in ["-1", "01", "+1", "-0", "1.0", " 1", "9223372036854775808"] {
        assert!(serde_json::from_value::<UnixMillis>(serde_json::json!(invalid)).is_err());
        assert!(serde_json::from_value::<JobFence>(serde_json::json!(invalid)).is_err());
    }
    assert!(serde_json::from_str::<JobFence>("1").is_err());
    assert!(JobFence::new(i64::MAX).unwrap().checked_add(1).is_none());
}

#[test]
fn transport_fields_are_camel_case_and_unsupported_profiles_fail() {
    let create = request();
    let candidate = compute_mutation(&create, None, &|| false).unwrap();
    let apply = OperationRequest {
        action: DocumentAction::Apply {
            document_id: candidate.document_id().clone(),
            base_revision: Digest::from_sha256([0; 32]),
            operations: vec![],
        },
        ..create
    };
    let v = serde_json::to_value(apply).unwrap();
    assert!(v["action"].get("documentId").is_some());
    assert!(v["action"].get("baseRevision").is_some());
    assert!(v["action"].get("document_id").is_none());
    let read = HostRequest::ReadDocument {
        document_id: candidate.document_id().clone(),
        revision: None,
    };
    assert!(
        serde_json::to_value(read)
            .unwrap()
            .get("documentId")
            .is_some()
    );
    let result = HostResponse::Succeeded {
        result: HostResult::Document {
            snapshot: Box::new(candidate.snapshot().clone()),
        },
    };
    let encoded = serde_json::to_value(result).unwrap();
    assert_eq!(encoded["outcome"], "succeeded");
    assert_eq!(encoded["result"]["kind"], "document");
    let mut v = serde_json::to_value(request()).unwrap();
    v["profileId"] = serde_json::json!("full-presentation-unverified");
    assert!(serde_json::from_value::<OperationRequest>(v).is_err());
}
