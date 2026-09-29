use mo_common::*;
use mo_opc::{Package, PackageLimits, PartName};
use mo_presentation_edit::{Operation, OperationEntry, Snapshot, Transaction, prepare};
use mo_presentation_model::{ObjectContent, ValidationLimits};
use mo_presentation_operations::*;

fn invocation() -> Invocation {
    decode_invocation(include_str!(
        "../../../fixtures/presentations/compose/invocation.json"
    ))
    .unwrap()
}

#[test]
fn composed_pages_survive_native_export_edit_and_actual_pptx_reimport() {
    let input = invocation();
    let request_id = input.request.request_id.clone();
    let receipt = compute_inline(input, &|| false).unwrap();
    assert_eq!(receipt.request_id, request_id);
    let ComputationResult::Mutated { snapshot, .. } = receipt.result else {
        panic!()
    };
    let original = Snapshot::restore(*snapshot, ValidationLimits::default()).unwrap();
    let object_id = ObjectId::new("object:title").unwrap();
    let ObjectContent::Shape {
        text: Some(text), ..
    } = &original.document().objects[&object_id].content
    else {
        panic!()
    };
    let transaction = Transaction {
        document_id: original.document().id.clone(),
        request_id: RequestId::new("request:edit-composed").unwrap(),
        base_revision: original.revision().clone(),
        operations: vec![OperationEntry {
            operation_id: OperationId::new("operation:replace-title").unwrap(),
            operation: Operation::SpliceText {
                object: object_id,
                paragraph: text.paragraphs[0].id.clone(),
                run: text.paragraphs[0].runs[0].id.clone(),
                start: 0,
                delete: "原生演示文稿".chars().count() as u32,
                insert: "第二版原生标题".into(),
            },
        }],
    };
    let edited = prepare(&original, &transaction, ValidationLimits::default())
        .unwrap()
        .snapshot;
    assert!(prepare(&edited, &transaction, ValidationLimits::default()).is_err());
    let defaults = serde_json::from_value(
        serde_json::from_str::<serde_json::Value>(include_str!(
            "../../../fixtures/presentations/native-export/request.json"
        ))
        .unwrap()["defaults"]
            .clone(),
    )
    .unwrap();
    for (snapshot, title) in [(&original, "原生演示文稿"), (&edited, "第二版原生标题")]
    {
        let bytes = mo_pptx::export(
            snapshot.document(),
            &defaults,
            &mo_pptx::NoResources,
            Default::default(),
            &|| false,
        )
        .unwrap();
        let package = Package::open(
            bytes.as_slice(),
            bytes.len() as u64,
            PackageLimits::default(),
            &|| false,
        )
        .unwrap();
        let source =
            mo_pptx::source::inspect_source(&package, Default::default(), &|| false).unwrap();
        assert_eq!(source.slides.len(), 3);
        for (page, expected) in [(1, title), (2, "中文排版"), (3, "创建 → 预览 → 导出")]
        {
            let xml = String::from_utf8(
                package
                    .read_part(
                        &PartName::new(format!("/ppt/slides/slide{page}.xml")).unwrap(),
                        100000,
                        &|| false,
                    )
                    .unwrap(),
            )
            .unwrap();
            assert_eq!(xml.matches("<p:sp>").count(), 1);
            assert!(!xml.contains("<p:pic>"));
            assert!(xml.contains(expected));
        }
    }
}

#[test]
fn compact_creation_has_no_base_and_keeps_the_current_request_digest() {
    let mut input = invocation();
    let expected_digest = input.request.digest().unwrap();
    let receipt = compute_inline(input.clone(), &|| false).unwrap();
    assert_eq!(receipt.request_digest, expected_digest);
    let ComputationResult::Mutated {
        snapshot,
        receipt: mutation,
    } = receipt.result
    else {
        panic!()
    };
    assert!(mutation.transaction.is_none());
    input.snapshot = Some(snapshot);
    assert_eq!(
        compute_inline(input, &|| false).unwrap_err().code,
        FailureCode::InputInvalid
    );
}
