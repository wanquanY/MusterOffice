#[allow(dead_code)]
#[path = "../../../tools/test-support/source_resource_page.rs"]
mod support;
use mo_image::{DecoderReply, ImageDecoder, ImageError};
use mo_opc::{Package, PackageLimits};
use mo_presentation_compile::{source_page::*, source_resource_page::*};
use mo_raster::{BackendReply, RasterBackend, RasterError};
use std::cell::Cell;
use support::*;
#[derive(Default)]
struct Decoder {
    calls: usize,
    invalidated: usize,
}
impl ImageDecoder for Decoder {
    fn decode(&mut self, bytes: &[u8]) -> Result<DecoderReply, ImageError> {
        self.calls += 1;
        let pixels = if bytes == PNG {
            vec![255, 0, 0, 255, 0, 0, 0, 0, 0, 0, 255, 255, 255, 255, 0, 255]
        } else if bytes == CYAN {
            [0, 255, 255, 255].repeat(4)
        } else {
            panic!("unexpected encoded image")
        };
        Ok(DecoderReply {
            status: 0,
            words: [2, 2, 2, 2, 1, 1, 8, 0, 16],
            pixels,
        })
    }
    fn invalidate(&mut self) {
        self.invalidated += 1;
    }
}
fn prepared(
    b: &[u8],
    d: &mut Decoder,
    check: &dyn Fn() -> bool,
) -> Result<PreparedResourcePage, SourcePageError> {
    let p = Package::open(b, b.len() as u64, PackageLimits::default(), &|| false).unwrap();
    let i = read(b);
    prepare(&p, &i, &request(&i), d, None, options(), check)
}
fn pic(id: u32, image: &str) -> String {
    picture(id, "", &blip(image, "", STRETCH), &solid("00AA00"))
}
#[test]
fn page_deduplicates_encoded_content_keeps_dual_fill_and_counts_real_draws() {
    let b = image_fixture(&(pic(42, "owned-image") + &pic(43, "owned-copy")));
    let mut decoder = Decoder::default();
    let plan = prepared(&b, &mut decoder, &|| false)
        .unwrap()
        .plan(&|| false)
        .unwrap();
    assert_eq!(
        (
            decoder.calls,
            plan.images.decoded.len(),
            plan.images.gather_copy_bytes
        ),
        (1, 1, 0)
    );
    assert_eq!(plan.images.bindings.len(), 2);
    assert_eq!(plan.page.bindings.len(), 3);
    assert_eq!(plan.page.paint_sources.len(), 5); // background, two fills per picture
    assert_eq!(
        (
            plan.image_work.resources,
            plan.image_work.draws,
            plan.image_work.resource_bytes
        ),
        (1, 2, 16)
    );
    assert!(plan.page.bindings[1].picture_fill.is_some());
    assert!(plan.page.paint_sources[1].fill_target.is_none());
    assert!(matches!(
        plan.page.paint_sources[2].fill_target,
        Some(mo_pptx::source::fill::resolve::FillTarget::Picture { native_id: 42 })
    ));
    assert!(plan.page.downstream_coordinate_error_bound >= plan.image_work.coordinate_error_bound);
    assert_eq!(plan.page.raster.scene.clips.len(), 2);
}
#[test]
fn distinct_images_gather_once_and_hidden_images_do_not_decode() {
    let b = image_fixture(&(pic(42, "owned-image") + &pic(43, "owned-cyan")));
    let mut d = Decoder::default();
    let plan = prepared(&b, &mut d, &|| false)
        .unwrap()
        .plan(&|| false)
        .unwrap();
    assert_eq!(
        (
            d.calls,
            plan.images.gather_copy_bytes,
            plan.image_work.resource_bytes
        ),
        (2, 32, 32)
    );
    let b = rewrite(&b, SLIDE, |s| {
        s.replace("id=\"43\"", "hidden=\"1\" id=\"43\"")
    });
    let mut d = Decoder::default();
    let plan = prepared(&b, &mut d, &|| false)
        .unwrap()
        .plan(&|| false)
        .unwrap();
    assert_eq!(d.calls, 1);
    assert_eq!(plan.images.bindings.len(), 1);
    // An unfilled geometry path does not consume an image, including an
    // orientation policy that would fail if this image actually painted.
    let b = image_fixture(&picture(
        42,
        "",
        &blip("owned-image", "rotWithShape=\"0\"", STRETCH),
        "<a:noFill/>",
    ));
    let b = rewrite(&b, SLIDE, |s| {
        s.replace("<a:prstGeom prst=\"rect\"/>","<a:custGeom><a:avLst/><a:gdLst/><a:ahLst/><a:cxnLst/><a:rect l=\"0\" t=\"0\" r=\"w\" b=\"h\"/><a:pathLst><a:path fill=\"none\" stroke=\"0\"><a:moveTo><a:pt x=\"0\" y=\"0\"/></a:moveTo><a:lnTo><a:pt x=\"w\" y=\"h\"/></a:lnTo></a:path></a:pathLst></a:custGeom>")
    });
    let mut d = Decoder::default();
    let p = prepared(&b, &mut d, &|| false)
        .unwrap()
        .plan(&|| false)
        .unwrap();
    assert_eq!(d.calls, 0);
    assert!(p.images.bindings.is_empty());
}
#[test]
fn whole_page_prerequisites_fail_before_decoder_and_plain_page_stays_explicit() {
    let good = pic(42, "owned-image");
    for bad in [
        picture(
            43,
            "",
            &blip("owned-image", "rotWithShape=\"0\"", STRETCH),
            "<a:noFill/>",
        ),
        pic(43, "missing-image"),
        picture(43, "", &blip("owned-external", "", STRETCH), "<a:noFill/>"),
        shape(43, 0, 0, "", &colored("A", "FF0000")), // no font/shaper context
    ] {
        let b = image_fixture(&(good.clone() + &bad));
        let mut d = Decoder::default();
        assert!(prepared(&b, &mut d, &|| false).is_err());
        assert_eq!(d.calls, 0);
    }
    let b = image_fixture(&good);
    let i = read(&b);
    assert!(mo_presentation_compile::source_page::compile(&i, &request(&i), &|| false).is_err());
}
#[test]
fn cancellation_at_every_page_checkpoint_publishes_nothing() {
    let b = image_fixture(&(pic(42, "owned-image") + &pic(43, "owned-cyan")));
    let mut d = Decoder::default();
    let calls = Cell::new(0);
    prepared(&b, &mut d, &|| {
        calls.set(calls.get() + 1);
        false
    })
    .unwrap()
    .plan(&|| false)
    .unwrap();
    // SourceIndex construction is outside this loop; every page preflight,
    // resource/decode/layout/emit checkpoint is interrupted deterministically.
    let p = Package::open(
        b.as_slice(),
        b.len() as u64,
        PackageLimits::default(),
        &|| false,
    )
    .unwrap();
    let i = read(&b);
    let q = request(&i);
    for at in 1..=calls.get() {
        let n = Cell::new(0);
        let mut d = Decoder::default();
        let r = prepare(&p, &i, &q, &mut d, None, options(), &|| {
            n.set(n.get() + 1);
            n.get() == at
        });
        assert!(r.is_err(), "checkpoint {at}");
    }
}
#[derive(Default)]
struct Raster {
    calls: u32,
}
impl RasterBackend for Raster {
    fn raster(&mut self, _: &[u32]) -> Result<BackendReply, RasterError> {
        panic!("resource-free backend")
    }
    fn raster_images(&mut self, frame: &[u32], images: &[u8]) -> Result<BackendReply, RasterError> {
        self.calls += 1;
        assert_eq!(frame[1], 7);
        assert_eq!(images.len(), 16);
        Err(RasterError::Host("owned failure probe"))
    }
    fn invalidate(&mut self) {}
}
#[test]
fn one_image_backend_call_and_atomic_failure_after_shared_compilation() {
    let b = image_fixture(&pic(42, "owned-image"));
    let mut d = Decoder::default();
    let mut r = Raster::default();
    assert!(
        prepared(&b, &mut d, &|| false)
            .unwrap()
            .render(&mut r, &|| false)
            .is_err()
    );
    assert_eq!((d.calls, r.calls), (1, 1));
}

