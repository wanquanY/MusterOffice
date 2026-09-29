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
        panic!("owned resource")
    }
}
#[test]
fn imported_group_motion_matches_static_layout_and_reuses_decoded_resources() {
    let child = picture(42, "rot=\"2700000\"", &blip("owned-image", "", STRETCH), "");
    let content = |x, y| {
        group(
            8,
            &transform(
                "rot=\"5400000\" flipH=\"1\"",
                [x, y, 1600000, 800000],
                [-100000, 0, 1000000, 800000],
            ),
            "<a:noFill/>",
            &child,
        )
    };
    let base = image_fixture(&content(300000, 300000));
    let motion = timing(8, 0, 0, "freeze")
        .replace(
            "<p:animRot from=\"0\" to=\"0\">",
            "<p:animMotion origin=\"layout\" path=\"M 0 0 L 0.25 -0.125 E\">",
        )
        .replace(
            "<p:attrName>r</p:attrName>",
            "<p:attrName>ppt_x</p:attrName><p:attrName>ppt_y</p:attrName>",
        )
        .replace("</p:animRot>", "</p:animMotion>");
    let bytes = rewrite(&base, SLIDE, |s| {
        s.replace("</p:sld>", &format!("{motion}</p:sld>"))
    });
    let package = Package::open(
        bytes.as_slice(),
        bytes.len() as u64,
        PackageLimits::default(),
        &|| false,
    )
    .unwrap();
    let index = read(&bytes);
    let size = index.page_size.unwrap();
    let binding = PlaybackBinding {
        session: PlaybackSessionId::new("motion-source").unwrap(),
        revision: index.source_sha256.clone(),
        generation: PlaybackGeneration::new(1),
    };
    let mut plan = SourcePlaybackPlan::new(
        &package,
        &index,
        request(&index),
        binding,
        SourceLimits::default(),
        TimelineLimits::default(),
        &|| false,
    )
    .unwrap();
    let sample = plan
        .sample(RationalTime::new(1, 2).unwrap(), None, &|| false)
        .unwrap();
    let actual = sample
        .prepare(
            &package,
            &index,
            &mut Decoder::default(),
            None,
            options(),
            &|| false,
        )
        .unwrap()
        .plan(&|| false)
        .unwrap();
    let control = image_fixture(&content(
        300000 + size.width.get() / 8,
        300000 - size.height.get() / 16,
    ));
    let p = Package::open(
        control.as_slice(),
        control.len() as u64,
        PackageLimits::default(),
        &|| false,
    )
    .unwrap();
    let ix = read(&control);
    let expected = mo_presentation_compile::source_resource_page::prepare(
        &p,
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
    let decodes = decoder.0;
    assert_eq!(decodes, 1);
    for ms in [0, 333, 999, 1000, 500, 0] {
        let (frame, _) = retained
            .prepare_frame(RationalTime::new(ms, 1000).unwrap(), None, &|| false)
            .unwrap();
        assert_eq!(frame.profile(), MOTION_PROFILE);
    }
    assert_eq!(decoder.0, decodes);
}
