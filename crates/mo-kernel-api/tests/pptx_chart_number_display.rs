#[path = "../../../tools/test-support/chart_number_display.rs"]
mod support;
use mo_kernel_api::compute_pptx_chart_labels_json;
use serde_json::Value;
#[test]
fn source_numeric_display_corpus_exports_exact_text_and_explicit_diagnostics() {
    for case in support::cases() {
        let response: Value = serde_json::from_str(&compute_pptx_chart_labels_json(
            &case.request.to_string(),
            &case.source,
        ))
        .unwrap();
        assert_eq!(response["status"], "planned", "{}: {response}", case.name);
        assert_eq!(
            response["labels"]["numberSymbols"],
            case.request["numberSymbols"]
        );
        for (idx, expected) in case.expected.iter().enumerate() {
            let display = &response["labels"]["labels"][idx]["formattedComponents"][0];
            if let Some(reason) = case.issue {
                assert_eq!(display["kind"], "unresolved", "{} {display}", case.name);
                assert_eq!(display["issue"]["reason"], reason, "{}", case.name);
            } else {
                assert_eq!(display["kind"], "number", "{} {display}", case.name);
                let mut text = String::new();
                for f in display["display"]["fragments"].as_array().unwrap() {
                    assert_eq!(f["kind"], "text");
                    text.push_str(f["value"].as_str().unwrap());
                }
                assert_eq!(text, *expected, "{} point {idx}", case.name);
            }
        }
        if let Ok(root) = std::env::var("MO_CHART_NUMBER_CASE_DIR") {
            let dir = std::path::Path::new(&root).join(&case.name);
            std::fs::create_dir(&dir).unwrap();
            std::fs::write(dir.join("source.pptx"), case.source).unwrap();
            std::fs::write(dir.join("request.json"), case.request.to_string()).unwrap();
            std::fs::write(
                dir.join("expected.json"),
                serde_json::to_string(
                    &serde_json::json!({"texts":case.expected,"issue":case.issue}),
                )
                .unwrap(),
            )
            .unwrap();
        }
    }
}
