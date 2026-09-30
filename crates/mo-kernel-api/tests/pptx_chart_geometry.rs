#[path = "../../mo-presentation-source/tests/support/charts.rs"]
mod support;
use mo_kernel_api::{PptxChartGeometryResponse, compile_pptx_chart_geometry_json};
use mo_opc::{Package, PackageLimits};
use serde_json::{Value, json};
use support::*;

fn input() -> (Vec<u8>, Value) {
    let series = r#"<c:ser><c:idx val="7"/><c:order val="0"/><c:val><c:numLit><c:ptCount val="2"/><c:pt idx="1"><c:v>2</c:v></c:pt><c:pt idx="0"><c:v>1</c:v></c:pt></c:numLit></c:val></c:ser>"#;
    let xml = format!(
        r#"<c:chartSpace xmlns:c="{C}"><c:chart><c:plotArea><c:doughnutChart>{series}<c:firstSliceAng val="0"/><c:holeSize val="50"/></c:doughnutChart></c:plotArea></c:chart></c:chartSpace>"#
    );
    let bytes = package(&xml, &frame(2), CT, "chart", false);
    let package = Package::open(
        bytes.as_slice(),
        bytes.len() as u64,
        PackageLimits::default(),
        &|| false,
    )
    .unwrap();
    let request = json!({"expectedSourceSha256":package.sha256(),"object":{"part":"/ppt/slides/slide1.xml","nativeId":2},"plotSourceOrdinal":3,"profile":"source-cache-declared-circular-plot-v1-draft","center":{"x":"0","y":"0"},"outerRadius":"4294967296000","coordinateTolerance":"16777216","negativeWeights":"reject"});
    (bytes, request)
}
#[test]
fn public_source_geometry_binds_native_object_formula_and_preserves_pptx_bytes() {
    let (bytes, q) = input();
    let before = bytes.clone();
    let r: PptxChartGeometryResponse =
        serde_json::from_str(&compile_pptx_chart_geometry_json(&q.to_string(), &bytes)).unwrap();
    let PptxChartGeometryResponse::Compiled { geometry } = r else {
        panic!("{r:?}")
    };
    assert_eq!(geometry.object.native_id, 2);
    assert_eq!(geometry.series[0].index, 7);
    assert_eq!(geometry.series[0].geometry.paths.len(), 2);
    assert_eq!(geometry.series[0].points[0].index, 0);
    assert_eq!(bytes, before);
    let mut stale = q.clone();
    stale["expectedSourceSha256"] = json!("0".repeat(64));
    let r: Value = serde_json::from_str(&compile_pptx_chart_geometry_json(
        &stale.to_string(),
        &bytes,
    ))
    .unwrap();
    assert_eq!(r["error"]["code"], "SOURCE_CONFLICT");
    let mut unknown = q;
    unknown["ignoreSourceErrors"] = json!(true);
    let r: Value = serde_json::from_str(&compile_pptx_chart_geometry_json(
        &unknown.to_string(),
        &bytes,
    ))
    .unwrap();
    assert_eq!(r["error"]["code"], "INPUT_INVALID");
    let oversized = " ".repeat(mo_kernel_api::MAX_REQUEST_BYTES + 1);
    let r: Value =
        serde_json::from_str(&compile_pptx_chart_geometry_json(&oversized, &bytes)).unwrap();
    assert_eq!(r["error"]["code"], "LIMIT_EXCEEDED");
}
