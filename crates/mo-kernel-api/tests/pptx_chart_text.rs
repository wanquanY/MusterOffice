#[path = "../../../tools/test-support/chart_text.rs"]
mod support;
use mo_kernel_api::compute_pptx_chart_labels_json;
use serde_json::Value;
#[test]
fn label_text_uses_native_roots_and_preserves_unresolved_cascade_outcomes() {
    for (name, bytes, request, expected) in support::cases() {
        let response: Value = serde_json::from_str(&compute_pptx_chart_labels_json(
            &request.to_string(),
            &bytes,
        ))
        .unwrap();
        assert_eq!(response["status"], "planned", "{name}: {response}");
        let labels = &response["labels"];
        let index = labels["labels"][0]["textCascade"].as_u64().unwrap() as usize;
        assert_eq!(
            labels["textCascades"][index]["status"], expected,
            "{name}: {response}"
        );
        if let Ok(root) = std::env::var("MO_CHART_TEXT_CASE_DIR") {
            let dir = std::path::Path::new(&root).join(name);
            std::fs::create_dir(&dir).unwrap();
            std::fs::write(dir.join("source.pptx"), bytes).unwrap();
            std::fs::write(dir.join("request.json"), request.to_string()).unwrap();
            std::fs::write(dir.join("expected-status.txt"), "planned").unwrap();
        }
    }
}
