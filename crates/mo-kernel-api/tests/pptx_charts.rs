#[path = "../../mo-presentation-source/tests/support/charts.rs"]
mod fixture;
use mo_kernel_api::{PptxChartsResponse, SourceChartQuery, inspect_pptx_charts_json};
use mo_opc::{Package, PackageLimits};
fn case() -> (String, Vec<u8>) {
    let bytes = fixture::package(
        &fixture::chart(&fixture::series()),
        &fixture::frame(2),
        fixture::CT,
        "chart",
        false,
    );
    let package = Package::open(
        bytes.as_slice(),
        bytes.len() as u64,
        PackageLimits::default(),
        &|| false,
    )
    .unwrap();
    let request = serde_json::to_string(&SourceChartQuery {
        expected_source_sha256: package.sha256().clone(),
        surface: "/ppt/slides/slide1.xml".into(),
    })
    .unwrap();
    (request, bytes)
}
#[test]
fn public_chart_query_is_source_bound_and_never_promises_rendering() {
    let (request, bytes) = case();
    let response = inspect_pptx_charts_json(&request, &bytes);
    let PptxChartsResponse::Inspected { charts } = serde_json::from_str(&response).unwrap() else {
        panic!("{response}")
    };
    assert_eq!(charts.charts.len(), 1);
    assert_eq!(charts.charts[0].plots[0].series[0].channels.len(), 3);
    let mut foreign: serde_json::Value = serde_json::from_str(&request).unwrap();
    foreign["expectedSourceSha256"] = serde_json::Value::String("0".repeat(64));
    let error: serde_json::Value =
        serde_json::from_str(&inspect_pptx_charts_json(&foreign.to_string(), &bytes)).unwrap();
    assert_eq!(error["error"]["code"], "SOURCE_CONFLICT");
    // Optional owned-fixture output for public CLI/WASM parity, never user source.
    if let Ok(dir) = std::env::var("MUSTER_OFFICE_CHART_FIXTURE_OUTPUT") {
        let dir = std::path::PathBuf::from(dir);
        std::fs::create_dir(&dir).unwrap();
        std::fs::write(dir.join("request.json"), request).unwrap();
        std::fs::write(dir.join("source.pptx"), bytes).unwrap();
        std::fs::write(dir.join("response.json"), response).unwrap();
    }
}
#[test]
fn chart_query_input_validation_is_typed() {
    let (request, bytes) = case();
    for input in [
        "{}".to_owned(),
        format!("{{\"extra\":1,{}", &request[1..]),
        " ".repeat(mo_kernel_api::MAX_REQUEST_BYTES + 1),
    ] {
        let value: serde_json::Value =
            serde_json::from_str(&inspect_pptx_charts_json(&input, &bytes)).unwrap();
        assert_eq!(value["status"], "error");
        assert!(matches!(
            value["error"]["code"].as_str(),
            Some("INPUT_INVALID" | "LIMIT_EXCEEDED")
        ));
    }
}
