use mo_kernel_api::dispatch_json;
use serde_json::{Value, json};
fn call(q: Value) -> Value {
    serde_json::from_str(&dispatch_json(&q.to_string())).unwrap()
}
#[test]
fn public_capabilities_bind_snapshot_and_rejections_preserve_structured_reason() {
    let schema: Value = serde_json::from_str(include_str!(
        "../../../contracts/generated/document.schema.json"
    ))
    .unwrap();
    let initialized = call(json!({"operation":"initialize","document":schema["examples"][0]}));
    let snapshot = &initialized["snapshot"];
    let (object, body) = snapshot["document"]["objects"]
        .as_object()
        .unwrap()
        .iter()
        .find_map(|(id, o)| {
            (o["content"]["kind"] == "shape" && o["content"]["text"].is_object())
                .then_some((id, &o["content"]["text"]))
        })
        .unwrap();
    let a = json!({"paragraph":body["paragraphs"][0]["id"],"scalarOffset":0,"affinity":"after"});
    let selected = json!({"anchor":a,"focus":a});
    let request = json!({"operation":"textCapabilities","snapshot":snapshot,"query":{"object":object,"selection":selected}});
    let response = call(request.clone());
    assert_eq!(response["status"], "textCapabilities");
    assert_eq!(response["capabilities"]["revision"], snapshot["revision"]);
    let reason = &response["capabilities"]["characterStyle"]["reason"];
    assert_eq!(reason["kind"], "nonemptySelectionRequired");
    let result = call(
        json!({"operation":"prepareText","snapshot":snapshot,"command":{
            "documentId":snapshot["document"]["id"],"baseRevision":snapshot["revision"],"requestId":"style","operationId":"style",
            "object":object,"action":{"kind":"setCharacterStyle","selection":selected,"patch":{}}
        }}),
    );
    assert_eq!(result["status"], "error");
    assert_eq!(result["error"]["textRestriction"], *reason);
    assert_eq!(result["error"]["code"], "INPUT_INVALID");
    let mut tampered = request.clone();
    tampered["snapshot"]["document"]["title"] = json!("forged");
    assert_eq!(call(tampered)["status"], "error");
    let mut extra = request;
    extra["query"]["unknown"] = json!(true);
    assert_eq!(call(extra)["status"], "error");
}
