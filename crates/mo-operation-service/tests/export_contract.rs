use mo_common::{Digest, DocumentId};
use mo_operation_service::*;
use mo_presentation_delivery::DeliveryError;

#[test]
fn export_errors_preserve_cancellation_budget_authority_and_render_detail() {
    for error in [
        DeliveryError::Cancelled,
        DeliveryError::Pptx(mo_pptx::PptxError::Cancelled),
        DeliveryError::Pptx(mo_pptx::PptxError::Opc(mo_opc::OpcError::Cancelled)),
        DeliveryError::Pptx(mo_pptx::PptxError::Xml(mo_xml::XmlError::Cancelled)),
        DeliveryError::Png(mo_image::png::PngError::Cancelled),
    ] {
        assert_eq!(delivery_failure(error).code, FailureCode::Cancelled);
    }
    for error in [
        DeliveryError::Limit("output"),
        DeliveryError::Pptx(mo_pptx::PptxError::Opc(mo_opc::OpcError::Limit("bytes"))),
        DeliveryError::Pptx(mo_pptx::PptxError::Opc(mo_opc::OpcError::Xml {
            part: "/owned.xml".into(),
            source: mo_xml::XmlError::Limit("nodes"),
        })),
        DeliveryError::Png(mo_image::png::PngError::Limit("bytes")),
    ] {
        assert_eq!(delivery_failure(error).code, FailureCode::LimitExceeded);
    }
    let expected = Failure::new(FailureCode::ResourceExpired, "expired result owner");
    for error in [
        DeliveryError::Io(std::io::Error::other(expected.clone())),
        DeliveryError::Pptx(mo_pptx::PptxError::Opc(mo_opc::OpcError::Io(
            std::io::Error::other(expected.clone()),
        ))),
        DeliveryError::Png(mo_image::png::PngError::Io(std::io::Error::other(
            expected.clone(),
        ))),
    ] {
        assert_eq!(delivery_failure(error), expected);
    }
    let detail = serde_json::json!({"stage":"page","error":{"location":{"part":"/ppt/slides/slide1.xml","object":42}}});
    let failure = delivery_failure(DeliveryError::Preview {
        message: "unimplemented visible content".into(),
        diagnostic: Some(Box::new(detail.clone())),
    });
    assert_eq!(failure.code, FailureCode::RenderFailure);
    assert_eq!(failure.detail.as_deref(), Some(&detail));
}

#[test]
fn mutation_receipt_and_failure_wire_shapes_remain_compatible() {
    #[derive(serde::Serialize)]
    struct Previous<'a> {
        outcome: &'static str,
        receipt: &'a MutationReceipt,
    }
    let mutation = MutationReceipt {
        template: None,
        document_id: DocumentId::new("doc").unwrap(),
        revision: Digest::from_sha256([1; 32]),
        semantic_digest: Digest::from_sha256([2; 32]),
        transaction: None,
    };
    let previous = serde_json::to_string(&Previous {
        outcome: "succeeded",
        receipt: &mutation,
    })
    .unwrap();
    let current = TerminalResult::Succeeded {
        receipt: Box::new(OperationReceipt::Mutation(mutation)),
    };
    assert_eq!(serde_json::to_string(&current).unwrap(), previous);
    let decoded: TerminalResult = mo_common::from_json_str(&previous).unwrap();
    assert!(
        matches!(decoded,TerminalResult::Succeeded {receipt} if matches!(*receipt,OperationReceipt::Mutation(_)))
    );
    let failure = Failure::new(FailureCode::Cancelled, "cancelled");
    assert_eq!(
        serde_json::to_string(&failure).unwrap(),
        r#"{"code":"CANCELLED","message":"cancelled"}"#
    );
    let mut mixed: serde_json::Value = serde_json::from_str(&previous).unwrap();
    mixed["receipt"]["bundle"] = serde_json::json!({});
    assert!(mo_common::from_json_str::<TerminalResult>(&mixed.to_string()).is_err());
}
