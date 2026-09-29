#[allow(dead_code)]
#[path = "../../../tools/test-support/source_text_page.rs"]
mod support;
use mo_geometry::Fixed;
use mo_harfbuzz_sys::NativeShaper;
use mo_presentation_compile::{
    source_page::*,
    source_text_page::{self, *},
};
use mo_raster::{BackendReply, RasterBackend, RasterError};
use mo_skia_sys::NativeRaster;
use mo_text::{TextError, backend::TextBackend, manifest::*};
use std::cell::Cell;
use support::*;

fn isolate(name: &str) -> bool {
    if std::env::var("MO_TEXT_PAGE_CHILD").ok().as_deref() == Some(name) {
        return false;
    }
    let result = std::process::Command::new(std::env::current_exe().unwrap())
        .args(["--exact", name, "--test-threads=1"])
        .env("MO_TEXT_PAGE_CHILD", name)
        .output()
        .unwrap();
    assert!(
        result.status.success(),
        "{}\n{}",
        String::from_utf8_lossy(&result.stdout),
        String::from_utf8_lossy(&result.stderr)
    );
    true
}
struct ProbeImage {
    plan: SourceTextPagePlan,
    pixels: Vec<u8>,
}
fn image(b: &[u8]) -> ProbeImage {
    let i = read(b);
    let author = author();
    let manifest = PreparedManifest::load(
        &author.manifest,
        include_bytes!("../../../fixtures/fonts/owned.ttf"),
        ManifestLimits::default(),
        &|| false,
    )
    .unwrap();
    let plan = source_text_page::compile(
        &i,
        &request(&i),
        &manifest,
        &mut NativeShaper::default(),
        TextPageLimits::default(),
        &|| false,
    )
    .unwrap();
    let rendered = source_text_page::render(
        &i,
        &request(&i),
        &manifest,
        &mut NativeShaper::default(),
        &mut NativeRaster,
        TextPageLimits::default(),
        &|| false,
    )
    .unwrap();
    assert_eq!(rendered.info.text_frames as usize, plan.texts.len());
    let capacity = rendered.info.text_capacity.as_ref().unwrap();
    capacity
        .validate(rendered.info.text_frames, &|| false)
        .unwrap();
    assert_eq!(
        serde_json::to_value(capacity.page_ink.as_ref().unwrap()).unwrap(),
        serde_json::to_value(plan.texts.iter().map(|t| &t.page_ink).collect::<Vec<_>>()).unwrap(),
    );
    assert_eq!(
        rendered.info.text_work.component_calls,
        plan.text_work.component_calls
    );
    let result = ProbeImage {
        plan,
        pixels: rendered.pixels,
    };
    assert_eq!(result.pixels.len(), 400 * 300 * 4);
    if let Some(dir) = std::env::var_os("MO_TEXT_PAGE_EVIDENCE_DIR") {
        let dir = std::path::PathBuf::from(dir);
        assert!(dir.is_absolute());
        std::fs::create_dir_all(&dir).unwrap();
        let stem = i.source_sha256.as_str();
        std::fs::write(dir.join(format!("{stem}.pptx")), b).unwrap();
        std::fs::write(
            dir.join(format!("{stem}.json")),
            serde_json::to_vec_pretty(&result.plan).unwrap(),
        )
        .unwrap();
        std::fs::write(dir.join(format!("{stem}.rgba")), &result.pixels).unwrap();
    }
    result
}
#[test]
fn real_native_text_paints_keep_run_clusters_and_object_order() {
    if isolate("real_native_text_paints_keep_run_clusters_and_object_order") {
        return;
    }
    let a = shape(
        42,
        100000,
        100000,
        "",
        &(colored("A", "C02040") + &colored("A", "2070C0")),
    );
    let r = image(&fixture(&a));
    assert_eq!(r.plan.texts.len(), 1);
    assert_eq!(r.plan.text_sources.len(), 2);
    assert_eq!(r.plan.texts[0].clusters[0].runs, [0]);
    assert_eq!(r.plan.texts[0].clusters[1].runs, [1]);
    assert!(r.pixels.chunks_exact(4).any(|p| p == [192, 32, 64, 255]));
    assert!(r.pixels.chunks_exact(4).any(|p| p == [32, 112, 192, 255]));
    let front = shape(43, 220000, 200000, "", &colored("A", "208040"));
    let r = image(&fixture(&(a + &front)));
    let text0 = r.plan.text_sources[0].instance;
    let shape1 = r
        .plan
        .page
        .paint_sources
        .iter()
        .find(|p| p.binding == 2)
        .unwrap()
        .instance;
    let text1 = r.plan.text_sources.last().unwrap().instance;
    assert!(text0 < shape1 && shape1 < text1);
    assert_eq!(r.plan.text_work.component_calls, 6);
}
#[test]
fn source_rotations_flips_groups_and_transparent_runs_reach_pixels() {
    if isolate("source_rotations_flips_groups_and_transparent_runs_reach_pixels") {
        return;
    }
    for extra in [
        "rot=\"1800000\"",
        "flipH=\"1\"",
        "rot=\"-2700001\" flipV=\"1\"",
    ] {
        let r = image(&fixture(&shape(
            42,
            200000,
            200000,
            extra,
            &colored("AA", "804020"),
        )));
        assert_eq!(r.plan.text_sources.len(), 2);
        assert!(r.plan.page.downstream_coordinate_error_bound <= Fixed::from_raw(1 << 24));
    }
    let child = shape(
        42,
        100000,
        100000,
        "rot=\"900000\"",
        &colored("A", "5040C0"),
    );
    let group = format!(
        "<p:grpSp><p:nvGrpSpPr><p:cNvPr id=\"2\" name=\"Group\"/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr><p:grpSpPr><a:xfrm rot=\"600000\"><a:off x=\"150000\" y=\"100000\"/><a:ext cx=\"1100000\" cy=\"800000\"/><a:chOff x=\"0\" y=\"0\"/><a:chExt cx=\"1600000\" cy=\"1200000\"/></a:xfrm></p:grpSpPr>{child}</p:grpSp>"
    );
    image(&fixture(&group));
    let r = image(&fixture(&shape(
        42,
        100000,
        100000,
        "",
        &(colored("A", "102030") + "<a:r><a:rPr><a:noFill/></a:rPr><a:t>A</a:t></a:r>"),
    )));
    assert_eq!(r.plan.texts[0].frame.glyphs.len(), 2);
    assert_eq!(r.plan.text_sources.len(), 1);
}
#[test]
fn font_reference_and_placeholder_colors_are_native_expressions() {
    if isolate("font_reference_and_placeholder_colors_are_native_expressions") {
        return;
    }
    for runs in [run("A"),"<a:r><a:rPr><a:solidFill><a:schemeClr val=\"phClr\"><a:shade val=\"50000\"/></a:schemeClr></a:solidFill></a:rPr><a:t>A</a:t></a:r>".into()] {
        let s=shape(42,100000,100000,"",&runs).replace("</p:spPr>","</p:spPr><p:style><a:lnRef idx=\"0\"><a:srgbClr val=\"000000\"/></a:lnRef><a:fillRef idx=\"0\"><a:srgbClr val=\"000000\"/></a:fillRef><a:effectRef idx=\"0\"><a:srgbClr val=\"000000\"/></a:effectRef><a:fontRef idx=\"minor\"><a:srgbClr val=\"6080C0\"/></a:fontRef></p:style>")
            .replace("</a:ln></p:spPr>","</a:ln><a:effectLst/></p:spPr>");
        let r=image(&fixture(&s));assert_eq!(r.plan.text_sources.len(),1);
    }
}
#[derive(Default)]
struct TextCounter {
    native: NativeShaper,
    calls: usize,
}
impl TextBackend for TextCounter {
    fn shape_batch(&mut self, f: &[u8], q: &[u32]) -> Result<Vec<u32>, TextError> {
        self.calls += 1;
        self.native.shape_batch(f, q)
    }
    fn measure_batch(&mut self, f: &[u8], q: &[u32]) -> Result<Vec<u32>, TextError> {
        self.calls += 1;
        self.native.measure_batch(f, q)
    }
    fn outline_batch(&mut self, f: &[u8], q: &[u32]) -> Result<Vec<u32>, TextError> {
        self.calls += 1;
        self.native.outline_batch(f, q)
    }
    fn invalidate(&mut self) {
        self.native.invalidate();
    }
}
#[derive(Default)]
struct RasterCounter {
    calls: usize,
}
impl RasterBackend for RasterCounter {
    fn raster(&mut self, _: &[u32]) -> Result<BackendReply, RasterError> {
        self.calls += 1;
        Err(RasterError::Host("unexpected raster call"))
    }
    fn invalidate(&mut self) {}
}
#[test]
fn whole_page_preflight_rejects_late_unsupported_text_before_components() {
    if isolate("whole_page_preflight_rejects_late_unsupported_text_before_components") {
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
    for bad in [
        colored("A", "123456").replace("<a:rPr>", "<a:rPr u=\"wavy\">"),
        run("A"),
    ] {
        let b = fixture(
            &(shape(42, 0, 0, "", &colored("A", "FF0000")) + &shape(43, 100000, 100000, "", &bad)),
        );
        // The writer now emits effective global defaults. This negative case
        // needs an actually missing paint, not a valid inherited text color.
        let b = without_default_text_style(&b);
        let i = read(&b);
        let before = i.clone();
        let mut text = TextCounter::default();
        let mut raster = RasterCounter::default();
        assert!(
            source_text_page::render(
                &i,
                &request(&i),
                &manifest,
                &mut text,
                &mut raster,
                TextPageLimits::default(),
                &|| false
            )
            .is_err()
        );
        assert_eq!((text.calls, raster.calls), (0, 0));
        assert_eq!(i, before);
    }
    let b = fixture(
        &(shape(42, 0, 0, "", &colored("A", "FF0000"))
            + &shape(43, 100000, 100000, "", &colored("A", "0000FF"))),
    );
    let i = read(&b);
    for limits in [
        TextPageLimits {
            max_frames: 1,
            ..Default::default()
        },
        TextPageLimits {
            max_paragraphs: 1,
            ..Default::default()
        },
        TextPageLimits {
            max_runs: 1,
            ..Default::default()
        },
        TextPageLimits {
            max_prepared_plan_bytes: 0,
            ..Default::default()
        },
    ] {
        let mut text = TextCounter::default();
        let mut raster = RasterCounter::default();
        assert!(
            source_text_page::render(
                &i,
                &request(&i),
                &manifest,
                &mut text,
                &mut raster,
                limits,
                &|| false
            )
            .is_err()
        );
        assert_eq!((text.calls, raster.calls), (0, 0));
    }
}
#[test]
fn combining_cluster_preserves_all_origins_and_rejects_conflicting_paint() {
    if isolate("combining_cluster_preserves_all_origins_and_rejects_conflicting_paint") {
        return;
    }
    let r = image(&fixture(&shape(
        42,
        100000,
        100000,
        "",
        &(colored("A", "FF0000") + &colored("\u{301}", "FF0000")),
    )));
    assert_eq!(r.plan.texts[0].clusters[0].runs, [0, 1]);
    let b = fixture(&shape(
        42,
        100000,
        100000,
        "",
        &(colored("A", "FF0000") + &colored("\u{301}", "0000FF")),
    ));
    let i = read(&b);
    let author = author();
    let manifest = PreparedManifest::load(
        &author.manifest,
        include_bytes!("../../../fixtures/fonts/owned.ttf"),
        ManifestLimits::default(),
        &|| false,
    )
    .unwrap();
    let mut raster = RasterCounter::default();
    assert!(matches!(
        source_text_page::render(
            &i,
            &request(&i),
            &manifest,
            &mut NativeShaper::default(),
            &mut raster,
            TextPageLimits::default(),
            &|| false
        ),
        Err(SourcePageError::AtObject { location, error })
            if location.object == Some(42) && matches!(*error,
                SourcePageError::GlyphPaintConflict { start: 0, end: 2, .. })
    ));
    assert_eq!(raster.calls, 0);
}
#[test]
fn page_work_budget_is_shared_and_no_partial_pixels_escape() {
    if isolate("page_work_budget_is_shared_and_no_partial_pixels_escape") {
        return;
    }
    let b = fixture(
        &(shape(42, 0, 0, "", &colored("A", "FF0000"))
            + &shape(43, 100000, 100000, "", &colored("A", "0000FF"))),
    );
    let i = read(&b);
    let author = author();
    let manifest = PreparedManifest::load(
        &author.manifest,
        include_bytes!("../../../fixtures/fonts/owned.ttf"),
        ManifestLimits::default(),
        &|| false,
    )
    .unwrap();
    let mut limits = TextPageLimits::default();
    limits.work.max_component_calls = 5;
    let mut text = TextCounter::default();
    let mut raster = RasterCounter::default();
    assert!(
        source_text_page::render(
            &i,
            &request(&i),
            &manifest,
            &mut text,
            &mut raster,
            limits,
            &|| false
        )
        .is_err()
    );
    assert_eq!((text.calls, raster.calls), (5, 0));
}
#[test]
fn page_cancellation_and_legacy_entry_remain_explicit() {
    if isolate("page_cancellation_and_legacy_entry_remain_explicit") {
        return;
    }
    let b = fixture(&shape(42, 0, 0, "", &colored("A", "FF0000")));
    let i = read(&b);
    assert!(
        matches!(mo_presentation_compile::source_page::compile(&i,&request(&i),&||false),Err(SourcePageError::Mapping{issue,..}) if matches!(*issue,SourcePageIssue::Text{..}))
    );
    let author = author();
    let manifest = PreparedManifest::load(
        &author.manifest,
        include_bytes!("../../../fixtures/fonts/owned.ttf"),
        ManifestLimits::default(),
        &|| false,
    )
    .unwrap();
    let calls = Cell::new(0);
    let mut text = TextCounter::default();
    let mut raster = RasterCounter::default();
    assert!(
        source_text_page::render(
            &i,
            &request(&i),
            &manifest,
            &mut text,
            &mut raster,
            TextPageLimits::default(),
            &|| {
                calls.set(calls.get() + 1);
                calls.get() > 50
            }
        )
        .is_err()
    );
    assert_eq!((text.calls, raster.calls), (0, 0));
    // A raster-host failure after complete text preparation returns no image.
    assert!(
        source_text_page::render(
            &i,
            &request(&i),
            &manifest,
            &mut text,
            &mut raster,
            TextPageLimits::default(),
            &|| false
        )
        .is_err()
    );
    assert_eq!((text.calls, raster.calls), (3, 1));
}

#[test]
fn page_ink_envelopes_cover_actual_glyph_pixels_after_placement_and_exclude_transparency() {
    if isolate(
        "page_ink_envelopes_cover_actual_glyph_pixels_after_placement_and_exclude_transparency",
    ) {
        return;
    }
    for extra in [
        "",
        "rot=\"1800000\"",
        "flipH=\"1\"",
        "rot=\"-2700001\" flipV=\"1\"",
    ] {
        let child = shape(42, 200000, 200000, extra, &colored("AA", "804020"));
        let group = format!(
            "<p:grpSp><p:nvGrpSpPr><p:cNvPr id=\"2\" name=\"Group\"/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr><p:grpSpPr><a:xfrm rot=\"600000\"><a:off x=\"150000\" y=\"100000\"/><a:ext cx=\"1100000\" cy=\"800000\"/><a:chOff x=\"0\" y=\"0\"/><a:chExt cx=\"1600000\" cy=\"1200000\"/></a:xfrm></p:grpSpPr>{child}</p:grpSp>"
        );
        for content in [&child, &group] {
            let result = image(&fixture(content));
            let ink = &result.plan.texts[0].page_ink;
            assert_eq!(ink.object.native_id, 42);
            assert!(!ink.clipping_applied);
            let bounds = ink.bounds.unwrap();
            let mut count = 0;
            for (i, pixel) in result.pixels.chunks_exact(4).enumerate() {
                if pixel != [128, 64, 32, 255] {
                    continue;
                }
                // Independent viewport: fixture maps 4000 EMU to one pixel.
                let x = Fixed::emu(mo_common::Emu::new((i % 400) as i64 * 4000 + 2000));
                let y = Fixed::emu(mo_common::Emu::new((i / 400) as i64 * 4000 + 2000));
                assert!(bounds.min.x <= x && x <= bounds.max.x);
                assert!(bounds.min.y <= y && y <= bounds.max.y);
                count += 1;
            }
            assert!(count > 100);
        }
    }
    for runs in [
        "<a:r><a:rPr><a:noFill/></a:rPr><a:t>A</a:t></a:r>".to_owned(),
        colored("A", "804020").replace(
            "<a:srgbClr val=\"804020\"/>",
            "<a:srgbClr val=\"804020\"><a:alpha val=\"0\"/></a:srgbClr>",
        ),
    ] {
        let result = image(&fixture(&shape(42, 100000, 100000, "", &runs)));
        assert!(result.plan.texts[0].page_ink.bounds.is_none());
        assert!(result.plan.texts[0].painted_ink.is_none());
        assert_eq!(result.plan.texts[0].frame.glyphs.len(), 1);
    }
    let clipped = shape(42, 100000, 100000, "", &colored("AA", "804020"))
        .replace("<a:bodyPr ", "<a:bodyPr horzOverflow=\"clip\" ");
    let result = image(&fixture(&clipped));
    assert!(result.plan.texts[0].page_ink.clipping_applied);
    assert!(result.plan.texts[0].page_ink.bounds.is_some());
}
