#[allow(dead_code)]
#[path = "../../../tools/test-support/source_playback.rs"]
mod support;
use mo_common::*;
use mo_opc::{Package, PackageLimits};
use mo_pptx::source::SourceLimits;
use mo_presentation_compile::source_playback::*;
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
            pixels: vec![255, 0, 0, 255, 0, 0, 0, 0, 0, 0, 255, 255, 255, 255, 0, 255],
        })
    }
    fn invalidate(&mut self) {
        panic!("valid owned resource")
    }
}
fn t(n: i64, d: u32) -> RationalTime {
    RationalTime::new(n, d).unwrap()
}
#[test]
fn native_scale_keeps_images_retained_and_reaches_zero_without_poisoning_other_samples() {
    let fill = blip("owned-image", "", STRETCH);
    let gradient = "<a:gradFill><a:gsLst><a:gs pos=\"0\"><a:srgbClr val=\"FF0000\"/></a:gs><a:gs pos=\"100000\"><a:srgbClr val=\"0000FF\"/></a:gs></a:gsLst><a:lin ang=\"2700000\" scaled=\"1\"/></a:gradFill>";
    for content in [
        drawing_shape(42, &solid("2255EE")),
        picture(42, "rot=\"2700000\"", &fill, ""),
        drawing_shape(42, &fill),
        drawing_shape(42, gradient),
    ] {
        let base = image_fixture(&content);
        let bytes = scaled(&base, 42, [0, 0], [200000, 100000], "freeze");
        let package = Package::open(
            bytes.as_slice(),
            bytes.len() as u64,
            PackageLimits::default(),
            &|| false,
        )
        .unwrap();
        let index = read(&bytes);
        let binding = PlaybackBinding {
            session: PlaybackSessionId::new("scale-resource").unwrap(),
            generation: PlaybackGeneration::new(1),
            revision: index.source_sha256.clone(),
        };
        let plan = SourcePlaybackPlan::new(
            &package,
            &index,
            request(&index),
            binding,
            SourceLimits::default(),
            TimelineLimits::default(),
            &|| false,
        )
        .unwrap();
        let mut decoder = Decoder::default();
        let mut retained = plan
            .retain(
                &package,
                index.clone(),
                &mut decoder,
                None,
                options(),
                &|| false,
            )
            .unwrap();
        let count = decoder.0;
        let resources = serde_json::to_value(retained.preparation()).unwrap();
        let before = retained.prepare_frame(t(1, 3), None, &|| false).unwrap().0;
        for at in [t(0, 1), t(1, 7), t(1, 1), t(0, 1), t(1, 3)] {
            let (frame, _) = retained
                .prepare_frame(at, None, &|| false)
                .unwrap_or_else(|e| panic!("time={at:?} content={content}: {e:?}"));
            assert_eq!(frame.profile(), TRANSFORM_PROFILE);
            assert_eq!(frame.source_sha256, index.source_sha256);
        }
        assert_eq!(
            retained
                .prepare_frame(t(1, 3), None, &|| false)
                .unwrap()
                .0
                .evaluated,
            before.evaluated
        );
        assert_eq!(decoder.0, count);
        assert_eq!(
            serde_json::to_value(retained.preparation()).unwrap(),
            resources
        );
    }
}
#[test]
fn nested_nonuniform_group_scale_keeps_child_centers_and_rotations_bound_to_the_source() {
    let child = support::receiver(
        42,
        [100000, 200000, 300000, 400000],
        "rot=\"5400000\"",
        &solid("EECC00"),
    );
    let group = |size| {
        support::group(
            8,
            &support::transform(
                "rot=\"2700000\" flipH=\"1\"",
                size,
                [-100000, 0, 1000000, 800000],
            ),
            "<a:noFill/>",
            &child,
        )
    };
    let base = image_fixture(&group([300000, 300000, 1600000, 800000]));
    let animated = scaled(&base, 8, [100000, 100000], [200000, 50000], "freeze");
    let p = Package::open(
        animated.as_slice(),
        animated.len() as u64,
        PackageLimits::default(),
        &|| false,
    )
    .unwrap();
    let index = read(&animated);
    let binding = PlaybackBinding {
        session: PlaybackSessionId::new("scale-group").unwrap(),
        revision: index.source_sha256.clone(),
        generation: PlaybackGeneration::new(1),
    };
    let mut plan = SourcePlaybackPlan::new(
        &p,
        &index,
        request(&index),
        binding,
        SourceLimits::default(),
        TimelineLimits::default(),
        &|| false,
    )
    .unwrap();
    let sample = plan.sample(t(1, 2), None, &|| false).unwrap();
    let actual = sample
        .prepare(
            &p,
            &index,
            &mut Decoder::default(),
            None,
            options(),
            &|| false,
        )
        .unwrap()
        .plan(&|| false)
        .unwrap();
    let control = image_fixture(&group([-100000, 400000, 2400000, 600000]));
    let p = Package::open(
        control.as_slice(),
        control.len() as u64,
        PackageLimits::default(),
        &|| false,
    )
    .unwrap();
    let index = read(&control);
    let expected = mo_presentation_compile::source_resource_page::prepare(
        &p,
        &index,
        &request(&index),
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
