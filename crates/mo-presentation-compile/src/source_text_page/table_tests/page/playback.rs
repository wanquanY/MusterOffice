use super::super::playback_support::{timing, visibility_timing};
use super::*;
use crate::{source_playback::SourcePlaybackPlan, source_resource_page::*};
use mo_common::RationalTime;
use mo_opc::Package;
struct Never;
impl mo_image::ImageDecoder for Never {
    fn decode(&mut self, _: &[u8]) -> Result<mo_image::DecoderReply, mo_image::ImageError> {
        panic!("owned table has no images")
    }
    fn invalidate(&mut self) {
        panic!("no decoder calls")
    }
}
fn options() -> ResourcePageOptions {
    ResourcePageOptions {
        selection: mo_presentation_source::source::images::ImageSourceSelection::EmbeddedSnapshot,
        sampling: mo_raster::ImageSampling::Nearest,
        text_limits: Default::default(),
    }
}
pub(in crate::source_text_page::table_tests) fn motion(base: &[u8]) -> Vec<u8> {
    let xml = timing(2, 0, 0, "freeze")
        .replace(
            "<p:animRot from=\"0\" to=\"0\">",
            "<p:animMotion origin=\"layout\" path=\"M 0 0 L 0.25 0.125 E\">",
        )
        .replace(
            "<p:attrName>r</p:attrName>",
            "<p:attrName>ppt_x</p:attrName><p:attrName>ppt_y</p:attrName>",
        )
        .replace("</p:animRot>", "</p:animMotion>");
    rewrite(base, SLIDE, |s| {
        s.replace("</p:sld>", &format!("{xml}</p:sld>"))
    })
}
pub(in crate::source_text_page::table_tests) fn fade(base: &[u8]) -> Vec<u8> {
    let xml = timing(2, 0, 0, "freeze")
        .replace(
            "<p:animRot from=\"0\" to=\"0\">",
            "<p:animEffect filter=\"fade\" transition=\"in\">",
        )
        .replace("</p:animRot>", "</p:animEffect>")
        .replace(
            "<p:attrNameLst><p:attrName>r</p:attrName></p:attrNameLst>",
            "",
        );
    rewrite(base, SLIDE, |s| {
        s.replace("</p:sld>", &format!("{xml}</p:sld>"))
    })
}
#[test]
fn native_table_motion_and_visibility_reuse_cell_text_without_reordering_paint() {
    let base = uniform_borders(&fixture(false));
    for (name, bytes) in [
        ("motion", motion(&base)),
        ("fade", fade(&base)),
        ("circle", motion(&gradient_cell(&base, "circle"))),
        (
            "reveal",
            rewrite(&base, SLIDE, |s| {
                s.replace("<p:cNvPr id=\"2\"", "<p:cNvPr hidden=\"1\" id=\"2\"")
                    .replace(
                        "</p:sld>",
                        &format!("{}</p:sld>", visibility_timing(2, "visible", "freeze")),
                    )
            }),
        ),
        (
            "hidden",
            rewrite(&base, SLIDE, |s| {
                s.replace(
                    "</p:sld>",
                    &format!("{}</p:sld>", visibility_timing(2, "hidden", "freeze")),
                )
            }),
        ),
    ] {
        let index = read(&bytes);
        let q = page_request(&index);
        let package = Package::open(
            bytes.as_slice(),
            bytes.len() as u64,
            Default::default(),
            &|| false,
        )
        .unwrap();
        let make = || {
            SourcePlaybackPlan::new(&package,&index,q.clone(),serde_json::from_value(serde_json::json!({"session":"table-test","generation":"1","revision":index.source_sha256})).unwrap(),Default::default(),Default::default(),&||false).unwrap()
        };
        let fonts = fonts();
        let manifest =
            PreparedManifest::load(&fonts, FONT_BYTES, Default::default(), &|| false).unwrap();
        let mut retained = make()
            .retain(
                &package,
                index.clone(),
                &mut Never,
                Some(TextPageContext {
                    manifest: &manifest,
                    backend: &mut NativeShaper::default(),
                }),
                options(),
                &|| false,
            )
            .unwrap();
        let mut direct = make();
        save(&format!("playback-{name}"), "pptx", &bytes);
        save(
            &format!("playback-{name}"),
            "request.json",
            &serde_json::to_vec(&q).unwrap(),
        );
        let mut sampled = Vec::new();
        for (ordinal, ms) in [0, 250, 500, 750, 1000, 250].into_iter().enumerate() {
            let at = RationalTime::new(ms, 1000).unwrap();
            let (state, result) = retained
                .render(at, None, &mut NativeRaster, &|| false)
                .unwrap();
            let sample = direct.sample(at, None, &|| false).unwrap();
            let expected = sample
                .prepare(
                    &package,
                    &index,
                    &mut Never,
                    Some(TextPageContext {
                        manifest: &manifest,
                        backend: &mut NativeShaper::default(),
                    }),
                    options(),
                    &|| false,
                )
                .unwrap()
                .render(&mut NativeRaster, &|| false)
                .unwrap();
            assert_eq!(result.pixels, expected.pixels);
            assert_eq!(
                serde_json::to_value(&result.info.text_capacity).unwrap(),
                serde_json::to_value(&expected.info.text_capacity).unwrap()
            );
            assert_eq!(
                serde_json::to_value(&state).unwrap(),
                serde_json::to_value(sample.frame()).unwrap()
            );
            assert_eq!(result.info.text_work.component_calls, 0);
            let frames = if (name == "hidden" && ms >= 500) || (name == "reveal" && ms < 500) {
                0
            } else {
                6
            };
            assert_eq!(result.info.text_frames, frames);
            if frames == 0 || name == "fade" && ms == 0 {
                assert!(result.pixels.chunks_exact(4).all(|p| p == [255; 4]));
            }
            save(
                &format!("playback-{name}-{ordinal}"),
                "state.json",
                &serde_json::to_vec(&state).unwrap(),
            );
            save(
                &format!("playback-{name}-{ordinal}"),
                "rgba",
                &result.pixels,
            );
            sampled.push(result.pixels);
        }
        assert_ne!(sampled[0], sampled[2]);
        assert_eq!(sampled[1], sampled[5]);
        if name == "fade" {
            // Composite the already-painted table once. Applying opacity per
            // cell/border/text primitive would darken their overlapping pixels.
            for (middle, full) in sampled[2].iter().zip(&sampled[4]) {
                let expected = (u16::from(*full) + 255) / 2;
                assert!((i32::from(*middle) - i32::from(expected)).abs() <= 1);
            }
        }
    }
}
