use mo_embedded_sdk::{Presentation, common::*, edit::*, model::*, template::*};
use std::collections::BTreeMap;

#[test]
fn caller_owned_template_values_restore_and_edit_without_changing_the_source() {
    let fixture: serde_json::Value = serde_json::from_str(include_str!(
        "../../../fixtures/presentations/playback/page.json"
    ))
    .unwrap();
    let document: Document = serde_json::from_value(fixture["page"]["document"].clone()).unwrap();
    let original = Presentation::create(document, &|| false).unwrap();
    let source = original.snapshot().clone();
    let definition = TemplateDefinition {
        format: TemplateVersion::V1,
        source: TemplateSource {
            document_id: source.document.id.clone(),
            revision: source.revision.clone(),
            semantic_digest: source.semantic_digest.clone(),
        },
        parameters: BTreeMap::new(),
    };
    assert!(matches!(
        original.prepare_template(
            definition.clone(),
            TemplateLimits {
                max_bytes: 1,
                ..Default::default()
            },
            &|| false
        ),
        Err(TemplateError::LimitExceeded(_))
    ));
    let template = original
        .prepare_template(definition, Default::default(), &|| false)
        .unwrap();
    let request = InstantiateRequest {
        request_id: RequestId::new("instantiate").unwrap(),
        template_digest: template.digest().clone(),
        document_id: DocumentId::new("independent").unwrap(),
        bindings: BTreeMap::new(),
    };
    let (instance, receipt) =
        Presentation::instantiate_template(&template, &request, &|| false).unwrap();
    assert_eq!(receipt.revision, instance.snapshot().revision);
    let mut edited = Presentation::from_snapshot(instance.into_snapshot()).unwrap();
    edited
        .edit(
            RequestId::new("edit-instance").unwrap(),
            vec![OperationEntry {
                operation_id: OperationId::new("title").unwrap(),
                operation: Operation::SetTitle {
                    title: "Changed independently".into(),
                },
            }],
            &|| false,
        )
        .unwrap();
    let (another, _) = Presentation::instantiate_template(&template, &request, &|| false).unwrap();
    assert_eq!(original.snapshot(), &source);
    assert_eq!(another.document().title, source.document.title);
    assert_eq!(edited.document().title, "Changed independently");
    assert!(matches!(
        Presentation::instantiate_template(&template, &request, &|| true),
        Err(TemplateError::Cancelled)
    ));
}
