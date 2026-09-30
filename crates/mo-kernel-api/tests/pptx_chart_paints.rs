#[path = "../../mo-presentation-source/tests/support/chart_paints.rs"]
mod support;
use mo_kernel_api::compute_pptx_chart_paints_json;
use mo_opc::{Package, PackageLimits};
use serde_json::{Value, json};
use support::*;
fn request(bytes: &[u8]) -> Value {
    let package = Package::open(bytes, bytes.len() as u64, PackageLimits::default(), &|| {
        false
    })
    .unwrap();
    json!({"expectedSourceSha256":package.sha256(),"object":{"part":"/ppt/slides/slide1.xml","nativeId":2},"profile":"ecma376-2016-draft-v1","context":{"systemColors":{},"placeholder":null}})
}
fn computed(bytes: &[u8], query: &Value) -> Value {
    serde_json::from_str(&compute_pptx_chart_paints_json(&query.to_string(), bytes)).unwrap()
}
#[test]
fn public_chart_paint_boundary_binds_actual_parts_and_rejects_unpinned_or_unknown_requests() {
    let bytes = fixture(
        r#"<c:spPr><a:solidFill><a:srgbClr val="CC8844"/></a:solidFill></c:spPr>"#,
        None,
        None,
        None,
    );
    let q = request(&bytes);
    let result = computed(&bytes, &q);
    assert_eq!(result["status"], "computed");
    assert_eq!(
        result["paints"]["declarations"][0]["colors"][0]["outcome"]["rgba8"],
        json!([204, 136, 68, 255])
    );
    let mut q = q.clone();
    q["expectedSourceSha256"] = json!("0".repeat(64));
    assert_eq!(computed(&bytes, &q)["error"]["code"], "SOURCE_CONFLICT");
    let mut q = request(&bytes);
    q["unexpected"] = json!(true);
    assert_eq!(computed(&bytes, &q)["error"]["code"], "INPUT_INVALID");
    let q = " ".repeat(mo_kernel_api::MAX_REQUEST_BYTES + 1);
    let result: Value = serde_json::from_str(&compute_pptx_chart_paints_json(&q, &bytes)).unwrap();
    assert_eq!(result["error"]["code"], "LIMIT_EXCEEDED");
}
#[test]
fn owned_chart_paint_inputs_are_exercised_by_the_public_boundary() {
    let theme = theme(r#"<a:srgbClr val="804020"/>"#);
    let mut cases = vec![];
    for (name, style) in [
        (
            "literal",
            r#"<a:solidFill><a:srgbClr val="193A62"/></a:solidFill>"#,
        ),
        (
            "mapped-theme",
            r#"<a:solidFill><a:schemeClr val="accent1"><a:lumMod val="50000"/><a:lumOff val="10000"/><a:alpha val="75000"/></a:schemeClr></a:solidFill>"#,
        ),
        (
            "gradient",
            r#"<a:gradFill><a:gsLst><a:gs pos="0"><a:srgbClr val="204080"/></a:gs><a:gs pos="100000"><a:schemeClr val="accent1"/></a:gs></a:gsLst><a:lin ang="0" scaled="0"/></a:gradFill>"#,
        ),
        (
            "pattern",
            r#"<a:pattFill prst="pct20"><a:fgClr><a:schemeClr val="accent1"/></a:fgClr><a:bgClr><a:srgbClr val="FFF8E0"/></a:bgClr></a:pattFill>"#,
        ),
        (
            "line",
            r#"<a:noFill/><a:ln w="9525"><a:solidFill><a:schemeClr val="accent2"/></a:solidFill><a:prstDash val="solid"/></a:ln>"#,
        ),
        (
            "system-fallback",
            r#"<a:solidFill><a:sysClr val="window" lastClr="CBA987"/></a:solidFill>"#,
        ),
        (
            "placeholder-unresolved",
            r#"<a:solidFill><a:schemeClr val="phClr"/></a:solidFill>"#,
        ),
        (
            "effect-retained",
            r#"<a:noFill/><a:effectLst><a:outerShdw blurRad="100"><a:srgbClr val="000000"/></a:outerShdw></a:effectLst>"#,
        ),
        ("duplicate-fill", r#"<a:noFill/><a:noFill/>"#),
    ] {
        let styles = format!("<c:spPr>{style}</c:spPr>");
        let bytes = fixture(&styles, None, None, Some(&theme));
        let q = request(&bytes);
        let status = if name == "duplicate-fill" {
            "error"
        } else {
            "computed"
        };
        assert_eq!(computed(&bytes, &q)["status"], status);
        cases.push((name.to_string(), bytes, q, status));
    }
    let bytes = fixture(
        r#"<c:spPr><a:solidFill><a:sysClr val="window"/></a:solidFill></c:spPr>"#,
        None,
        None,
        None,
    );
    for (name, color) in [
        ("missing-system", None),
        ("host-system", Some([12, 34, 56])),
    ] {
        let mut q = request(&bytes);
        if let Some(color) = color {
            q["context"]["systemColors"]["window"] = json!(color);
        }
        assert_eq!(computed(&bytes, &q)["status"], "computed");
        cases.push((name.to_string(), bytes.clone(), q, "computed"));
    }
    let mut q = request(&bytes);
    q["object"]["nativeId"] = json!(99);
    assert_eq!(computed(&bytes, &q)["status"], "error");
    cases.push(("wrong-object".into(), bytes, q, "error"));
    if let Ok(dir) = std::env::var("MO_CHART_PAINT_CASE_DIR") {
        use std::{fs::OpenOptions, io::Write, path::Path};
        let dir = Path::new(&dir);
        let mut manifest = vec![];
        for (name, bytes, q, status) in cases {
            let source = dir.join(format!("{name}.pptx"));
            let request = dir.join(format!("{name}.json"));
            OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&source)
                .unwrap()
                .write_all(&bytes)
                .unwrap();
            OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&request)
                .unwrap()
                .write_all(q.to_string().as_bytes())
                .unwrap();
            manifest.push(
                json!({"name":name,"source":source,"request":request,"expectedStatus":status}),
            );
        }
        OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(dir.join("cases.json"))
            .unwrap()
            .write_all(serde_json::to_string_pretty(&manifest).unwrap().as_bytes())
            .unwrap();
    }
}