#[test]
fn background_image_window_reuses_one_decoded_background() {
    let shape = drawing_shape(42, "<a:noFill/>").replacen("<p:sp>", "<p:sp useBgFill=\"1\">", 1);
    let b = background_image(&image_fixture(&shape));
    let mut d = Decoder::default();
    let p = prepared(&b, &mut d, &|| false)
        .unwrap()
        .plan(&|| false)
        .unwrap();
    assert_eq!(d.calls, 1);
    assert_eq!(p.images.bindings.len(), 1);
    assert_eq!(p.images.gather_copy_bytes, 0);
    assert_eq!(p.image_work.draws, 1);
    let window = p.page.raster.scene.instances.last().unwrap();
    assert!(matches!(
        window.brush,
        mo_raster::Brush::Snapshot { after_draws: 1 }
    ));
    assert_eq!(window.blend, mo_raster::BlendMode::Source);
}

#[test]
fn translucent_background_window_is_not_an_ordinary_source_over_fill() {
    let shape = drawing_shape(42, "<a:noFill/>").replacen("<p:sp>", "<p:sp useBgFill=\"1\">", 1);
    let b = fixture(&shape);
    let b = rewrite(&b, SLIDE, |s| {
        s.replacen(
            "<a:srgbClr val=\"FFFFFF\"/>",
            "<a:srgbClr val=\"FFFFFF\"><a:alpha val=\"50000\"/></a:srgbClr>",
            1,
        )
    });
    let i = read(&b);
    let p = mo_presentation_compile::source_page::compile(&i, &request(&i), &|| false).unwrap();
    let window = p.raster.scene.instances.last().unwrap();
    assert!(matches!(
        window.brush,
        mo_raster::Brush::Snapshot { after_draws: 1 }
    ));
    assert_eq!(window.blend, mo_raster::BlendMode::Source);
    let mut d = Decoder::default();
    let resource = prepared(&b, &mut d, &|| false)
        .unwrap()
        .plan(&|| false)
        .unwrap();
    assert_eq!(
        serde_json::to_value(resource.page.raster).unwrap(),
        serde_json::to_value(p.raster).unwrap()
    );
    assert_eq!(d.calls, 0);
}
