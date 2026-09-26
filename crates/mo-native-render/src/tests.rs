use super::*;

#[test]
fn transport_preserves_structured_diagnostics_without_api_dependency() {
    let error = serde_json::json!({"stage":"page","error":{"code":"UNSUPPORTED", "location":{"part":"/ppt/slides/slide1.xml","object":5}}, "text":{"kind":"font","details":[1,2]}});
    let response: PptxResourcePageRasterResponse =
        serde_json::from_value(serde_json::json!({"status":"error","error":error})).unwrap();
    let Err(DeliveryError::Preview {
        diagnostic: Some(detail),
        ..
    }) = response_image(response.clone(), vec![])
    else {
        panic!("diagnostic lost");
    };
    assert_eq!(*detail, error);
    assert!(matches!(
        response_image(response, vec![0]),
        Err(DeliveryError::Invalid(_))
    ));
}

#[test]
fn document_protocol_does_not_multiply_font_metadata_by_page_count() {
    let fixture: serde_json::Value = serde_json::from_str(include_str!(
        "../../../fixtures/presentations/delivery/input.json"
    ))
    .unwrap();
    let page: PreviewRequest = serde_json::from_value(serde_json::json!({
        "page": {
            "expectedSourceSha256": "0".repeat(64),
            "slide": "/ppt/slides/slide1.xml",
            "profile": "drawingml-static-solid-page-v1-draft",
            "viewport": {"width": 1, "height": 1, "origin": {"x": "0", "y": "0"},
                "scale": {"numerator": 1, "denominator": 1},
                "coordinateTolerance": "1048576", "background": [0, 0, 0, 0]},
            "colorContext": {"systemColors": {}, "placeholder": null}
        },
        "imageSource": "embeddedSnapshot",
        "sampling": "nearest"
    }))
    .unwrap();
    let mut batch = PptxResourceDocumentRequest {
        profile: PptxResourcePageProfile::NativeResourcesDraftV1,
        pages: vec![page; 256],
        fonts: Some(serde_json::from_value(fixture["settings"]["fonts"].clone()).unwrap()),
    };
    // Exercise transport admission, independently of later semantic font checks.
    batch.fonts.as_mut().unwrap().typefaces[0].typeface = "x".repeat(1024 * 1024);
    let bytes = request_json(&batch).unwrap();
    assert!(
        bytes.len() < 2 * 1024 * 1024,
        "manifest was multiplied by page count"
    );
    let decoded: PptxResourceDocumentRequest = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(decoded.pages.len(), 256);
    assert_eq!(
        decoded.fonts.unwrap().typefaces[0].typeface.len(),
        1024 * 1024
    );
    batch.fonts.as_mut().unwrap().typefaces[0].typeface = "x".repeat(protocol::MAX_REQUEST_BYTES);
    assert!(matches!(request_json(&batch), Err(DeliveryError::Limit(_))));
}
