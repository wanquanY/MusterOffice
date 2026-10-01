#[allow(dead_code)]
#[path = "../../../tools/test-support/source_text_page.rs"]
mod support;
use mo_common::Emu;
use mo_geometry::Fixed;
use mo_harfbuzz_sys::NativeShaper;
use mo_pptx::{AuthorPlan, ExportDefaults, NoResources};
use mo_presentation_compile::{source_frame, source_text_page};
use mo_skia_sys::NativeRaster;
use mo_text::manifest::{ManifestLimits, PreparedManifest};
use serde_json::json;

#[test]
fn authored_paragraph_layout_matches_native_files_pixels_and_independent_offsets() {
    let fonts = support::author();
    let manifest = PreparedManifest::load(
        &fonts.manifest,
        include_bytes!("../../../fixtures/fonts/owned.ttf"),
        ManifestLimits::default(),
        &|| false,
    )
    .unwrap();
    let defaults_input: serde_json::Value = serde_json::from_str(include_str!(
        "../../../fixtures/presentations/native-export/request.json"
    ))
    .unwrap();
    let mut defaults: ExportDefaults =
        serde_json::from_value(defaults_input["defaults"].clone()).unwrap();
    defaults.font_family = support::FONT.into();
    defaults.text_size = Emu::new(127_000);
    for (name, spacing, baseline_step) in [
        (
            "single",
            json!({"kind":"percent", "value":100_000}),
            127_000,
        ),
        (
            "one-and-half",
            json!({"kind":"percent", "value":150_000}),
            190_500,
        ),
        (
            "double",
            json!({"kind":"percent", "value":200_000}),
            254_000,
        ),
        ("exact", json!({"kind":"exact", "height":"317500"}), 317_500),
    ] {
        let mut value: serde_json::Value = serde_json::from_str(include_str!(
            "../../../fixtures/presentations/basic-shape.json"
        ))
        .unwrap();
        value["pageSize"] = json!({"width":"1600000","height":"1200000"});
        let object = &mut value["objects"]["shape:1"];
        object["transform"]["size"] = json!({"width":"1000000","height":"1200000"});
        let text = &mut object["content"]["text"];
        text["style"] = json!({"language":{"kind":"value","value":"en"}});
        text["paragraphs"][0]["style"] = json!({
            "lineSpacing":spacing,"leftMargin":"127000","rightMargin":"254000","indent":"-63500"
        });
        text["paragraphs"][0]["runs"] = json!([
            {"id":"run:1","style":{},"content":{"kind":"text","text":"A"}},
            {"id":"break:1","style":{},"content":{"kind":"break"}},
            {"id":"run:2","style":{},"content":{"kind":"text","text":"A"}},
            {"id":"break:2","style":{},"content":{"kind":"break"}},
            {"id":"run:3","style":{},"content":{"kind":"text","text":"A"}}
        ]);
        let document = serde_json::from_value(value).unwrap();
        let author = AuthorPlan::new(&document, &defaults, Default::default(), &|| false).unwrap();
        let file = mo_pptx::export_plan_to(
            &author,
            &NoResources,
            Vec::new(),
            Default::default(),
            &|| false,
        )
        .unwrap();
        let source =
            mo_pptx::source::inspect_source(file.package(), Default::default(), &|| false).unwrap();
        let mut expected_pixels = None;
        let mut expected_glyphs = None;
        for index in [author.declarations(), &source] {
            let request = source_frame::SourceFrameRequest {
                expected_source_sha256: index.source_sha256.clone(),
                object: support::target(index),
                bounds_tolerance: Fixed::from_raw(1 << 26),
            };
            let frame = source_frame::compile(
                index,
                &request,
                &manifest,
                &mut NativeShaper::default(),
                Default::default(),
                &|| false,
            )
            .unwrap();
            assert_eq!(frame.glyphs.len(), 3);
            let paragraph = &frame.paragraphs[0];
            assert_eq!(paragraph.spec.widths.rest, Fixed::emu(Emu::new(619_000)));
            assert_eq!(paragraph.spec.widths.first, Fixed::emu(Emu::new(682_500)));
            assert_eq!(paragraph.line_offsets[0].x, Fixed::emu(Emu::new(63_500)));
            assert_eq!(paragraph.line_offsets[1].x, Fixed::emu(Emu::new(127_000)));
            for pair in frame.glyphs.windows(2) {
                assert_eq!(
                    pair[1].origin.y.checked_sub(pair[0].origin.y).unwrap(),
                    Fixed::emu(Emu::new(baseline_step)),
                    "{name}"
                );
            }
            let glyphs = serde_json::to_value(&frame.glyphs).unwrap();
            if let Some(expected) = &expected_glyphs {
                assert_eq!(&glyphs, expected);
            }
            expected_glyphs = Some(glyphs);
            let page = support::request(index);
            let rendered = source_text_page::render(
                index,
                &page,
                &manifest,
                &mut NativeShaper::default(),
                &mut NativeRaster,
                Default::default(),
                &|| false,
            )
            .unwrap();
            if let Some(expected) = &expected_pixels {
                assert_eq!(&rendered.pixels, expected);
            }
            expected_pixels = Some(rendered.pixels);
        }
        if let Some(root) = std::env::var_os("MO_PARAGRAPH_LAYOUT_OUTPUT") {
            let root = std::path::PathBuf::from(root);
            assert!(root.is_dir());
            let save = |suffix: &str, bytes: &[u8]| {
                use std::io::Write;
                std::fs::OpenOptions::new()
                    .write(true)
                    .create_new(true)
                    .open(root.join(format!("{name}.{suffix}")))
                    .unwrap()
                    .write_all(bytes)
                    .unwrap();
            };
            let bytes = mo_pptx::export(
                &document,
                &defaults,
                &NoResources,
                Default::default(),
                &|| false,
            )
            .unwrap();
            save("pptx", &bytes);
            save("rgba", &expected_pixels.unwrap());
            save("request.json", &serde_json::to_vec(&json!({
                "profile":"drawingml-solid-text-page-q32-draft-v1", "page":support::request(&source),
                "fonts":fonts.manifest
            })).unwrap());
        }
    }
}
