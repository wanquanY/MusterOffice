#[allow(dead_code)]
#[path = "../../../tools/test-support/source_glyphs.rs"]
mod support;
use mo_common::Emu;
use mo_geometry::Fixed;
use mo_harfbuzz_sys::NativeShaper;
use mo_presentation_compile::source_frame::*;
use mo_text::{TextError, backend::TextBackend, manifest::*};
use std::cell::Cell;
use support::*;

// Native component invalidation is process-wide and permanent. Each test and
// every destructive/cancellation case gets a fresh process, never a reset API.
fn subprocess(name: &str, case: Option<&str>) {
    let mut command = std::process::Command::new(std::env::current_exe().unwrap());
    command
        .args(["--exact", name, "--test-threads=1"])
        .env("MO_FRAME_TEST_CHILD", name);
    if let Some(case) = case {
        command.env("MO_FRAME_TEST_CASE", case);
    }
    let result = command.output().unwrap();
    assert!(
        result.status.success(),
        "child {name}/{case:?}: {}\n{}",
        String::from_utf8_lossy(&result.stdout),
        String::from_utf8_lossy(&result.stderr)
    );
}
fn isolate(name: &str) -> bool {
    if std::env::var("MO_FRAME_TEST_CHILD").ok().as_deref() == Some(name) {
        false
    } else {
        subprocess(name, None);
        true
    }
}
fn isolated_case(case: &str, run: impl FnOnce()) {
    match std::env::var("MO_FRAME_TEST_CASE").ok() {
        Some(selected) if selected == case => run(),
        Some(_) => (),
        None => subprocess(&std::env::var("MO_FRAME_TEST_CHILD").unwrap(), Some(case)),
    }
}
fn fixture(paragraphs: &str, body: &str) -> Vec<u8> {
    let b = bytes(paragraphs);
    rewrite(&b, SLIDE, |mut s| {
        s = s.replace(
            "<a:bodyPr/>",
            &format!("<a:bodyPr {body}><a:noAutofit/></a:bodyPr>"),
        );
        let start = s.find("<p:spPr>").unwrap();
        let end = start + s[start..].find("</p:spPr>").unwrap() + "</p:spPr>".len();
        s.replace_range(start..end, "<p:spPr><a:xfrm><a:off x=\"0\" y=\"0\"/><a:ext cx=\"1000000\" cy=\"2000000\"/></a:xfrm><a:prstGeom prst=\"rect\"><a:avLst/></a:prstGeom></p:spPr>");
        s
    })
}
fn author() -> ManifestParagraphRequest {
    serde_json::from_str(include_str!(
        "../../../fixtures/fonts/manifest-paragraph.json"
    ))
    .unwrap()
}
fn request(index: &mo_pptx::source::SourceIndex) -> SourceFrameRequest {
    SourceFrameRequest {
        expected_source_sha256: index.source_sha256.clone(),
        object: target(index),
        bounds_tolerance: Fixed::from_raw(1 << 26),
    }
}
fn frame(bytes: &[u8]) -> SourceFramePlan {
    let index = read(bytes);
    let author = author();
    let manifest = PreparedManifest::load(
        &author.manifest,
        include_bytes!("../../../fixtures/fonts/owned.ttf"),
        ManifestLimits::default(),
        &|| false,
    )
    .unwrap();
    let plan = compile(
        &index,
        &request(&index),
        &manifest,
        &mut NativeShaper::default(),
        SourceFrameLimits::default(),
        &|| false,
    )
    .unwrap();
    if let Some(directory) = std::env::var_os("MO_SOURCE_FRAME_EVIDENCE_DIR") {
        let directory = std::path::PathBuf::from(directory);
        assert!(directory.is_absolute());
        std::fs::create_dir_all(&directory).unwrap();
        let stem = index.source_sha256.as_str();
        std::fs::write(directory.join(format!("{stem}.pptx")), bytes).unwrap();
        std::fs::write(
            directory.join(format!("{stem}.json")),
            serde_json::to_vec_pretty(&plan).unwrap(),
        )
        .unwrap();
    }
    plan
}
fn emu(n: i64) -> Fixed {
    Fixed::emu(Emu::new(n))
}
#[test]
fn native_insets_margins_indent_and_fractional_available_width_drive_real_lines() {
    if isolate("native_insets_margins_indent_and_fractional_available_width_drive_real_lines") {
        return;
    }
    let b = fixture(
        "<a:p><a:pPr marL=\"10000\" marR=\"20000\" indent=\"30000\" latinLnBrk=\"1\"/><a:r><a:t>AAAAAAAAAA</a:t></a:r></a:p>",
        "lIns=\"1.00001pt\" rIns=\"2pt\" tIns=\"3pt\" bIns=\"4pt\"",
    );
    let r = frame(&b);
    let p = &r.paragraphs[0];
    assert_eq!(r.region.inner.min.y, emu(38100));
    assert!(r.region.inner.min.x > emu(12700) && r.region.inner.min.x < emu(12701));
    assert_eq!(
        p.spec.widths.rest.checked_sub(p.spec.widths.first).unwrap(),
        emu(30000)
    );
    assert_eq!(
        p.line_offsets[0].x,
        r.region.inner.min.x.checked_add(emu(40000)).unwrap()
    );
    assert!(p.line_offsets.len() > 1);
    assert_eq!(
        p.line_offsets[1].x,
        r.region.inner.min.x.checked_add(emu(10000)).unwrap()
    );
    assert_eq!(r.work.glyphs as usize, r.glyphs.len());
}
#[test]
fn exact_line_spacing_outer_paragraph_edges_and_vertical_anchors() {
    if isolate("exact_line_spacing_outer_paragraph_edges_and_vertical_anchors") {
        return;
    }
    let p = "<a:p><a:pPr><a:lnSpc><a:spcPts val=\"2000\"/></a:lnSpc><a:spcBef><a:spcPts val=\"1000\"/></a:spcBef><a:spcAft><a:spcPts val=\"2000\"/></a:spcAft></a:pPr><a:r><a:t>A</a:t></a:r></a:p>";
    for outer in [false, true] {
        for anchor in ["t", "ctr", "b"] {
            let r = frame(&fixture(
                &(p.to_owned() + p),
                &format!(
                    "spcFirstLastPara=\"{}\" anchor=\"{anchor}\"",
                    u8::from(outer)
                ),
            ));
            let height = if outer { 1_270_000 } else { 889_000 };
            assert_eq!(r.content_height, emu(height));
            let free = r
                .region
                .inner
                .max
                .y
                .checked_sub(r.region.inner.min.y)
                .unwrap()
                .checked_sub(r.content_height)
                .unwrap();
            let shift = match anchor {
                "t" => Fixed::ZERO,
                "ctr" => free.half().unwrap(),
                _ => free,
            };
            let top = r
                .region
                .inner
                .min
                .y
                .checked_add(shift)
                .unwrap()
                .checked_add(emu(if outer { 127000 } else { 0 }))
                .unwrap();
            assert_eq!(r.paragraphs[0].line_offsets[0].y, top);
            assert_eq!(
                r.paragraphs[1].line_offsets[0].y.checked_sub(top).unwrap(),
                emu(635000)
            );
        }
    }
}
#[test]
fn left_center_right_and_rtl_indent_keep_precise_visual_pen_alignment() {
    if isolate("left_center_right_and_rtl_indent_keep_precise_visual_pen_alignment") {
        return;
    }
    for alignment in ["l", "ctr", "r"] {
        for rtl in [false, true] {
            let r = frame(&fixture(
                &format!(
                    "<a:p><a:pPr rtl=\"{}\" algn=\"{alignment}\" indent=\"20000\"/><a:r><a:t>A</a:t></a:r></a:p>",
                    u8::from(rtl)
                ),
                "",
            ));
            let p = &r.paragraphs[0];
            let line = &p.computed.geometry.precise.as_ref().unwrap().lines[0];
            let left = r
                .region
                .inner
                .min
                .x
                .checked_add(emu(if rtl { 0 } else { 20000 }))
                .unwrap();
            let offset = p.line_offsets[0].x.checked_sub(left).unwrap();
            match alignment {
                "l" => assert_eq!(offset, Fixed::ZERO),
                "r" => assert_eq!(
                    offset.checked_add(line.pen_max).unwrap(),
                    p.spec.widths.first
                ),
                _ => assert!(
                    (offset.raw() * 2 + line.pen_max.raw() + line.pen_min.raw()
                        - p.spec.widths.first.raw())
                    .abs()
                        <= 1
                ),
            }
        }
    }
}
#[test]
fn empty_paragraph_and_missing_native_rectangle_use_source_bound_shape_extent() {
    if isolate("empty_paragraph_and_missing_native_rectangle_use_source_bound_shape_extent") {
        return;
    }
    let b = fixture("<a:p><a:endParaRPr sz=\"2301\"/></a:p>", "");
    let b = rewrite(&b, SLIDE, |s| {
        s.replace(
            "<a:prstGeom prst=\"rect\"><a:avLst/></a:prstGeom>",
            "<a:custGeom><a:avLst/><a:gdLst/><a:ahLst/><a:cxnLst/><a:pathLst/></a:custGeom>",
        )
    });
    let r = frame(&b);
    assert!(matches!(
        r.region.source,
        TextRectangleSource::ShapeBounds { .. }
    ));
    assert!(r.glyphs.is_empty() && r.bounds.is_none());
    assert!(r.content_height > Fixed::ZERO);
}

