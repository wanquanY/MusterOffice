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
            pixels: vec![255; 16],
        })
    }
    fn invalidate(&mut self) {
        panic!("owned decoder invalidation")
    }
}
fn t(ms: i64) -> RationalTime {
    RationalTime::new(ms, 1000).unwrap()
}
fn package(bytes: &[u8]) -> Package<&[u8]> {
    Package::open(bytes, bytes.len() as u64, PackageLimits::default(), &|| {
        false
    })
    .unwrap()
}
#[test]
fn source_fade_isolates_group_picture_and_shape_and_reuses_decoded_resources() {
    let content = picture(42, "", &blip("owned-image", "", STRETCH), "")
        + &receiver(43, [0, 0, 800000, 800000], "", &solid("FF0000"));
    let base = image_fixture(&group(
        90,
        &transform("", [0, 0, 800000, 800000], [0, 0, 800000, 800000]),
        "<a:noFill/>",
        &content,
    ));
    let timing = timing(90, 0, 0, "remove")
        .replace(
            "<p:animRot from=\"0\" to=\"0\">",
            "<p:animEffect filter=\"fade\" transition=\"in\">",
        )
        .replace("</p:animRot>", "</p:animEffect>")
        .replace(
            "<p:attrNameLst><p:attrName>r</p:attrName></p:attrNameLst>",
            "",
        );
    let bytes = rewrite(&base, SLIDE, |s| {
        s.replace("</p:sld>", &format!("{timing}</p:sld>"))
    });
    let index = read(&bytes);
    let before = serde_json::to_value(&index).unwrap();
    let binding = PlaybackBinding {
        session: PlaybackSessionId::new("source-fade").unwrap(),
        revision: index.source_sha256.clone(),
        generation: PlaybackGeneration::new(1),
    };
    let mut plan = SourcePlaybackPlan::new(
        &package(&bytes),
        &index,
        request(&index),
        binding,
        SourceLimits::default(),
        TimelineLimits::default(),
        &|| false,
    )
    .unwrap();
    let mut decoder = Decoder::default();
    let frame = plan
        .sample(t(500), None, &|| false)
        .unwrap()
        .prepare(
            &package(&bytes),
            &index,
            &mut decoder,
            None,
            options(),
            &|| false,
        )
        .unwrap()
        .plan(&|| false)
        .unwrap();
    let groups = &frame.page.raster.scene.opacity_groups;
    assert_eq!(groups.len(), 1);
    assert_eq!(groups[0].opacity, 32768);
    let range = groups[0].first_draw as usize..groups[0].end_draw as usize;
    let objects: std::collections::BTreeSet<_> = frame.page.paint_sources[range]
        .iter()
        .map(|p| {
            frame.page.bindings[p.binding as usize]
                .location
                .object
                .unwrap()
        })
        .collect();
    assert_eq!(objects, std::collections::BTreeSet::from([42, 43]));
    let mut retained = plan
        .retain(
            &package(&bytes),
            index.clone(),
            &mut decoder,
            None,
            options(),
            &|| false,
        )
        .unwrap();
    let count = decoder.0;
    let prepared = serde_json::to_value(retained.preparation()).unwrap();
    for ms in [250, 500, 1000, 0, 750] {
        let (_, f) = retained.prepare_frame(t(ms), None, &|| false).unwrap();
        assert_eq!(f.raster().frame()[1] == 13, ms < 1000);
    }
    assert_eq!(decoder.0, count);
    assert_eq!(
        serde_json::to_value(retained.preparation()).unwrap(),
        prepared
    );
    assert_eq!(serde_json::to_value(index).unwrap(), before);
}

#[test]
fn faded_background_window_captures_output_before_first_or_later_object() {
    for earlier_object in [false, true] {
        let window =
            drawing_shape(42, "<a:noFill/>").replacen("<p:sp>", "<p:sp useBgFill=\"1\">", 1);
        let content = if earlier_object {
            drawing_shape(43, &solid("FF0000")) + &window
        } else {
            window
        };
        let bytes = background_image(&image_fixture(&content));
        let bytes = animated(&bytes, 42, 0, 0, "remove");
        let bytes = rewrite(&bytes, SLIDE, |s| {
            s.replace(
                "<p:animRot from=\"0\" to=\"0\">",
                "<p:animEffect filter=\"fade\" transition=\"in\">",
            )
            .replace("</p:animRot>", "</p:animEffect>")
            .replace(
                "<p:attrNameLst><p:attrName>r</p:attrName></p:attrNameLst>",
                "",
            )
        });
        let index = read(&bytes);
        let mut plan = SourcePlaybackPlan::new(
            &package(&bytes),
            &index,
            request(&index),
            PlaybackBinding {
                session: PlaybackSessionId::new("background-fade").unwrap(),
                revision: index.source_sha256.clone(),
                generation: PlaybackGeneration::new(1),
            },
            SourceLimits::default(),
            TimelineLimits::default(),
            &|| false,
        )
        .unwrap();
        let mut decoder = Decoder::default();
        let frame = plan
            .sample(t(500), None, &|| false)
            .unwrap()
            .prepare(
                &package(&bytes),
                &index,
                &mut decoder,
                None,
                options(),
                &|| false,
            )
            .unwrap()
            .plan(&|| false)
            .unwrap();
        assert_eq!(decoder.0, 1);
        let scene = &frame.page.raster.scene;
        assert!(scene.instances.iter().any(|paint| matches!(
            paint.brush,
            mo_raster::Brush::Snapshot {
                after_draws: 1,
                scope: mo_raster::SnapshotScope::Output
            }
        )));
        assert_eq!(scene.opacity_groups.len(), 1);
        assert_eq!(scene.opacity_groups[0].opacity, 32768);
        // Retained preparation compiles the complete raster, which used to
        // reject the later object or capture an empty first-object surface.
        let mut retained = plan
            .retain(
                &package(&bytes),
                index.clone(),
                &mut decoder,
                None,
                options(),
                &|| false,
            )
            .unwrap();
        let count = decoder.0;
        for ms in [0, 500, 999, 1000, 250] {
            let (_, frame) = retained.prepare_frame(t(ms), None, &|| false).unwrap();
            assert_eq!(frame.raster().frame()[1] == 14, ms < 1000);
        }
        assert_eq!(decoder.0, count);
    }
}
