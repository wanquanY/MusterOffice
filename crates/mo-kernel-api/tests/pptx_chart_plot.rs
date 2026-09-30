#[path = "../../../tools/test-support/chart_plot.rs"]
mod support;
use mo_kernel_api::compile_pptx_chart_plot_json;
use serde_json::{Value, json};
use support::*;
fn response(bytes: &[u8], request: &Value) -> Value {
    serde_json::from_str(&compile_pptx_chart_plot_json(&request.to_string(), bytes)).unwrap()
}
#[test]
fn public_plot_boundary_owns_style_composition_and_rejects_stale_source_and_unknown_options() {
    let bytes = fixture(BASE, "", &["1", "2", "3"]);
    let q = request(&bytes);
    let r = response(&bytes, &q);
    assert_eq!(r["status"], "compiled");
    assert_eq!(r["plot"]["scene"]["instances"].as_array().unwrap().len(), 6);
    let mut stale = q.clone();
    stale["geometry"]["expectedSourceSha256"] = json!("0".repeat(64));
    assert_eq!(response(&bytes, &stale)["error"]["code"], "SOURCE_CONFLICT");
    let mut unknown = q;
    unknown["ignoreUnsupportedEffects"] = json!(true);
    assert_eq!(response(&bytes, &unknown)["error"]["code"], "INPUT_INVALID");
    let r: Value = serde_json::from_str(&compile_pptx_chart_plot_json(
        &" ".repeat(mo_kernel_api::MAX_REQUEST_BYTES + 1),
        &bytes,
    ))
    .unwrap();
    assert_eq!(r["error"]["code"], "LIMIT_EXCEEDED");
}
#[test]
fn owned_plot_corpus_runs_through_the_actual_public_boundary() {
    let dir = std::env::var("MO_CHART_PLOT_CASE_DIR").ok();
    let mut manifest = vec![];
    for (name, bytes, status) in cases() {
        let q = request(&bytes);
        let r = response(&bytes, &q);
        assert_eq!(r["status"], status, "{name}: {r}");
        if let Some(dir) = &dir {
            use std::{fs::OpenOptions, io::Write, path::Path};
            let source = Path::new(dir).join(format!("{name}.pptx"));
            let request = Path::new(dir).join(format!("{name}.json"));
            for (p, contents) in [
                (&source, bytes.as_slice()),
                (&request, q.to_string().as_bytes()),
            ] {
                OpenOptions::new()
                    .write(true)
                    .create_new(true)
                    .open(p)
                    .unwrap()
                    .write_all(contents)
                    .unwrap();
            }
            manifest.push(
                json!({"name":name,"source":source,"request":request,"expectedStatus":status}),
            );
        }
    }
    if let Some(dir) = dir {
        use std::{fs::OpenOptions, io::Write, path::Path};
        OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(Path::new(&dir).join("cases.json"))
            .unwrap()
            .write_all(serde_json::to_string_pretty(&manifest).unwrap().as_bytes())
            .unwrap();
    }
}
