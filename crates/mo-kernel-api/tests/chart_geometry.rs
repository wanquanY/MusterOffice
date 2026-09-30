use mo_kernel_api::{
    ChartGeometryFailureCode as Code, ChartGeometryResponse, compile_chart_geometry_json,
};
use serde_json::json;

fn input() -> serde_json::Value {
    json!({"sectors":{"startTurn":"0","direction":"clockwise","negativeWeights":"reject",
        "weights":[{"pointIndex":7,"value":"1"},{"pointIndex":42,"value":"2"}]},
        "center":{"x":"0","y":"0"},"outerRadius":"4294967296000","innerRadius":"2147483648000","coordinateTolerance":"4194304"})
}
fn response(input: &str, cancel: bool) -> ChartGeometryResponse {
    serde_json::from_str(&compile_chart_geometry_json(input, &|| cancel)).unwrap()
}
#[test]
fn public_geometry_preserves_ids_paths_bounds_and_precise_failures() {
    let ChartGeometryResponse::Computed { geometry } = response(&input().to_string(), false) else {
        panic!("geometry");
    };
    assert_eq!(geometry.paths[0].point_index, 7);
    assert!(!geometry.paths[0].commands.is_empty());
    assert!(
        geometry
            .paths
            .iter()
            .all(|p| p.coordinate_error_bound.raw() <= 4194304)
    );
    let mut invalid = input();
    invalid["sectors"]["weights"][1]["value"] = json!("-2");
    let ChartGeometryResponse::Error { error } = response(&invalid.to_string(), false) else {
        panic!()
    };
    assert_eq!(error.code, Code::InputInvalid);
    assert_eq!(error.point_index, Some(42));
    invalid["sectors"]["weights"][1]["value"] = json!("1e-4000");
    let ChartGeometryResponse::Error { error } = response(&invalid.to_string(), false) else {
        panic!()
    };
    assert_eq!(error.code, Code::PrecisionExceeded);
}
#[test]
fn strict_wire_does_not_accept_bypass_fields_or_unbounded_input() {
    let mut unknown = input();
    unknown["skipPrecision"] = json!(true);
    let mut float = input();
    float["outerRadius"] = json!(1000);
    for json in [
        unknown.to_string(),
        float.to_string(),
        "{\"center\":{},\"center\":{}}".to_owned(),
    ] {
        let ChartGeometryResponse::Error { error } = response(&json, false) else {
            panic!()
        };
        assert_eq!(error.code, Code::InputInvalid);
    }
    let ChartGeometryResponse::Error { error } = response("{}", true) else {
        panic!()
    };
    assert_eq!(error.code, Code::Cancelled);
    let ChartGeometryResponse::Error { error } =
        response(&" ".repeat(mo_kernel_api::MAX_REQUEST_BYTES + 1), false)
    else {
        panic!()
    };
    assert_eq!(error.code, Code::LimitExceeded);
}
