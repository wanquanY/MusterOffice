//! Native table image receivers through the real decoder, scene and playback.
use super::*;
use super::{
    cases::edit_cell,
    page::{page_request, uniform_borders},
};
use crate::{source_playback::SourcePlaybackPlan, source_resource_page::*};
use mo_common::RationalTime;
use mo_image::{DecoderReply, ImageDecoder, ImageError};
use mo_opc::Package;
#[allow(dead_code)]
#[path = "../../../../../tools/test-support/source_image_resources.rs"]
mod resources;

#[derive(Default)]
struct Decoder {
    calls: usize,
}
impl ImageDecoder for Decoder {
    fn decode(&mut self, bytes: &[u8]) -> Result<DecoderReply, ImageError> {
        self.calls += 1;
        NativeRaster.decode(bytes)
    }
    fn invalidate(&mut self) {
        ImageDecoder::invalidate(&mut NativeRaster);
    }
}
fn options() -> ResourcePageOptions {
    ResourcePageOptions {
        selection: mo_presentation_source::source::images::ImageSourceSelection::EmbeddedSnapshot,
        sampling: mo_raster::ImageSampling::Nearest,
        text_limits: Default::default(),
    }
}
fn blip(id: &str, mode: &str) -> String {
    format!(
        "<a:blipFill dpi=\"72\" rotWithShape=\"1\"><a:blip r:embed=\"{id}\"/>{mode}</a:blipFill>"
    )
}
const STRETCH: &str = "<a:stretch><a:fillRect/></a:stretch>";
fn image_cells(rtl: bool) -> Vec<u8> {
    let mut bytes = uniform_borders(&fixture(false));
    for cell in 0..9 {
        let fill = match cell {
            0 => blip("owned-image", STRETCH),
            2 => blip(
                "owned-copy",
                "<a:srcRect r=\"50000\"/><a:stretch><a:fillRect l=\"25000\" r=\"25000\"/></a:stretch>",
            ),
            6 => blip(
                "owned-image",
                "<a:tile tx=\"0\" ty=\"0\" sx=\"400000\" sy=\"400000\" flip=\"xy\" algn=\"tl\"/>",
            ),
            _ => "<a:noFill/>".into(),
        };
        bytes = edit_cell(&bytes, cell, |s| {
            let start = s.rfind("<a:solidFill>").unwrap();
            let end = start + s[start..].find("</a:solidFill>").unwrap() + 14;
            format!("{}{}{}", &s[..start], fill, &s[end..])
        });
    }
    let background = blip("owned-cyan", STRETCH);
    bytes = rewrite(&bytes, SLIDE, |s| {
        s.replace(
            "<a:tblPr>",
            &format!("<a:tblPr rtl=\"{}\">{background}", u8::from(rtl)),
        )
    });
    resources::with_image_resources(&bytes)
}
fn save(name: &str, suffix: &str, bytes: &[u8]) {
    let Some(dir) = std::env::var_os("MO_TABLE_IMAGE_EVIDENCE_DIR") else {
        return;
    };
    let dir = std::path::PathBuf::from(dir);
    assert!(dir.is_absolute());
    std::fs::create_dir_all(&dir).unwrap();
    use std::io::Write;
    std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(dir.join(format!("{name}.{suffix}")))
        .unwrap()
        .write_all(bytes)
        .unwrap();
}
#[test]
fn merged_rtl_crop_tile_and_background_images_keep_native_boxes_and_decode_once() {
    for rtl in [false, true] {
        let bytes = image_cells(rtl);
        let index = read(&bytes);
        let q = page_request(&index);
        let package = Package::open(
            bytes.as_slice(),
            bytes.len() as u64,
            Default::default(),
            &|| false,
        )
        .unwrap();
        let fonts = fonts();
        let manifest =
            PreparedManifest::load(&fonts, FONT_BYTES, Default::default(), &|| false).unwrap();
        let mut decoder = Decoder::default();
        let prepared = crate::source_resource_page::prepare(
            &package,
            &index,
            &q,
            &mut decoder,
            Some(TextPageContext {
                manifest: &manifest,
                backend: &mut NativeShaper::default(),
            }),
            options(),
            &|| false,
        )
        .unwrap();
        let plan = prepared.plan(&|| false).unwrap();
        assert_eq!(decoder.calls, 2);
        assert_eq!(plan.images.decoded.len(), 2);
        assert_eq!(plan.images.bindings.len(), 4);
        assert_eq!(plan.images.gather_copy_bytes, 32);
        let table = plan
            .images
            .bindings
            .iter()
            .find(|b| matches!(b.layout.target, FillTarget::TableBackground { .. }))
            .unwrap();
        assert_eq!(
            table.layout.layout.fill_rectangle.right,
            Fixed::emu(mo_common::Emu::new(5486400))
        );
        let merged = plan
            .images
            .bindings
            .iter()
            .find(|b| {
                b.layout.target
                    == FillTarget::TableCell {
                        native_id: 2,
                        cell: cell(0, 0),
                    }
            })
            .unwrap();
        assert_eq!(
            merged.layout.layout.fill_rectangle.left,
            Fixed::emu(mo_common::Emu::new(if rtl { 1828800 } else { 0 }))
        );
        assert_eq!(
            merged.layout.placement.as_ref().unwrap().source_size,
            table.layout.placement.as_ref().unwrap().source_size
        );
        // Standalone public image layout must produce the same bound receiver,
        // not a whole-table substitute for every cell.
        let image_query = mo_presentation_source::source::images::SourceImageQuery {
            fill: SourceFillQuery {
                expected_source_sha256: index.source_sha256.clone(),
                surface: SLIDE.into(),
                targets: plan
                    .images
                    .bindings
                    .iter()
                    .map(|b| b.layout.target.clone())
                    .collect(),
                profile: FillProfile::Drawingml2024DraftV1,
            },
            selection: options().selection,
        };
        let catalog = mo_presentation_source::source::images::query(
            &package,
            &index,
            &image_query,
            Default::default(),
            &|| false,
        )
        .unwrap();
        let decoded: Vec<_> = catalog
            .resources
            .iter()
            .map(|r| {
                let encoded = package
                    .read_part(&mo_opc::PartName::new(&r.part).unwrap(), 1 << 20, &|| false)
                    .unwrap();
                mo_image::decode(&encoded, &r.sha256, &mut NativeRaster, &|| false).unwrap()
            })
            .collect();
        let standalone =
            crate::source_image_layout::layout_source(&index, &catalog, &decoded, &|| false)
                .unwrap();
        for (a, b) in standalone.iter().zip(&plan.images.bindings) {
            // Standalone catalog IDs name encoded parts; the page deliberately
            // deduplicates identical bytes referenced through different parts.
            assert_eq!(
                catalog.resources[a.resource as usize].sha256,
                plan.images.decoded[b.layout.resource as usize].source_sha256
            );
            let mut a = a.clone();
            a.resource = b.layout.resource;
            assert_eq!(
                serde_json::to_value(&a).unwrap(),
                serde_json::to_value(&b.layout).unwrap()
            );
        }
        let result = crate::source_resource_page::prepare(
            &package,
            &index,
            &q,
            &mut Decoder::default(),
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
        let pixel =
            |x: usize, y: usize| &result.pixels[(y * q.viewport.width as usize + x) * 4..][..4];
        let x = if rtl { 144 } else { 0 };
        assert_eq!(pixel(x + 20, 10), [255, 0, 0, 255]);
        assert_eq!(pixel(x + 200, 10), [0, 255, 255, 255]);
        assert_eq!(pixel(x + 20, 130), [0, 0, 255, 255]);
        assert_eq!(pixel(x + 200, 130), [255, 255, 0, 255]);
        let x = if rtl { 0 } else { 288 };
        assert_eq!(pixel(x + 10, 10), [0, 255, 255, 255]);
        assert_eq!(pixel(x + 50, 10), [255, 0, 0, 255]);
        assert_eq!(pixel(x + 50, 65), [0, 0, 255, 255]);
        assert_eq!(pixel(x + 130, 10), [0, 255, 255, 255]);
        let x = if rtl { 288 } else { 0 };
        assert_eq!(pixel(x + 2, 146), [255, 0, 0, 255]);
        assert_eq!(pixel(x + 6, 146), [0, 255, 255, 255]);
        // XY mirror repeats in an eight-pixel tile then reverses the next tile.
        assert_eq!(pixel(x + 10, 146), [0, 255, 255, 255]);
        assert_eq!(pixel(x + 14, 146), [255, 0, 0, 255]);
        let name = format!("images-rtl-{rtl}");
        save(&name, "pptx", &bytes);
        save(&name, "request.json", &serde_json::to_vec(&q).unwrap());
        save(&name, "plan.json", &serde_json::to_vec(&plan).unwrap());
        save(&name, "rgba", &result.pixels);
    }
}

#[test]
fn table_image_relationship_and_orientation_fail_before_any_component() {
    for broken in ["missing-image", "owned-external"] {
        let bytes = rewrite(&image_cells(false), SLIDE, |s| {
            s.replace("r:embed=\"owned-copy\"", &format!("r:embed=\"{broken}\""))
        });
        let index = read(&bytes);
        let package = Package::open(
            bytes.as_slice(),
            bytes.len() as u64,
            Default::default(),
            &|| false,
        )
        .unwrap();
        let mut decoder = Decoder::default();
        let mut shaper = Shaper::default();
        let fonts = fonts();
        let manifest =
            PreparedManifest::load(&fonts, FONT_BYTES, Default::default(), &|| false).unwrap();
        assert!(
            crate::source_resource_page::prepare(
                &package,
                &index,
                &page_request(&index),
                &mut decoder,
                Some(TextPageContext {
                    manifest: &manifest,
                    backend: &mut shaper
                }),
                options(),
                &|| false
            )
            .is_err()
        );
        assert_eq!((decoder.calls, shaper.calls), (0, 0));
    }
    let bytes = rewrite(&image_cells(false), SLIDE, |s| {
        s.replace("rotWithShape=\"1\"", "rotWithShape=\"0\"")
    });
    let index = read(&bytes);
    let package = Package::open(
        bytes.as_slice(),
        bytes.len() as u64,
        Default::default(),
        &|| false,
    )
    .unwrap();
    let mut decoder = Decoder::default();
    let fonts = fonts();
    let manifest =
        PreparedManifest::load(&fonts, FONT_BYTES, Default::default(), &|| false).unwrap();
    let mut shaper = Shaper::default();
    let error = crate::source_resource_page::prepare(
        &package,
        &index,
        &page_request(&index),
        &mut decoder,
        Some(TextPageContext {
            manifest: &manifest,
            backend: &mut shaper,
        }),
        options(),
        &|| false,
    )
    .err()
    .unwrap();
    assert!(
        format!("{error:?}").contains("OrientationRequired"),
        "{error:?}"
    );
    assert_eq!(decoder.calls, 0);
    assert_eq!(shaper.calls, 0);
}

#[test]
fn retained_image_tables_reuse_decodes_through_motion_fade_visibility_and_replay() {
    let base = image_cells(true);
    for (name, bytes) in [
        ("motion", super::page::playback::motion(&base)),
        ("fade", super::page::playback::fade(&base)),
        (
            "hidden",
            rewrite(&base, SLIDE, |s| {
                s.replace(
                    "</p:sld>",
                    &format!(
                        "{}</p:sld>",
                        playback_support::visibility_timing(2, "hidden", "freeze")
                    ),
                )
            }),
        ),
        (
            "reveal",
            rewrite(&base, SLIDE, |s| {
                s.replace("<p:cNvPr id=\"2\"", "<p:cNvPr hidden=\"1\" id=\"2\"")
                    .replace(
                        "</p:sld>",
                        &format!(
                            "{}</p:sld>",
                            playback_support::visibility_timing(2, "visible", "freeze")
                        ),
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
        let mut decoder = Decoder::default();
        let mut retained = make()
            .retain(
                &package,
                index.clone(),
                &mut decoder,
                Some(TextPageContext {
                    manifest: &manifest,
                    backend: &mut NativeShaper::default(),
                }),
                options(),
                &|| false,
            )
            .unwrap();
        assert_eq!(decoder.calls, 2);
        let mut direct = make();
        let mut pixels = Vec::new();
        save(&format!("playback-{name}"), "pptx", &bytes);
        save(
            &format!("playback-{name}"),
            "request.json",
            &serde_json::to_vec(&q).unwrap(),
        );
        for (ordinal, ms) in [0, 250, 500, 750, 1000, 250].into_iter().enumerate() {
            let at = RationalTime::new(ms, 1000).unwrap();
            let (state, actual) = retained
                .render(at, None, &mut NativeRaster, &|| false)
                .unwrap();
            let mut fresh_decoder = Decoder::default();
            let sample = direct.sample(at, None, &|| false).unwrap();
            let expected = sample
                .prepare(
                    &package,
                    &index,
                    &mut fresh_decoder,
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
            assert_eq!(actual.pixels, expected.pixels);
            assert_eq!(actual.info.text_work.component_calls, 0);
            assert_eq!(actual.info.gather_copy_bytes, 0);
            assert_eq!(decoder.calls, 2);
            assert_eq!(
                serde_json::to_value(&state).unwrap(),
                serde_json::to_value(sample.frame()).unwrap()
            );
            if (name == "hidden" && ms >= 500) || (name == "reveal" && ms < 500) {
                assert!(actual.pixels.chunks_exact(4).all(|p| p == [255; 4]));
                assert_eq!(fresh_decoder.calls, 0);
            } else {
                assert_eq!(fresh_decoder.calls, 2);
            }
            save(
                &format!("playback-{name}-{ordinal}"),
                "state.json",
                &serde_json::to_vec(&state).unwrap(),
            );
            save(
                &format!("playback-{name}-{ordinal}"),
                "rgba",
                &actual.pixels,
            );
            pixels.push(actual.pixels);
        }
        assert_ne!(pixels[0], pixels[2]);
        assert_eq!(pixels[1], pixels[5]);
        if name == "fade" {
            for (a, b) in pixels[2].iter().zip(&pixels[4]) {
                assert!((i32::from(*a) - i32::from((u16::from(*b) + 255) / 2)).abs() <= 1);
            }
        }
    }
}
