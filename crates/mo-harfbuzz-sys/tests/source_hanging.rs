#[allow(dead_code)]
#[path = "../../../tools/test-support/source_glyphs.rs"]
mod support;
use mo_common::Emu;
use mo_geometry::Fixed;
use mo_harfbuzz_sys::NativeShaper;
use mo_presentation_compile::source_frame::*;
use mo_text::manifest::*;
use support::*;

fn emu(value: i64) -> Fixed {
    Fixed::emu(Emu::new(value))
}
fn frame(text: &str, attribute: &str, alignment: &str, rtl: bool) -> SourceFramePlan {
    // At 18pt each owned glyph advances exactly 137160 EMU. The frame has
    // room for two glyphs and 10000 EMU spare, but not three glyphs.
    let paragraphs = format!(
        "<a:p><a:pPr {attribute} algn=\"{alignment}\" rtl=\"{}\"/><a:r><a:rPr sz=\"1800\"/><a:t>{text}</a:t></a:r></a:p>",
        u8::from(rtl)
    );
    let b = rewrite(&bytes(&paragraphs), SLIDE, |mut s| {
        s = s
            .replace("typeface=\"EA\"", &format!("typeface=\"{FONT}\""))
            .replace("typeface=\"CS\"", &format!("typeface=\"{FONT}\""));
        s = s.replace(
            "<a:bodyPr/>",
            "<a:bodyPr lIns=\"0\" rIns=\"0\" tIns=\"0\" bIns=\"0\"><a:noAutofit/></a:bodyPr>",
        );
        let from = s.find("<p:spPr>").unwrap();
        let to = from + s[from..].find("</p:spPr>").unwrap() + "</p:spPr>".len();
        s.replace_range(from..to, "<p:spPr><a:xfrm><a:off x=\"0\" y=\"0\"/><a:ext cx=\"284320\" cy=\"2000000\"/></a:xfrm><a:prstGeom prst=\"rect\"><a:avLst/></a:prstGeom></p:spPr>");
        s
    });
    let index = read(&b);
    let author: ManifestParagraphRequest = serde_json::from_str(include_str!(
        "../../../fixtures/fonts/manifest-hanging.json"
    ))
    .unwrap();
    let manifest = PreparedManifest::load(
        &author.manifest,
        include_bytes!("../../../fixtures/fonts/owned-hanging.ttf"),
        ManifestLimits::default(),
        &|| false,
    )
    .unwrap();
    let plan = compile(
        &index,
        &SourceFrameRequest {
            expected_source_sha256: index.source_sha256.clone(),
            object: target(&index),
            bounds_tolerance: Fixed::from_raw(1 << 26),
        },
        &manifest,
        &mut NativeShaper::default(),
        SourceFrameLimits::default(),
        &|| false,
    )
    .unwrap();
    if let Some(dir) = std::env::var_os("MO_HANGING_EVIDENCE_DIR") {
        let dir = std::path::PathBuf::from(dir);
        assert!(dir.is_absolute());
        std::fs::create_dir_all(&dir).unwrap();
        let stem = index.source_sha256.as_str();
        std::fs::write(dir.join(format!("{stem}.pptx")), b).unwrap();
        std::fs::write(
            dir.join(format!("{stem}.json")),
            serde_json::to_vec_pretty(&plan).unwrap(),
        )
        .unwrap();
    }
    plan
}
#[test]
fn real_shaper_retains_punctuation_and_aligns_the_non_hanging_body() {
    for (text, rtl, body_min, body_max) in
        [("中中。", false, 0, 274320), ("אב!", true, 137160, 411480)]
    {
        for (alignment, offset) in [
            ("l", -body_min),
            ("r", 284320 - body_max),
            ("ctr", (284320 - body_min - body_max) / 2),
        ] {
            for attribute in ["", "hangingPunct=\"1\""] {
                let result = frame(text, attribute, alignment, rtl);
                let p = &result.paragraphs[0];
                let layout = &p.computed.geometry.paths.layout;
                assert_eq!(layout.decisions.len(), 1);
                assert!(!layout.decisions[0].overflows);
                let h = layout.decisions[0].hanging.as_ref().unwrap();
                assert_eq!((h.start.scalar_offset, h.end.scalar_offset), (2, 3));
                assert_eq!(
                    (h.body_pen_min, h.body_pen_max),
                    (emu(body_min), emu(body_max))
                );
                assert_eq!(p.line_offsets[0].x, emu(offset));
                assert_eq!(result.glyphs.len(), 3);
                let capacity = capacity::measure(&result, &|| false).unwrap();
                assert_eq!(capacity.horizontal_overflow_lines, 0);
                assert_eq!(capacity.maximum_left_excess, emu((-offset).max(0)));
                assert_eq!(
                    capacity.maximum_right_excess,
                    emu((411480 + offset - 284320).max(0))
                );
                assert_eq!(
                    p.computed.geometry.precise.as_ref().unwrap().lines[0].advance,
                    emu(411480)
                );
            }
        }
    }
}
#[test]
fn false_attribute_and_excess_body_width_preserve_overflow_diagnostics() {
    for (text, attribute) in [
        ("AA!", "hangingPunct=\"0\""),
        ("AAA!", "hangingPunct=\"1\""),
    ] {
        let result = frame(text, attribute, "l", false);
        let layout = &result.paragraphs[0].computed.geometry.paths.layout;
        assert_eq!(layout.decisions.len(), 1);
        assert!(layout.decisions[0].overflows);
        assert!(layout.decisions[0].hanging.is_none());
        assert_eq!(result.glyphs.len(), text.chars().count());
    }
}
