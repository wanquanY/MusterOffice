use mo_kernel_api::{DeliveryPlaybackResponse, PptxFailureCode, prepare_delivery_playback_at};
use serde_json::{Value, json};

fn request() -> Value {
    json!({"width":640,"delivery":serde_json::from_str::<Value>(include_str!(
        "../../../fixtures/presentations/delivery-receive/request.json"
    )).unwrap()})
}
const BYTES: &[u8] = include_bytes!("../../../fixtures/presentations/delivery-receive/assets.bin");

fn run(value: &Value, bytes: &[u8]) -> DeliveryPlaybackResponse {
    prepare_delivery_playback_at(&value.to_string(), &bytes, bytes.len() as u64, &|| false)
}

#[test]
fn derives_transport_independent_playback_inputs_after_full_admission() {
    let request = request();
    let DeliveryPlaybackResponse::Prepared { inputs } = run(&request, BYTES) else {
        panic!("valid delivery")
    };
    assert_eq!(inputs.pages.len(), 2);
    assert_eq!(
        inputs.source.id.to_string(),
        request["delivery"]["bundle"]["pptxAssetId"]
    );
    assert_eq!(
        inputs.revision.to_string(),
        request["delivery"]["expected"]["revision"]
    );
    assert_eq!(inputs.pages[0].request.page.viewport.height, 360);
    assert!(inputs.font_bundle.is_some());
    assert!(inputs.fonts.is_some());
}

#[test]
fn playback_preparation_does_not_weaken_delivery_admission() {
    let mut cases = Vec::new();
    for width in [0, 8193] {
        let mut v = request();
        v["width"] = json!(width);
        cases.push(v);
    }
    let mut v = request();
    v["delivery"]["expected"]["revision"] = json!("0".repeat(64));
    cases.push(v);
    let mut v = request();
    v["delivery"]["contents"].as_array_mut().unwrap().pop();
    cases.push(v);
    let mut v = request();
    v["delivery"]["contents"][1]["byteOffset"] = json!("0");
    cases.push(v);
    let mut v = request();
    v["delivery"]["bundle"]["previews"]
        .as_array_mut()
        .unwrap()
        .reverse();
    cases.push(v);
    let mut v = request();
    v["slide"] = json!("ppt/slides/other.xml");
    cases.push(v);
    for value in cases {
        assert!(matches!(
            run(&value, BYTES),
            DeliveryPlaybackResponse::Error { .. }
        ));
    }
    let mut corrupt = BYTES.to_vec();
    corrupt[0] ^= 1;
    assert!(matches!(
        run(&request(), &corrupt),
        DeliveryPlaybackResponse::Error { .. }
    ));
    let result =
        prepare_delivery_playback_at(&request().to_string(), &BYTES, BYTES.len() as u64, &|| true);
    assert!(
        matches!(result, DeliveryPlaybackResponse::Error { error } if matches!(error.code, PptxFailureCode::Cancelled))
    );
}