#[derive(Default)]
struct Counting {
    native: NativeShaper,
    calls: usize,
    fail: Option<usize>,
}

#[test]
fn fractional_line_heights_accumulate_before_any_integer_wire_rounding() {
    if isolate("fractional_line_heights_accumulate_before_any_integer_wire_rounding") {
        return;
    }
    let r = frame(&fixture(
        "<a:p><a:endParaRPr sz=\"2301\"/></a:p><a:p><a:endParaRPr sz=\"2301\"/></a:p>",
        "",
    ));
    let first = r.paragraphs[0]
        .computed
        .geometry
        .precise
        .as_ref()
        .unwrap()
        .height;
    let second = r.paragraphs[1]
        .computed
        .geometry
        .precise
        .as_ref()
        .unwrap()
        .height;
    assert_eq!(
        first,
        Fixed::scale(1025, Emu::new(2301 * 127), 1000).unwrap()
    );
    assert_eq!(r.content_height, first.checked_add(second).unwrap());
    let rounded = emu(first.wire().unwrap().get() + second.wire().unwrap().get());
    assert_ne!(r.content_height, rounded);
    assert_eq!(
        r.paragraphs[1].line_offsets[0]
            .y
            .checked_sub(r.paragraphs[0].line_offsets[0].y)
            .unwrap(),
        first
    );
}
impl Counting {
    fn before(&mut self) -> Result<(), TextError> {
        self.calls += 1;
        if self.fail == Some(self.calls) {
            Err(TextError::Host("injected frame component failure"))
        } else {
            Ok(())
        }
    }
}
impl TextBackend for Counting {
    fn shape_batch(&mut self, f: &[u8], r: &[u32]) -> Result<Vec<u32>, TextError> {
        self.before()?;
        self.native.shape_batch(f, r)
    }
    fn measure_batch(&mut self, f: &[u8], r: &[u32]) -> Result<Vec<u32>, TextError> {
        self.before()?;
        self.native.measure_batch(f, r)
    }
    fn outline_batch(&mut self, f: &[u8], r: &[u32]) -> Result<Vec<u32>, TextError> {
        self.before()?;
        self.native.outline_batch(f, r)
    }
    fn invalidate(&mut self) {
        self.native.invalidate();
    }
}
#[test]
fn late_paragraph_preflight_and_frame_work_limits_never_publish_partial_geometry() {
    if isolate("late_paragraph_preflight_and_frame_work_limits_never_publish_partial_geometry") {
        return;
    }
    let author = author();
    let manifest = PreparedManifest::load(
        &author.manifest,
        include_bytes!("../../../fixtures/fonts/owned.ttf"),
        ManifestLimits::default(),
        &|| false,
    )
    .unwrap();
    let i = read(&fixture(
        "<a:p><a:r><a:t>A</a:t></a:r></a:p><a:p><a:r><a:rPr b=\"1\"/><a:t>A</a:t></a:r></a:p>",
        "",
    ));
    let mut backend = Counting::default();
    assert!(
        compile(
            &i,
            &request(&i),
            &manifest,
            &mut backend,
            SourceFrameLimits::default(),
            &|| false
        )
        .is_err()
    );
    assert_eq!(backend.calls, 0);
    let i = read(&fixture(
        "<a:p><a:r><a:t>A</a:t></a:r></a:p><a:p><a:r><a:t>A</a:t></a:r></a:p>",
        "",
    ));
    let good = compile(
        &i,
        &request(&i),
        &manifest,
        &mut Counting::default(),
        SourceFrameLimits::default(),
        &|| false,
    )
    .unwrap();
    for max in [0, 1, good.work.component_calls - 1] {
        isolated_case(&format!("budget-{max}"), || {
            let mut backend = Counting::default();
            let limits = SourceFrameLimits {
                max_component_calls: max,
                ..SourceFrameLimits::default()
            };
            assert!(compile(&i, &request(&i), &manifest, &mut backend, limits, &|| false).is_err());
            assert_eq!(backend.calls, max as usize);
        });
    }
    for fail in 1..=good.work.component_calls as usize {
        isolated_case(&format!("failure-{fail}"), || {
            let mut backend = Counting {
                fail: Some(fail),
                ..Default::default()
            };
            assert!(
                compile(
                    &i,
                    &request(&i),
                    &manifest,
                    &mut backend,
                    SourceFrameLimits::default(),
                    &|| false
                )
                .is_err()
            );
            assert_eq!(backend.calls, fail);
        });
    }
    for (resource, total) in [
        ("font-bytes", good.work.font_upload_bytes),
        ("request-words", good.work.request_words),
    ] {
        for max in [0, total - 1] {
            isolated_case(&format!("{resource}-{max}"), || {
                let mut backend = Counting::default();
                let mut limits = SourceFrameLimits::default();
                if resource == "font-bytes" {
                    limits.max_font_upload_bytes = max;
                } else {
                    limits.max_request_words = max;
                }
                assert!(
                    compile(&i, &request(&i), &manifest, &mut backend, limits, &|| false).is_err()
                );
                assert!(backend.calls < good.work.component_calls as usize);
                if max == 0 {
                    assert_eq!(backend.calls, 0);
                }
            });
        }
    }
    if std::env::var_os("MO_FRAME_TEST_CASE").is_some() {
        return;
    }
    let limits = SourceFrameLimits {
        max_glyphs: 1,
        ..SourceFrameLimits::default()
    };
    assert!(
        compile(
            &i,
            &request(&i),
            &manifest,
            &mut Counting::default(),
            limits,
            &|| false
        )
        .is_err()
    );
    assert_eq!(
        compile(
            &i,
            &request(&i),
            &manifest,
            &mut Counting::default(),
            SourceFrameLimits::default(),
            &|| false
        )
        .unwrap()
        .glyphs
        .len(),
        2
    );
}
#[test]
fn selected_cancellation_and_unsupported_native_properties_fail_explicitly() {
    if isolate("selected_cancellation_and_unsupported_native_properties_fail_explicitly") {
        return;
    }
    let author = author();
    let manifest = PreparedManifest::load(
        &author.manifest,
        include_bytes!("../../../fixtures/fonts/owned.ttf"),
        ManifestLimits::default(),
        &|| false,
    )
    .unwrap();
    let i = read(&fixture("<a:p><a:r><a:t>A</a:t></a:r></a:p>", ""));
    let before = i.clone();
    let count = Cell::new(0usize);
    compile(
        &i,
        &request(&i),
        &manifest,
        &mut Counting::default(),
        SourceFrameLimits::default(),
        &|| {
            count.set(count.get() + 1);
            false
        },
    )
    .unwrap();
    let total = count.get();
    let mut points: Vec<_> = (1..=16).map(|n| (total * n / 16).max(1)).collect();
    points.push(1);
    for at in points {
        isolated_case(&format!("cancel-{at}"), || {
            let seen = Cell::new(0usize);
            let result = compile(
                &i,
                &request(&i),
                &manifest,
                &mut Counting::default(),
                SourceFrameLimits::default(),
                &|| {
                    seen.set(seen.get() + 1);
                    seen.get() == at
                },
            );
            assert!(result.is_err(), "checkpoint {at}/{total}");
        });
    }
    if std::env::var_os("MO_FRAME_TEST_CASE").is_some() {
        return;
    }
    assert_eq!(i, before);
    for attrs in [
        "numCol=\"2\"",
        "vert=\"vert\"",
        "anchorCtr=\"1\"",
        "vertOverflow=\"ellipsis\"",
    ] {
        let i = read(&fixture("<a:p><a:r><a:t>A</a:t></a:r></a:p>", attrs));
        let mut backend = Counting::default();
        assert!(matches!(
            compile(
                &i,
                &request(&i),
                &manifest,
                &mut backend,
                SourceFrameLimits::default(),
                &|| false
            ),
            Err(SourceFrameError::Mapping(_))
        ));
        assert_eq!(backend.calls, 0);
    }
    // Wrapping and end punctuation have dedicated positive real-shaper coverage.
    // Justification is still a preflight prerequisite, before component work.
    let i = read(&fixture(
        "<a:p><a:pPr algn=\"just\"/><a:r><a:t>A</a:t></a:r></a:p>",
        "",
    ));
    let mut backend = Counting::default();
    assert!(matches!(
        compile(
            &i,
            &request(&i),
            &manifest,
            &mut backend,
            SourceFrameLimits::default(),
            &|| false
        ),
        Err(SourceFrameError::Mapping(_))
    ));
    assert_eq!(backend.calls, 0);
    let i = read(&fixture("<a:p><a:r><a:t>Z</a:t></a:r></a:p>", ""));
    let failure = compile(
        &i,
        &request(&i),
        &manifest,
        &mut Counting::default(),
        SourceFrameLimits::default(),
        &|| false,
    )
    .unwrap_err();
    let SourceFrameError::Mapping(issue) = failure else {
        panic!("{failure:?}")
    };
    assert!(
        matches!(*issue, SourceFrameIssue::IncompleteParagraph { paragraph: 0, ref flow, .. } if !flow.is_empty())
    );
}

