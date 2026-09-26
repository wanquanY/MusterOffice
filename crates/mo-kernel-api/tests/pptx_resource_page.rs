#[allow(dead_code)]
#[path = "../../../tools/test-support/source_resource_page.rs"]
mod support;
use mo_image::{DecoderReply, ImageDecoder, ImageError};
use mo_kernel_api::*;
use mo_raster::{BackendReply, RasterBackend, RasterError};
use mo_text::{TextError, backend::TextBackend};
use support::*;
#[derive(Default)]
struct Decoder {
    calls: u32,
    invalidated: u32,
}
impl ImageDecoder for Decoder {
    fn decode(&mut self, _: &[u8]) -> Result<DecoderReply, ImageError> {
        self.calls += 1;
        Err(ImageError::Host("owned decoder error"))
    }
    fn invalidate(&mut self) {
        self.invalidated += 1;
    }
}
#[derive(Default)]
struct Raster {
    calls: u32,
}
impl RasterBackend for Raster {
    fn raster(&mut self, _: &[u32]) -> Result<BackendReply, RasterError> {
        self.calls += 1;
        panic!("unexpected raster")
    }
    fn raster_images(&mut self, _: &[u32], _: &[u8]) -> Result<BackendReply, RasterError> {
        self.calls += 1;
        panic!("unexpected image raster")
    }
    fn invalidate(&mut self) {}
}
#[derive(Default)]
struct Text {
    calls: u32,
}
impl TextBackend for Text {
    fn shape_batch(&mut self, _: &[u8], _: &[u32]) -> Result<Vec<u32>, TextError> {
        self.calls += 1;
        panic!("unexpected text")
    }
    fn invalidate(&mut self) {}
}
fn query(bytes: &[u8]) -> PptxResourcePageRequest {
    PptxResourcePageRequest {
        profile: PptxResourcePageProfile::NativeResourcesDraftV1,
        page: request(&read(bytes)),
        image_source: mo_pptx::source::images::ImageSourceSelection::EmbeddedSnapshot,
        sampling: mo_raster::ImageSampling::Nearest,
        fonts: None,
    }
}
fn reject(
    q: &str,
    source: &[u8],
    fonts: &[u8],
    check: &dyn Fn() -> bool,
) -> (serde_json::Value, u32) {
    let (mut d, mut t, mut r) = (Decoder::default(), Text::default(), Raster::default());
    let (metadata, pixels) = render_pptx_resource_page_json(
        q,
        source,
        fonts,
        PptxResourcePageBackends {
            decoder: &mut d,
            text: Some(&mut t),
            raster: &mut r,
        },
        check,
    );
    assert!(pixels.is_empty());
    assert_eq!((t.calls, r.calls), (0, 0));
    assert!(matches!(
        mo_common::from_json_str::<PptxResourcePageRasterResponse>(&metadata).unwrap(),
        PptxResourcePageRasterResponse::Error { .. }
    ));
    assert_eq!(d.calls, d.invalidated);
    (
        serde_json::from_str::<serde_json::Value>(&metadata).unwrap()["error"].clone(),
        d.calls,
    )
}
#[test]
fn resource_page_json_identity_and_font_capabilities_fail_before_decode() {
    let source = image_fixture(&picture(
        42,
        "",
        &blip("owned-image", "", STRETCH),
        "<a:noFill/>",
    ));
    let q = query(&source);
    let json = serde_json::to_string(&q).unwrap();
    let (e, n) = reject(&json, &source, &[], &|| true);
    assert_eq!(n, 0);
    assert_eq!(e["error"]["code"], "CANCELLED");
    let (e, n) = reject(&json, &source, b"unclaimed fonts", &|| false);
    assert_eq!(n, 0);
    assert_eq!(e["stage"], "request");
    let (e, n) = reject(
        &json.replace("drawingml-resource-page-q32-v1-draft", "html"),
        &source,
        &[],
        &|| false,
    );
    assert_eq!(n, 0);
    assert_eq!(e["stage"], "request");
    let (e, n) = reject(
        &json.replacen("\"sampling\":", "\"sampling\":\"linear\",\"sampling\":", 1),
        &source,
        &[],
        &|| false,
    );
    assert_eq!(n, 0);
    assert_eq!(e["stage"], "request");
    let mut q = q.clone();
    q.page.expected_source_sha256 = mo_common::Digest::from_sha256([0; 32]);
    let (e, n) = reject(&serde_json::to_string(&q).unwrap(), &source, &[], &|| false);
    assert_eq!(n, 0);
    assert_eq!(e["error"]["code"], "SOURCE_CONFLICT");
    let mut q = query(&source);
    q.fonts = Some(author().manifest);
    let (e, n) = reject(&serde_json::to_string(&q).unwrap(), &source, &[], &|| false);
    assert_eq!(n, 0);
    assert_eq!(e["stage"], "fonts");
}
#[test]
fn resource_page_preserves_image_prerequisite_and_component_failure_ownership() {
    for (extra, image) in [
        ("rotWithShape=\"0\"", "stationaryOrientation"),
        ("", "resource"),
    ] {
        let source = image_fixture(&picture(
            42,
            "",
            &blip(
                if extra.is_empty() {
                    "missing"
                } else {
                    "owned-image"
                },
                extra,
                STRETCH,
            ),
            "<a:noFill/>",
        ));
        let (e, n) = reject(
            &serde_json::to_string(&query(&source)).unwrap(),
            &source,
            &[],
            &|| false,
        );
        assert_eq!(n, 0);
        assert_eq!(e["stage"], "page");
        assert_eq!(e["image"]["kind"], image);
        assert_eq!(e["error"]["location"]["object"], 42);
    }
    let source = image_fixture(&picture(
        42,
        "",
        &blip("owned-image", "", STRETCH),
        "<a:noFill/>",
    ));
    let (e, n) = reject(
        &serde_json::to_string(&query(&source)).unwrap(),
        &source,
        &[],
        &|| false,
    );
    assert_eq!(n, 1);
    assert_eq!(e["error"]["code"], "HOST_FAILURE");
    assert_eq!(e["error"]["location"]["object"], 42);
}

#[test]
fn prepared_document_rejects_switched_source_or_fonts_before_using_backends() {
    let source = image_fixture(&picture(
        42,
        "",
        &blip("owned-image", "", STRETCH),
        "<a:noFill/>",
    ));
    let original = query(&source);
    let prepared = prepare_pptx_resource_document(
        &original,
        source.as_slice(),
        source.len() as u64,
        &[],
        &|| false,
    )
    .unwrap();
    for switch_source in [true, false] {
        let mut request = original.clone();
        if switch_source {
            request.page.expected_source_sha256 = mo_common::Digest::from_sha256([9; 32]);
        } else {
            request.fonts = Some(author().manifest);
        }
        let (mut decoder, mut text, mut raster) =
            (Decoder::default(), Text::default(), Raster::default());
        let (response, pixels) = render_prepared_pptx_resource_page(
            &prepared,
            &request,
            PptxResourcePageBackends {
                decoder: &mut decoder,
                text: Some(&mut text),
                raster: &mut raster,
            },
            &|| false,
        );
        assert!(pixels.is_empty());
        assert_eq!((decoder.calls, text.calls, raster.calls), (0, 0, 0));
        let error = &serde_json::to_value(response).unwrap()["error"];
        assert_eq!(
            error["error"]["code"],
            if switch_source {
                "SOURCE_CONFLICT"
            } else {
                "INPUT_INVALID"
            }
        );
    }
}
