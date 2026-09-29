use mo_embedded_sdk::{Inputs, common::*, edit::*, execute, operation::*, template::*};
use std::{cell::Cell, collections::BTreeMap};

fn input() -> Invocation {
    let fixture: serde_json::Value = serde_json::from_str(include_str!(
        "../../../fixtures/presentations/playback/page.json"
    ))
    .unwrap();
    let source = Snapshot::new(
        serde_json::from_value(fixture["page"]["document"].clone()).unwrap(),
        Default::default(),
    )
    .unwrap();
    let definition = TemplateDefinition {
        format: TemplateVersion::V1,
        source: TemplateSource::of(&source),
        parameters: BTreeMap::new(),
    };
    let template = Template::new(
        source.clone(),
        definition.clone(),
        Default::default(),
        &|| false,
    )
    .unwrap();
    Invocation {
        request: OperationRequest {
            contract_version: ContractVersion::V1,
            request_id: RequestId::new("template-dispatch").unwrap(),
            profile_id: OperationProfile::AuthorModel,
            action: DocumentAction::InstantiateTemplate {
                document_id: DocumentId::new("new-instance").unwrap(),
                definition: Box::new(definition),
                template_digest: template.digest().clone(),
                bindings: BTreeMap::new(),
            },
        },
        snapshot: Some(Box::new(source.into_record())),
    }
}

#[test]
fn sdk_description_uses_the_same_read_only_computation_as_wasm() {
    let mut input = input();
    let DocumentAction::InstantiateTemplate { definition, .. } = input.request.action else {
        panic!()
    };
    input.request.action = DocumentAction::DescribeTemplate { definition };
    let expected = compute_inline(input.clone(), &|| false).unwrap();
    let output = execute(input, &Inputs::new(), None, &|| false).unwrap();
    assert!(output.export().is_none());
    assert!(matches!(
        output.receipt().result,
        ComputationResult::DescribedTemplate { .. }
    ));
    assert_eq!(
        serde_json::to_value(output.receipt()).unwrap(),
        serde_json::to_value(expected).unwrap()
    );
}
#[test]
fn native_sdk_and_inline_dispatch_share_the_whole_receipt_and_release_no_binary() {
    let input = input();
    let expected = compute_inline(input.clone(), &|| false).unwrap();
    let output = execute(input.clone(), &Inputs::new(), None, &|| false).unwrap();
    assert!(output.export().is_none());
    assert_eq!(
        serde_json::to_value(output.receipt()).unwrap(),
        serde_json::to_value(&expected).unwrap()
    );
    let ComputationResult::Mutated { snapshot, receipt } = &output.receipt().result else {
        panic!()
    };
    let mut original = input.snapshot.as_ref().unwrap().document.clone();
    original.id = DocumentId::new("new-instance").unwrap();
    assert_eq!(snapshot.document, original);
    assert_eq!(
        receipt.template.as_ref().unwrap().source.revision,
        input.snapshot.as_ref().unwrap().revision
    );
    let calls = Cell::new(0);
    execute(input.clone(), &Inputs::new(), None, &|| {
        calls.set(calls.get() + 1);
        false
    })
    .unwrap();
    for stop in 1..=calls.get() {
        let count = Cell::new(0);
        let result = execute(input.clone(), &Inputs::new(), None, &|| {
            count.set(count.get() + 1);
            count.get() == stop
        });
        assert_eq!(result.err().unwrap().code, FailureCode::Cancelled);
    }
}