#[test]
fn no_wrap_keeps_native_text_and_actual_alignment_with_explicit_breaks() {
    if isolate("no_wrap_keeps_native_text_and_actual_alignment_with_explicit_breaks") {
        return;
    }
    for (align, rtl) in [
        ("l", false),
        ("ctr", false),
        ("r", false),
        ("l", true),
        ("ctr", true),
        ("r", true),
    ] {
        let paragraphs = format!(
            "<a:p><a:pPr algn=\"{align}\" rtl=\"{}\" latinLnBrk=\"1\"/><a:r><a:rPr sz=\"1800\"/><a:t>AAAAA AAAAA</a:t></a:r><a:br/><a:r><a:rPr sz=\"1800\"/><a:t>A</a:t></a:r><a:br/></a:p>",
            u8::from(rtl)
        );
        let b = rewrite(&fixture(&paragraphs, "wrap=\"none\""), SLIDE, |s| {
            let mut s = s.replacen("<p:sld ", "<p:sld showMasterSp=\"0\" ", 1);
            let from = s.find("</p:sp>").unwrap() + "</p:sp>".len();
            let to = s.find("</p:spTree>").unwrap();
            s.replace_range(from..to, "");
            s
        });
        let original = read(&b);
        let plan = frame(&b);
        assert_eq!(read(&b), original);
        assert_eq!(plan.inputs[0].text, "AAAAA AAAAA\u{2028}A\u{2028}");
        let p = &plan.paragraphs[0];
        assert_eq!(p.spec.wrapping, mo_text::flow::LineWrapping::NoWrap);
        let layout = &p.computed.geometry.paths.layout;
        assert_eq!(layout.decisions.len(), 3);
        assert_eq!(layout.work.evaluated_candidates, 3);
        assert!(layout.decisions[0].overflows);
        assert!(layout.decisions.iter().all(|d| !d.emergency));
        let wide = frame(&rewrite(&b, SLIDE, |s| {
            s.replace("cx=\"1000000\"", "cx=\"6000000\"")
        }));
        let lines = &p.computed.geometry.precise.as_ref().unwrap().lines;
        let wide_lines = &wide.paragraphs[0]
            .computed
            .geometry
            .precise
            .as_ref()
            .unwrap()
            .lines;
        assert_eq!(lines.len(), wide_lines.len());
        assert_eq!(lines[0].advance, wide_lines[0].advance);
        assert_eq!(plan.glyphs.len(), wide.glyphs.len());
        let width = p.spec.widths.first;
        let line = &lines[0];
        let expected = match align {
            "l" => Fixed::ZERO,
            "r" => width.checked_sub(line.pen_max).unwrap(),
            "ctr" => width
                .checked_sub(line.pen_max)
                .unwrap()
                .checked_sub(line.pen_min)
                .unwrap()
                .half()
                .unwrap(),
            _ => unreachable!(),
        };
        let base = plan.region.inner.min.x.checked_add(p.spec.left).unwrap();
        assert_eq!(p.line_offsets[0].x.checked_sub(base).unwrap(), expected);
        let capacity = capacity::measure(&plan, &|| false).unwrap();
        assert_eq!(capacity.line_count, 3);
        assert_eq!(capacity.horizontal_overflow_lines, 1);
        assert_eq!(capacity.emergency_lines, 0);
        assert!(
            capacity.maximum_left_excess > Fixed::ZERO
                || capacity.maximum_right_excess > Fixed::ZERO
        );
        if let Some(root) = std::env::var_os("MO_NOWRAP_OUTPUT") {
            let root = std::path::PathBuf::from(root);
            std::fs::create_dir_all(&root).unwrap();
            let stem = original.source_sha256.as_str();
            std::fs::write(root.join(format!("{stem}.pptx")), &b).unwrap();
            std::fs::write(
                root.join(format!("{stem}.json")),
                serde_json::to_vec_pretty(&capacity).unwrap(),
            )
            .unwrap();
        }
        let wrapping = frame(&rewrite(&b, SLIDE, |s| {
            s.replace("wrap=\"none\"", "wrap=\"square\"")
        }));
        assert!(
            wrapping.paragraphs[0]
                .computed
                .geometry
                .paths
                .layout
                .decisions
                .len()
                > 3
        );
    }
}

