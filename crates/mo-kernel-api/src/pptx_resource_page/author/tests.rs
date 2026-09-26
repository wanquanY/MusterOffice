use super::*;
use mo_common::ByteLength;
use mo_presentation_compile::source_resource_page::protocol::AuthorResourceRange;
struct NoComponents;
impl mo_image::ImageDecoder for NoComponents {
    fn decode(&mut self, _: &[u8]) -> Result<mo_image::DecoderReply, mo_image::ImageError> {
        panic!("invalid batch reached image component")
    }
    fn invalidate(&mut self) {}
}
impl mo_raster::RasterBackend for NoComponents {
    fn raster(&mut self, _: &[u32]) -> Result<mo_raster::BackendReply, mo_raster::RasterError> {
        panic!("invalid batch reached raster component")
    }
    fn invalidate(&mut self) {}
}
fn input() -> AuthorResourceDocumentRequest {
    let fixture: serde_json::Value = serde_json::from_str(include_str!(
        "../../../../../fixtures/presentations/native-export/request.json"
    ))
    .unwrap();
    let document = serde_json::from_value(fixture["document"].clone()).unwrap();
    let defaults = serde_json::from_value(fixture["defaults"].clone()).unwrap();
    let plan = AuthorPlan::new(&document, &defaults, Default::default(), &|| false).unwrap();
    let page = serde_json::from_value(serde_json::json!({
        "page": {"expectedSourceSha256":plan.identity(), "slide":"/ppt/slides/slide1.xml", "profile":"drawingml-static-solid-page-v1-draft",
        "viewport":{"width":160,"height":90,"origin":{"x":"0","y":"0"},"scale":{"numerator":1,"denominator":76200},"coordinateTolerance":"1048576","background":[0,0,0,0]},
        "colorContext":{"systemColors":{},"placeholder":null}},"imageSource":"embeddedSnapshot","sampling":"nearest"
    })).unwrap();
    let resources = vec![AuthorResourceRange {
        id: plan.bindings().images.keys().next().unwrap().clone(),
        offset: ByteLength::new(0),
        byte_length: ByteLength::new(1),
    }];
    AuthorResourceDocumentRequest {
        profile: PptxResourcePageProfile::NativeResourcesDraftV1,
        document,
        defaults,
        resources,
        pages: vec![page],
        fonts: None,
    }
}
#[test]
fn document_admission_rejects_cross_identity_ranges_and_cancel_before_components() {
    for mode in 0..7 {
        let mut request = input();
        match mode {
            0 => request.pages[0].page.expected_source_sha256 = Digest::from_sha256([0; 32]),
            1 => request.resources[0].offset = ByteLength::new(1),
            2 => request.resources[0].byte_length = ByteLength::new(2),
            3 => request.resources[0].id = "resource:unknown".to_owned().try_into().unwrap(),
            4 => request.resources.clear(),
            5 => request.pages.clear(),
            _ => (),
        }
        let mut frames = 0;
        render_author_resource_document(
            &request,
            &[1],
            &[],
            PptxResourcePageBackends {
                decoder: &mut NoComponents,
                text: None,
                raster: &mut NoComponents,
            },
            &|| mode == 6,
            &mut |frame, pixels| -> Result<(), ()> {
                assert!(matches!(
                    frame,
                    PptxResourcePageRasterResponse::Error { .. }
                ));
                assert!(pixels.is_empty());
                frames += 1;
                Ok(())
            },
        )
        .unwrap();
        assert_eq!(frames, 1);
    }
    let result = render_author_resource_document(
        &input(),
        &[1, 2],
        &[],
        PptxResourcePageBackends {
            decoder: &mut NoComponents,
            text: None,
            raster: &mut NoComponents,
        },
        &|| false,
        &mut |_, _| Err("closed output"),
    );
    assert_eq!(result, Err("closed output"));
}
