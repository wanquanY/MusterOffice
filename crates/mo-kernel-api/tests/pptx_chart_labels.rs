#[path = "../../../tools/test-support/chart_labels.rs"]
mod support;
use mo_kernel_api::compute_pptx_chart_labels_json;
use serde_json::{Value, json};
use support::*;
#[test]
fn public_label_corpus_preserves_typed_results_and_exports_owned_parity_inputs() {
    for (name, bytes, request, status) in cases() {
        let response: Value = serde_json::from_str(&compute_pptx_chart_labels_json(
            &request.to_string(),
            &bytes,
        ))
        .unwrap();
        assert_eq!(response["status"], status, "{name}: {response}");
        if let Ok(root) = std::env::var("MO_CHART_LABEL_CASE_DIR") {
            let dir = std::path::Path::new(&root).join(name);
            std::fs::create_dir(&dir).unwrap();
            std::fs::write(dir.join("source.pptx"), bytes).unwrap();
            std::fs::write(dir.join("request.json"), request.to_string()).unwrap();
            std::fs::write(dir.join("expected-status.txt"), status).unwrap();
        }
    }
}
#[test]
fn label_api_checks_source_pin_request_fields_and_request_budget() {
    let b = package(&actual_like());
    let q = request(&b);
    let mut foreign = q.clone();
    foreign["expectedSourceSha256"] = json!("0".repeat(64));
    let r: Value =
        serde_json::from_str(&compute_pptx_chart_labels_json(&foreign.to_string(), &b)).unwrap();
    assert_eq!(r["error"]["code"], "SOURCE_CONFLICT");
    let mut unknown = q;
    unknown["extra"] = json!(true);
    for request in [
        unknown.to_string(),
        "{}".into(),
        " ".repeat(mo_kernel_api::MAX_REQUEST_BYTES + 1),
    ] {
        let r: Value = serde_json::from_str(&compute_pptx_chart_labels_json(&request, &b)).unwrap();
        assert_eq!(r["status"], "error");
        assert!(matches!(
            r["error"]["code"].as_str(),
            Some("INPUT_INVALID" | "LIMIT_EXCEEDED")
        ));
    }
}