#[test]
fn unsupported_native_tab_semantics_fail_before_component_work() {
    let fonts = author();
    let manifest = PreparedManifest::load(
        &fonts.manifest,
        include_bytes!("../../../fixtures/fonts/owned.ttf"),
        Default::default(),
        &|| false,
    )
    .unwrap();
    for properties in [
        "<a:pPr defTabSz=\"0\"/>",
        "<a:pPr rtl=\"1\"/>",
        "<a:pPr algn=\"ctr\"/>",
        "<a:pPr><a:tabLst><a:tab pos=\"900000\" algn=\"ctr\"/></a:tabLst></a:pPr>",
        "<a:pPr><a:tabLst><a:tab pos=\"900000\" algn=\"r\"/></a:tabLst></a:pPr>",
        "<a:pPr><a:tabLst><a:tab pos=\"900000\" algn=\"dec\"/></a:tabLst></a:pPr>",
        "<a:pPr><a:tabLst><a:tab pos=\"900000\"/></a:tabLst></a:pPr>",
        "<a:pPr><a:tabLst><a:tab pos=\"900000\" algn=\"l\"/><a:tab pos=\"800000\" algn=\"l\"/></a:tabLst></a:pPr>",
    ] {
        let i = read(&fixture(
            &format!("<a:p>{properties}<a:r><a:t>A\tA</a:t></a:r></a:p>"),
            "",
        ));
        let mut backend = Counting::default();
        assert!(
            compile(
                &i,
                &request(&i),
                &manifest,
                &mut backend,
                Default::default(),
                &|| false
            )
            .is_err(),
            "{properties}"
        );
        assert_eq!(backend.calls, 0);
    }
    for property in ["u=\"sng\"", "strike=\"sngStrike\""] {
        let i = read(&fixture(
            &format!("<a:p><a:r><a:rPr {property}/><a:t>A\tA</a:t></a:r></a:p>"),
            "",
        ));
        let mut backend = Counting::default();
        assert!(
            compile(
                &i,
                &request(&i),
                &manifest,
                &mut backend,
                Default::default(),
                &|| false
            )
            .is_err()
        );
        assert_eq!(backend.calls, 0);
    }
}
