use mo_common::*;
use mo_presentation_edit::Snapshot;
use mo_presentation_template::*;
use serde_json::{Value, json};
use std::{cell::Cell, collections::BTreeMap};
#[path = "../../mo-presentation-edit/tests/support/mod.rs"]
mod support;

fn request() -> TemplateRequest {
    let mut document = support::document();
    support::connector(&mut document);
    let source = Snapshot::new(document, Default::default()).unwrap();
    let definition = TemplateDefinition {
        format: TemplateVersion::V1,
        source: TemplateSource::of(&source),
        parameters: BTreeMap::from([(
            TemplateParameterId::new("title").unwrap(),
            Parameter {
                label: "标题".into(),
                required: true,
                target: ParameterTarget::TextRun {
                    object: support::id(),
                    paragraph: support::paragraph_id(),
                    run: support::run_id(),
                    min_scalars: 0,
                    max_scalars: 200,
                },
            },
        )]),
    };
    TemplateRequest {
        version: TemplateComputationVersion::V1,
        source: source.into_record(),
        definition,
        action: TemplateAction::Describe {},
    }
}

fn calculate(input: &str) -> Value {
    serde_json::from_str(&compute_template_json(input, Default::default(), &|| false)).unwrap()
}

fn record(name: &str, input: &str, response: &Value) {
    if let Some(root) = std::env::var_os("MO_TEMPLATE_TEST_OUTPUT") {
        let root = std::path::PathBuf::from(root);
        std::fs::create_dir_all(&root).unwrap();
        std::fs::write(root.join(format!("{name}.request.json")), input).unwrap();
        std::fs::write(
            root.join(format!("{name}.response.json")),
            serde_json::to_vec(response).unwrap(),
        )
        .unwrap();
    }
}

#[test]
fn describe_then_instantiate_preserves_the_document_graph_and_is_repeatable() {
    let mut input = request();
    let describe = serde_json::to_string(&input).unwrap();
    let response = calculate(&describe);
    assert_eq!(response["status"], "described");
    assert_eq!(
        response["description"]["examples"]["title"],
        json!({"kind":"text","value":"A😀é中"})
    );
    record("describe", &describe, &response);
    let request = InstantiateRequest {
        request_id: RequestId::new("create:wire").unwrap(),
        template_digest: serde_json::from_value(response["description"]["templateDigest"].clone())
            .unwrap(),
        document_id: DocumentId::new("instance:wire").unwrap(),
        bindings: BTreeMap::from([(
            TemplateParameterId::new("title").unwrap(),
            BindingValue::Text("Unicode 中文 😀 <&>".into()),
        )]),
    };
    input.action = TemplateAction::Instantiate {
        request: request.clone(),
    };
    let wire = serde_json::to_string(&input).unwrap();
    let response = calculate(&wire);
    assert_eq!(response["status"], "instantiated");
    assert_eq!(response, calculate(&wire));
    let instance: TemplateInstance = serde_json::from_value(response["instance"].clone()).unwrap();
    let restored = Snapshot::restore(instance.snapshot.clone(), Default::default()).unwrap();
    assert_eq!(restored.document().id, request.document_id);
    assert_eq!(
        instance.receipt.source.document_id,
        input.source.document.id
    );
    assert_eq!(instance.receipt.revision, *restored.revision());
    assert_eq!(
        restored.document().objects[&ObjectId::new("connector:1").unwrap()],
        input.source.document.objects[&ObjectId::new("connector:1").unwrap()]
    );
    record("authored", &wire, &response);
}

#[test]
fn closed_wire_rejects_duplicate_keys_unknown_fields_and_tampered_source() {
    let value = serde_json::to_value(request()).unwrap();
    let mut cases = Vec::new();
    for field in ["permissions", "jobId", "storage", "unexpected"] {
        let mut bad = value.clone();
        bad[field] = json!({"forged":"authority"});
        cases.push((field, bad.to_string(), "INPUT_INVALID"));
    }
    let mut bad = value.clone();
    bad["action"]["path"] = json!("/not-a-core-input");
    cases.push(("action-extra", bad.to_string(), "INPUT_INVALID"));
    let mut bad = value.clone();
    bad["definition"]["parameters"]["title"]["target"]["inferred"] = json!(true);
    cases.push(("target-extra", bad.to_string(), "INPUT_INVALID"));
    let mut bad = value.clone();
    bad["source"]["document"]["title"] = json!("corrupted");
    cases.push(("source-integrity", bad.to_string(), "EDIT_REJECTED"));
    let mut bad = value.clone();
    bad["definition"]["source"]["revision"] = json!("a".repeat(64));
    cases.push(("source-pin", bad.to_string(), "SOURCE_CONFLICT"));
    let mut bad = value.clone();
    bad["version"] = json!("musteroffice.template-computation/unknown");
    cases.push(("version", bad.to_string(), "INPUT_INVALID"));
    let duplicate = value.to_string().replacen(
        "\"action\":",
        "\"action\":{\"kind\":\"describe\"},\"action\":",
        1,
    );
    cases.push(("duplicate", duplicate, "INPUT_INVALID"));
    let mut bad = value;
    bad["action"] = json!({"kind":"instantiate","request":{
        "requestId":"stale","documentId":"new", "bindings":{}, "templateDigest":"b".repeat(64)
    }});
    cases.push(("template-pin", bad.to_string(), "TEMPLATE_CONFLICT"));
    for (name, wire, code) in cases {
        let response = calculate(&wire);
        assert_eq!(response["status"], "error", "{name}");
        assert_eq!(response["error"]["code"], code, "{name}");
        assert!(response.get("instance").is_none());
        record(name, &wire, &response);
    }
}

#[test]
fn calculation_envelope_checks_byte_limits_and_each_cancellation_boundary() {
    let request = request();
    let wire = serde_json::to_string(&request).unwrap();
    let response: Value = serde_json::from_str(&compute_template_json(
        &wire,
        TemplateLimits {
            max_bytes: wire.len() - 1,
            ..Default::default()
        },
        &|| false,
    ))
    .unwrap();
    assert_eq!(response["error"]["code"], "LIMIT_EXCEEDED");
    let count = Cell::new(0);
    let expected = compute_template_json(&wire, Default::default(), &|| {
        count.set(count.get() + 1);
        false
    });
    for stop in 1..=count.get() {
        let calls = Cell::new(0);
        let actual: Value =
            serde_json::from_str(&compute_template_json(&wire, Default::default(), &|| {
                calls.set(calls.get() + 1);
                calls.get() == stop
            }))
            .unwrap();
        assert_eq!(actual["error"]["code"], "CANCELLED", "{stop}");
    }
    assert_eq!(
        expected,
        compute_template_json(&wire, Default::default(), &|| false)
    );
}
