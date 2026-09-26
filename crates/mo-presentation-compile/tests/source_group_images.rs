#[allow(dead_code)]
#[path = "../../../tools/test-support/source_group_images.rs"]
mod support;
use mo_image::{DecoderReply, ImageDecoder, ImageError};
use mo_opc::{Package, PackageLimits};
use mo_presentation_compile::{source_page::SourcePageError, source_resource_page::*};
use support::*;

#[derive(Default)]
struct Decoder(usize);
impl ImageDecoder for Decoder {
    fn decode(&mut self, b: &[u8]) -> Result<DecoderReply, ImageError> {
        self.0 += 1;
        assert_eq!(b, PNG, "only the nearest inherited resource is used");
        Ok(DecoderReply {
            status: 0,
            words: [2, 2, 2, 2, 1, 1, 8, 0, 16],
            pixels: vec![255, 0, 0, 255, 0, 0, 0, 0, 0, 0, 255, 255, 255, 255, 0, 255],
        })
    }
    fn invalidate(&mut self) {}
}
fn plan(b: &[u8], d: &mut Decoder) -> Result<SourceResourcePagePlan, SourcePageError> {
    let p = Package::open(b, b.len() as u64, PackageLimits::default(), &|| false).unwrap();
    let i = read(b);
    prepare(&p, &i, &request(&i), d, None, options(), &|| false)?.plan(&|| false)
}
#[test]
fn group_fill_keeps_receiver_space_and_declaring_relationship_owner() {
    for c in cases() {
        let mut d = Decoder::default();
        let a = plan(&c.inherited, &mut d).unwrap_or_else(|e| panic!("{}: {e:?}", c.name));
        assert_eq!(d.0, 1, "{}: decode once across all receivers", c.name);
        assert_eq!(a.images.gather_copy_bytes, 0);
        assert_eq!(a.images.bindings.len(), c.expected_image_uses);
        let b = plan(&c.explicit, &mut Decoder::default()).unwrap();
        assert_eq!(
            serde_json::to_value(&a.page.raster).unwrap(),
            serde_json::to_value(&b.page.raster).unwrap(),
            "{}: inherited properties must use each receiver's own geometry",
            c.name
        );
        assert!(
            a.images
                .bindings
                .iter()
                .any(|i| !i.source.redirects.is_empty()),
            "{}",
            c.name
        );
        for image in &a.images.bindings {
            assert_eq!(
                serde_json::to_value(image.layout.placement.as_ref()).unwrap(),
                serde_json::to_value(&a.page.bindings[image.binding as usize].placement).unwrap()
            );
        }
        if c.name == "layout-placeholder" {
            let image = &a.images.bindings[0];
            assert_eq!(
                image.source.redirects[0].target.part,
                "/ppt/slideLayouts/slideLayout2.xml"
            );
            assert_eq!(
                image
                    .layout
                    .placement
                    .as_ref()
                    .unwrap()
                    .source_size
                    .width
                    .get(),
                400000
            );
            assert_eq!(a.page.bindings[1].location.part, SLIDE);
        }
    }
}
#[test]
fn inherited_image_resource_and_orientation_fail_before_any_decode() {
    let t = transform("", [0, 0, 1600000, 800000], [0, 0, 1600000, 800000]);
    let child = receiver(42, [0, 0, 800000, 800000], "", INHERIT);
    for (id, extra) in [
        ("owned-image", "rotWithShape=\"0\""),
        ("missing", ""),
        ("owned-external", ""),
    ] {
        let bad = group(90, &t, &blip(id, extra, STRETCH), &child);
        let good = picture(44, "", &blip("owned-copy", "", STRETCH), "<a:noFill/>");
        let mut d = Decoder::default();
        let result = plan(&image_fixture(&(good + &bad)), &mut d);
        assert!(result.is_err(), "{id} {extra}");
        assert_eq!(d.0, 0, "whole-page prerequisite: {id} {extra}");
    }
}
