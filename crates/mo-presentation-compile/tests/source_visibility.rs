#[allow(dead_code)]
#[path = "../../../tools/test-support/source_visibility.rs"]
mod support;
use mo_common::*;
use mo_opc::{Package, PackageLimits};
use mo_pptx::source::SourceLimits;
use mo_presentation_compile::{source_playback::*, source_resource_page};
use mo_timeline::*;
use support::*;
#[derive(Default)]
struct Decoder(usize);
impl mo_image::ImageDecoder for Decoder {
    fn decode(&mut self, b: &[u8]) -> Result<mo_image::DecoderReply, mo_image::ImageError> {
        self.0 += 1;
        assert_eq!(b, PNG);
        Ok(mo_image::DecoderReply {
            status: 0,
            words: [2, 2, 2, 2, 1, 1, 8, 0, 16],
            pixels: vec![255; 16],
        })
    }
    fn invalidate(&mut self) {
        panic!("owned decoder invalidation")
    }
}
fn package(bytes: &[u8]) -> Package<&[u8]> {
    Package::open(bytes, bytes.len() as u64, PackageLimits::default(), &|| {
        false
    })
    .unwrap()
}
fn binding(bytes: &[u8]) -> PlaybackBinding {
    PlaybackBinding {
        session: PlaybackSessionId::new("source-visibility").unwrap(),
        revision: read(bytes).source_sha256,
        generation: PlaybackGeneration::new(1),
    }
}
fn plan(bytes: &[u8]) -> SourcePlaybackPlan {
    let index = read(bytes);
    SourcePlaybackPlan::new(
        &package(bytes),
        &index,
        request(&index),
        binding(bytes),
        SourceLimits::default(),
        TimelineLimits::default(),
        &|| false,
    )
    .unwrap()
}
fn time(ms: i64) -> RationalTime {
    RationalTime::new(ms, 1000).unwrap()
}
#[test]
fn hidden_property_matches_static_source_composition_and_group_inheritance() {
    let child = receiver(42, [0, 0, 800000, 800000], "", &solid("FF0000"));
    let group = group(
        90,
        &transform("", [0, 0, 800000, 800000], [0, 0, 800000, 800000]),
        "<a:noFill/>",
        &child,
    );
    for (target, body) in [(42, child), (90, group)] {
        let base = image_fixture(
            &(body + &receiver(43, [800000, 0, 800000, 800000], "", &solid("00FF00"))),
        );
        let animated = visibility(&base, target, "hidden", "remove");
        let index = read(&animated);
        let before = serde_json::to_value(&index).unwrap();
        let mut p = plan(&animated);
        for (ms, hide) in [
            (0, false),
            (500, true),
            (1499, true),
            (1500, false),
            (750, true),
            (0, false),
        ] {
            let actual = p
                .sample(time(ms), None, &|| false)
                .unwrap()
                .prepare(
                    &package(&animated),
                    &index,
                    &mut Decoder::default(),
                    None,
                    options(),
                    &|| false,
                )
                .unwrap()
                .plan(&|| false)
                .unwrap();
            let control = if hide {
                hidden(&base, target)
            } else {
                base.clone()
            };
            let ix = read(&control);
            let expected = source_resource_page::prepare(
                &package(&control),
                &ix,
                &request(&ix),
                &mut Decoder::default(),
                None,
                options(),
                &|| false,
            )
            .unwrap()
            .plan(&|| false)
            .unwrap();
            assert_eq!(
                serde_json::to_value(actual.page.raster).unwrap(),
                serde_json::to_value(expected.page.raster).unwrap()
            );
        }
        assert_eq!(serde_json::to_value(&index).unwrap(), before);
    }
}
#[test]
fn retained_images_use_stable_source_identity_when_earlier_object_disappears() {
    let image = blip("owned-image", "", STRETCH);
    let base = image_fixture(
        &(receiver(42, [0, 0, 800000, 800000], "", &image)
            + &receiver(43, [800000, 0, 800000, 800000], "", &image)),
    );
    let animated = visibility(&base, 42, "hidden", "remove");
    let mut decoder = Decoder::default();
    let mut owner = plan(&animated)
        .retain(
            &package(&animated),
            read(&animated),
            &mut decoder,
            None,
            options(),
            &|| false,
        )
        .unwrap();
    let prepared = serde_json::to_value(owner.preparation()).unwrap();
    let count = decoder.0;
    assert_eq!(count, 1);
    let before = owner
        .prepare_frame(time(0), None, &|| false)
        .unwrap()
        .1
        .raster()
        .frame()
        .to_vec();
    let hidden = owner
        .prepare_frame(time(500), None, &|| false)
        .unwrap()
        .1
        .raster()
        .frame()
        .to_vec();
    assert_ne!(hidden, before);
    for ms in [1500, 3000, 0] {
        assert_eq!(
            owner
                .prepare_frame(time(ms), None, &|| false)
                .unwrap()
                .1
                .raster()
                .frame(),
            before
        );
    }
    assert_eq!(
        owner
            .prepare_frame(time(750), None, &|| false)
            .unwrap()
            .1
            .raster()
            .frame(),
        hidden
    );
    assert_eq!(decoder.0, count);
    assert_eq!(serde_json::to_value(owner.preparation()).unwrap(), prepared);
}
#[test]
fn retained_plan_prepares_revealable_hidden_resources_once_without_changing_initial_pose() {
    let base = hidden(
        &image_fixture(&picture(42, "", &blip("owned-image", "", STRETCH), "")),
        42,
    );
    let animated = visibility(&base, 42, "visible", "remove");
    let mut decoder = Decoder::default();
    let mut owner = plan(&animated)
        .retain(
            &package(&animated),
            read(&animated),
            &mut decoder,
            None,
            options(),
            &|| false,
        )
        .unwrap();
    assert_eq!(decoder.0, 1);
    let before = owner
        .prepare_frame(time(0), None, &|| false)
        .unwrap()
        .1
        .raster()
        .frame()
        .to_vec();
    let shown = owner
        .prepare_frame(time(500), None, &|| false)
        .unwrap()
        .1
        .raster()
        .frame()
        .to_vec();
    assert_ne!(before, shown);
    assert_eq!(
        owner
            .prepare_frame(time(1500), None, &|| false)
            .unwrap()
            .1
            .raster()
            .frame(),
        before
    );
    assert_eq!(
        owner
            .prepare_frame(time(1000), None, &|| false)
            .unwrap()
            .1
            .raster()
            .frame(),
        shown
    );
    assert_eq!(decoder.0, 1);
}
#[test]
fn revealable_invalid_resources_fail_preparation_but_permanently_hidden_content_is_not_loaded() {
    let content = picture(42, "", &blip("missing-image", "", STRETCH), "");
    let base = hidden(&image_fixture(&content), 42);
    let mut decoder = Decoder::default();
    let hidden_only = visibility(&base, 42, "hidden", "hold");
    assert!(
        plan(&hidden_only)
            .retain(
                &package(&hidden_only),
                read(&hidden_only),
                &mut decoder,
                None,
                options(),
                &|| false
            )
            .is_ok()
    );
    let reveal = visibility(&base, 42, "visible", "remove");
    assert!(
        plan(&reveal)
            .retain(
                &package(&reveal),
                read(&reveal),
                &mut decoder,
                None,
                options(),
                &|| false
            )
            .is_err()
    );
    assert_eq!(decoder.0, 0);
}
