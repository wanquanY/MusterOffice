//! No author render is allowed to depend on serializing a PPTX. Roundtrip is
//! used only as an independent equivalence oracle after direct compilation.
#[allow(dead_code)]
#[path = "../../../tools/test-support/source_resource_page.rs"]
mod support;
use mo_common::{Digest, ResourceId};
use mo_image::{DecoderReply, ImageDecoder, ImageError};
use mo_presentation_compile::{source_page::*, source_resource_page::*};
use mo_presentation_model::*;
use mo_presentation_source::{
    PptxError,
    author::*,
    source::{SourceLimits, images::*, inspect_source},
};
use sha2::{Digest as _, Sha256};

struct Bytes(Vec<u8>);
impl Resources for Bytes {
    fn open(&self, _: &ResourceId) -> Result<ResourceData<'_>, PptxError> {
        Ok(ResourceData {
            reader: &self.0,
            byte_length: self.0.len() as u64,
        })
    }
}
#[derive(Default)]
struct Decoder(usize);
impl ImageDecoder for Decoder {
    fn decode(&mut self, bytes: &[u8]) -> Result<DecoderReply, ImageError> {
        self.0 += 1;
        assert_eq!(bytes, support::PNG);
        Ok(DecoderReply {
            status: 0,
            words: [2, 2, 2, 2, 1, 1, 8, 0, 16],
            pixels: [11, 22, 33, 255].repeat(4),
        })
    }
    fn invalidate(&mut self) {}
}
fn input() -> (Document, ExportDefaults, Bytes) {
    let json: serde_json::Value = serde_json::from_str(include_str!(
        "../../../fixtures/presentations/native-export/request.json"
    ))
    .unwrap();
    let mut document: Document = serde_json::from_value(json["document"].clone()).unwrap();
    // Font-independent geometry/image test. The native worker tests cover the
    // same direct path with real shaping and explicit font bytes as well.
    for object in document.objects.values_mut() {
        if let ObjectContent::Shape { text, .. } = &mut object.content {
            *text = None;
        }
    }
    for resource in document
        .resources
        .values_mut()
        .filter(|r| r.kind == ResourceKind::Picture)
    {
        resource.sha256 = Digest::from_sha256(Sha256::digest(support::PNG).into());
    }
    (
        document,
        serde_json::from_value(json["defaults"].clone()).unwrap(),
        Bytes(support::PNG.to_vec()),
    )
}

#[test]
fn author_images_and_page_commands_equal_verified_source_without_format_bridge() {
    let (document, defaults, resources) = input();
    let plan = AuthorPlan::new(&document, &defaults, Default::default(), &|| false).unwrap();
    let images = AuthorImages::new(&plan, &resources, &|| false).unwrap();
    let index = plan.declarations();
    let mut decoder = Decoder::default();
    let mut request = support::request(index);
    request.viewport.width = 160;
    request.viewport.height = 90;
    request.viewport.scale = mo_raster::PixelScale {
        numerator: 1,
        denominator: 76200,
    };
    request.slide = index.slides[0].part.clone();
    let direct = prepare_input(
        &images,
        index,
        &request,
        &mut decoder,
        None,
        support::options(),
        &|| false,
    )
    .unwrap()
    .plan(&|| false)
    .unwrap();
    assert_eq!(decoder.0, 1);
    // Independent oracle runs only after the direct plan has completed.
    let output =
        mo_pptx::export_plan_to(&plan, &resources, Vec::new(), Default::default(), &|| false)
            .unwrap();
    let readback = inspect_source(output.package(), SourceLimits::default(), &|| false).unwrap();
    request.expected_source_sha256 = readback.source_sha256.clone();
    let imported = prepare(
        output.package(),
        &readback,
        &request,
        &mut decoder,
        None,
        support::options(),
        &|| false,
    )
    .unwrap()
    .plan(&|| false)
    .unwrap();
    assert_eq!(direct.resources_sha256, imported.resources_sha256);
    assert_eq!(
        serde_json::to_value(direct.page.raster).unwrap(),
        serde_json::to_value(imported.page.raster).unwrap()
    );
    assert_eq!(
        serde_json::to_value(direct.image_work).unwrap(),
        serde_json::to_value(imported.image_work).unwrap()
    );
}

#[test]
fn author_resource_binding_and_cancellation_fail_before_decode() {
    let (document, defaults, mut bytes) = input();
    let plan = AuthorPlan::new(&document, &defaults, Default::default(), &|| false).unwrap();
    bytes.0[16] ^= 1;
    let images = AuthorImages::new(&plan, &bytes, &|| false).unwrap();
    let mut request = support::request(plan.declarations());
    request.viewport.width = 160;
    request.viewport.height = 90;
    request.viewport.scale = mo_raster::PixelScale {
        numerator: 1,
        denominator: 76200,
    };
    request.slide = plan.declarations().slides[0].part.clone();
    let mut decoder = Decoder::default();
    assert!(
        prepare_input(
            &images,
            plan.declarations(),
            &request,
            &mut decoder,
            None,
            support::options(),
            &|| false
        )
        .is_err()
    );
    assert!(
        prepare_input(
            &images,
            plan.declarations(),
            &request,
            &mut decoder,
            None,
            support::options(),
            &|| true
        )
        .is_err()
    );
    request.expected_source_sha256 = Digest::from_sha256([0; 32]);
    assert!(matches!(
        prepare_input(
            &images,
            plan.declarations(),
            &request,
            &mut decoder,
            None,
            support::options(),
            &|| false
        ),
        Err(SourcePageError::SourceConflict)
    ));
    assert_eq!(decoder.0, 0);
}
