//! Real native shaping/line layout; no synthetic capacity verdicts.
#[allow(dead_code)]
#[path = "../../../tools/test-support/source_glyphs.rs"]
mod support;
use mo_common::Emu;
use mo_geometry::Fixed;
use mo_harfbuzz_sys::NativeShaper;
use mo_presentation_compile::source_frame::{
    self, SourceFrameLimits, SourceFrameRequest, capacity,
};
use mo_text::manifest::{ManifestLimits, ManifestParagraphRequest, PreparedManifest};
use support::*;

fn measured(
    text: &str,
    width: i64,
    height: i64,
    anchor: &str,
    emergency: bool,
) -> capacity::FrameCapacity {
    let source = bytes(&format!(
        "<a:p><a:pPr latinLnBrk=\"{}\"><a:lnSpc><a:spcPts val=\"1000\"/></a:lnSpc></a:pPr><a:r><a:rPr sz=\"1000\"/><a:t>{text}</a:t></a:r></a:p>",
        u8::from(emergency)
    ));
    let source = rewrite(&source, SLIDE, |mut xml| {
        xml = xml.replacen("<p:sld ", "<p:sld showMasterSp=\"0\" ", 1);
        xml = xml.replace("<a:bodyPr/>", &format!("<a:bodyPr lIns=\"0\" rIns=\"0\" tIns=\"0\" bIns=\"0\" anchor=\"{anchor}\"><a:noAutofit/></a:bodyPr>"));
        let start = xml.find("<p:spPr>").unwrap();
        let end = start + xml[start..].find("</p:spPr>").unwrap() + "</p:spPr>".len();
        xml.replace_range(start..end,&format!("<p:spPr><a:xfrm><a:off x=\"0\" y=\"0\"/><a:ext cx=\"{width}\" cy=\"{height}\"/></a:xfrm><a:prstGeom prst=\"rect\"><a:avLst/></a:prstGeom></p:spPr>"));
        // A focused text-only page exercises both text and resource renderers.
        // The shared export fixture's unrelated pictures belong to other tests.
        let first_shape_end = xml.find("</p:sp>").unwrap() + "</p:sp>".len();
        let tree_end = xml.find("</p:spTree>").unwrap();
        xml.replace_range(first_shape_end..tree_end, "");
        xml
    });
    let index = read(&source);
    let definition: ManifestParagraphRequest = serde_json::from_str(include_str!(
        "../../../fixtures/fonts/manifest-paragraph.json"
    ))
    .unwrap();
    let manifest = PreparedManifest::load(
        &definition.manifest,
        include_bytes!("../../../fixtures/fonts/owned.ttf"),
        ManifestLimits::default(),
        &|| false,
    )
    .unwrap();
    let frame = source_frame::compile(
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
    let measurement = capacity::measure(&frame, &|| false).unwrap();
    assert_eq!(measurement.object, target(&index));
    if let Some(root) = std::env::var_os("MO_CAPACITY_OUTPUT") {
        let root = std::path::PathBuf::from(root);
        std::fs::create_dir_all(&root).unwrap();
        let id = index.source_sha256.as_str();
        std::fs::write(root.join(format!("{id}.pptx")), source).unwrap();
        std::fs::write(
            root.join(format!("{id}.json")),
            serde_json::to_vec_pretty(&measurement).unwrap(),
        )
        .unwrap();
    }
    measurement
}
fn emu(n: i64) -> Fixed {
    Fixed::emu(Emu::new(n))
}

#[test]
fn native_capacity_distinguishes_logical_height_and_unbreakable_width_without_losing_text() {
    let normal = measured("AAAAAAAAAA", 1_000_000, 2_000_000, "t", false);
    assert_eq!(normal.line_count, 1);
    assert_eq!(normal.content_height, emu(127000)); // authored exact 10pt line spacing
    assert_eq!(normal.vertical_excess, Fixed::ZERO);
    assert_eq!(normal.horizontal_overflow_lines, 0);
    let narrow = measured("AAAAAAAAAA", 10000, 100000, "t", false);
    assert_eq!(narrow.line_count, 1);
    assert_eq!(narrow.horizontal_overflow_lines, 1);
    assert_eq!(narrow.content_height, normal.content_height);
    assert_eq!(narrow.vertical_excess, emu(27000));
    assert!(narrow.maximum_right_excess > Fixed::ZERO);
    assert_eq!(narrow.first_horizontal_overflow.as_ref().unwrap().line, 0);
    // Every source character remains in the same unbroken glyph run.
    assert_eq!(narrow.ink_bounds, normal.ink_bounds);
}

#[test]
fn emergency_wrap_reports_each_overwide_cluster_and_anchor_keeps_capacity_invariant() {
    let top = measured("AAAA", 10000, 100000, "t", true);
    assert_eq!(top.line_count, 4);
    assert_eq!(top.horizontal_overflow_lines, 4);
    assert_eq!(top.content_height, emu(508000));
    assert_eq!(top.vertical_excess, emu(408000));
    assert!(top.emergency_lines > 0);
    for anchor in ["ctr", "b"] {
        let anchored = measured("AAAA", 10000, 100000, anchor, true);
        assert_eq!(anchored.vertical_excess, top.vertical_excess);
        assert_eq!(
            anchored.horizontal_overflow_lines,
            top.horizontal_overflow_lines
        );
        assert_ne!(anchored.ink_bounds, top.ink_bounds);
    }
}

#[test]
fn capacity_arithmetic_coverage_and_cancellation_are_checked_without_quality_claims() {
    let frame = measured("A", 1_000_000, 2_000_000, "t", false);
    let report = capacity::TextCapacity {
        page_ink: None,
        profile: capacity::PROFILE.into(),
        frames: vec![frame],
    };
    report.validate(1, &|| false).unwrap();
    assert!(report.validate(0, &|| false).is_err());
    assert!(matches!(
        report.validate(1, &|| true),
        Err(source_frame::SourceFrameError::Cancelled)
    ));
    let mut changed = report.clone();
    changed.frames[0].vertical_excess = emu(1);
    assert!(changed.validate(1, &|| false).is_err());
    let mut changed = report.clone();
    changed.frames.push(changed.frames[0].clone());
    assert!(changed.validate(2, &|| false).is_err());
    let mut changed = report.clone();
    changed.frames[0].horizontal_overflow_lines = 1;
    assert!(changed.validate(1, &|| false).is_err());
    let json = serde_json::to_value(&report).unwrap();
    assert!(json.get("passed").is_none());
    assert!(json.get("quality").is_none());
}
