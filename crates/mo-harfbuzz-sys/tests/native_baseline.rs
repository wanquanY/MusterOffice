#[allow(dead_code)]
#[path = "../../../tools/test-support/source_text_page.rs"]
mod support;
use mo_harfbuzz_sys::NativeShaper;
use mo_pptx::source::color::ColorContext;
use mo_presentation_compile::{
    source_page::*,
    source_text_page::{self, *},
};
use mo_raster::{BackendReply, RasterBackend, RasterError};
use mo_skia_sys::NativeRaster;
use mo_text::{TextError, backend::TextBackend, manifest::*};
use support::*;
const FONT_BYTES: &[u8] = include_bytes!("../../../fixtures/fonts/owned-decorations.ttf");
fn fonts() -> FontManifest {
    serde_json::from_str(include_str!(
        "../../../fixtures/fonts/decoration-manifest.json"
    ))
    .unwrap()
}
fn text(value: &str, attrs: &str) -> String {
    colored(value, "2070C0").replace("<a:rPr>", &format!("<a:rPr {attrs}>"))
}
fn isolate(name: &str) -> bool {
    if std::env::var("MO_BASELINE_CHILD").ok().as_deref() == Some(name) {
        return false;
    }
    let r = std::process::Command::new(std::env::current_exe().unwrap())
        .args(["--exact", name, "--test-threads=1"])
        .env("MO_BASELINE_CHILD", name)
        .output()
        .unwrap();
    assert!(
        r.status.success(),
        "{}\n{}",
        String::from_utf8_lossy(&r.stdout),
        String::from_utf8_lossy(&r.stderr)
    );
    true
}
fn save(
    name: &str,
    source: &[u8],
    q: &SourcePageRequest,
    plan: Option<&SourceTextPagePlan>,
    pixels: Option<&[u8]>,
) {
    if let Some(dir) = std::env::var_os("MO_BASELINE_EVIDENCE_DIR") {
        let dir = std::path::PathBuf::from(dir);
        assert!(dir.is_absolute());
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join(format!("{name}.pptx")), source).unwrap();
        std::fs::write(
            dir.join(format!("{name}.page.json")),
            serde_json::to_vec(q).unwrap(),
        )
        .unwrap();
        if let Some(plan) = plan {
            std::fs::write(
                dir.join(format!("{name}.plan.json")),
                serde_json::to_vec_pretty(plan).unwrap(),
            )
            .unwrap();
        }
        if let Some(pixels) = pixels {
            std::fs::write(dir.join(format!("{name}.rgba")), pixels).unwrap();
        }
    }
}
fn image(name: &str, source: &[u8], context: ColorContext) -> SourceTextPagePlan {
    let i = read(source);
    let f = fonts();
    let manifest =
        PreparedManifest::load(&f, FONT_BYTES, ManifestLimits::default(), &|| false).unwrap();
    let mut q = request(&i);
    q.color_context = context;
    let plan = source_text_page::compile(
        &i,
        &q,
        &manifest,
        &mut NativeShaper::default(),
        TextPageLimits::default(),
        &|| false,
    )
    .unwrap();
    let r = source_text_page::render(
        &i,
        &q,
        &manifest,
        &mut NativeShaper::default(),
        &mut NativeRaster,
        TextPageLimits::default(),
        &|| false,
    )
    .unwrap();
    assert_eq!(r.pixels.len(), 400 * 300 * 4);
    save(name, source, &q, Some(&plan), Some(&r.pixels));
    plan
}

#[test]
fn native_baselines_shift_real_glyphs_line_extents_and_decorations() {
    if isolate("native_baselines_shift_real_glyphs_line_extents_and_decorations") {
        return;
    }
    let normal = text("A", "");
    let mixed = normal.clone() + &text("A", "baseline=\"30000\"") + &text("A", "baseline=\"-25%\"");
    for (name, runs, extra) in [
        ("mixed", mixed.clone(), ""),
        (
            "fractional",
            normal.clone() + &text("A", "sz=\"1733\" baseline=\"12.345678901%\""),
            "",
        ),
        (
            "small-thousandth",
            normal.clone() + &text("A", "baseline=\"1\""),
            "",
        ),
        (
            "decorated",
            mixed.replace("<a:rPr ", "<a:rPr u=\"sng\" strike=\"sngStrike\" "),
            "",
        ),
        (
            "rotated",
            normal.clone() + &text("A", "baseline=\"-25000\""),
            "rot=\"1800000\" flipH=\"1\"",
        ),
        (
            "wrapped",
            normal.clone() + &text("A A A A A", "sz=\"2000\" baseline=\"30%\""),
            "",
        ),
        (
            "end-style",
            normal.clone() + "<a:endParaRPr baseline=\"-25%\"/>",
            "",
        ),
        ("empty", "<a:endParaRPr baseline=\"30%\"/>".into(), ""),
    ] {
        let p = image(
            name,
            &fixture(&shape(42, 100000, 100000, extra, &runs)),
            Default::default(),
        );
        assert_eq!(p.texts.len(), 1);
        if name == "mixed" {
            let g = &p.texts[0].frame.glyphs;
            assert_eq!(g.len(), 3);
            // The independent oracle below verifies coordinates and metrics;
            // source declarations and requested size are retained in this plan.
            assert_eq!(p.texts[0].frame.inputs[0].geometry.len(), 3);
        }
        if name == "decorated" {
            assert_eq!(p.texts[0].decorations.len(), 6);
        }
    }
}
#[test]
fn inherited_and_equivalent_baselines_keep_original_runs_and_precision() {
    if isolate("inherited_and_equivalent_baselines_keep_original_runs_and_precision") {
        return;
    }
    let runs = text("A", "") + &text("A", "baseline=\"0\"");
    let s = shape(42, 100000, 100000, "", &runs)
        .replace("<a:defRPr sz=", "<a:defRPr baseline=\"30%\" sz=");
    let p = image("inherited", &fixture(&s), Default::default());
    assert_eq!(p.texts[0].frame.inputs[0].geometry.len(), 2);
    let runs = text("A", "baseline=\"1000\"") + &text("\u{301}", "baseline=\"1%\"");
    let p = image(
        "equal-lexeme",
        &fixture(&shape(42, 100000, 100000, "", &runs)),
        Default::default(),
    );
    assert_eq!(p.texts[0].clusters[0].runs, [0, 1]);
    let runs = text("A", "") + &text("\u{301}", "baseline=\"0.0000000000000000000001%\"");
    let p = image(
        "tiny-merged",
        &fixture(&shape(42, 100000, 100000, "", &runs)),
        Default::default(),
    );
    assert_eq!(p.texts[0].frame.inputs[0].geometry.len(), 1);
    assert_eq!(p.texts[0].clusters[0].runs, [0, 1]);
    assert_eq!(
        p.texts[0].frame.inputs[0].baseline_conversion_error.raw(),
        1
    );
}
struct NoText;
impl TextBackend for NoText {
    fn shape_batch(&mut self, _: &[u8], _: &[u32]) -> Result<Vec<u32>, TextError> {
        panic!("preflight failure")
    }
    fn measure_batch(&mut self, _: &[u8], _: &[u32]) -> Result<Vec<u32>, TextError> {
        panic!("preflight failure")
    }
    fn outline_batch(&mut self, _: &[u8], _: &[u32]) -> Result<Vec<u32>, TextError> {
        panic!("preflight failure")
    }
    fn invalidate(&mut self) {
        panic!("preflight must not invalidate component")
    }
}
struct NoRaster;
impl RasterBackend for NoRaster {
    fn raster(&mut self, _: &[u32]) -> Result<BackendReply, RasterError> {
        panic!("failed page cannot render")
    }
    fn invalidate(&mut self) {}
}

#[test]
fn baseline_range_precision_and_cluster_failures_precede_components() {
    if isolate("baseline_range_precision_and_cluster_failures_precede_components") {
        return;
    }
    let f = fonts();
    let manifest =
        PreparedManifest::load(&f, FONT_BYTES, ManifestLimits::default(), &|| false).unwrap();
    let cases = [
        (
            "range",
            text("A", &format!("baseline=\"{}%\"", "9".repeat(70))),
        ),
        (
            "lexical",
            text("A", &format!("baseline=\"0.{}%\"", "0".repeat(260))),
        ),
        (
            "cluster-conflict",
            text("A", "") + &text("\u{301}", "baseline=\"30000\""),
        ),
    ];
    for (name, runs) in cases {
        let b = fixture(&shape(42, 100000, 100000, "", &runs));
        let i = read(&b);
        let q = request(&i);
        let error = source_text_page::render(
            &i,
            &q,
            &manifest,
            &mut NoText,
            &mut NoRaster,
            TextPageLimits::default(),
            &|| false,
        )
        .err()
        .unwrap();
        assert!(matches!(error, SourcePageError::AtObject { .. }));
        save(name, &b, &q, None, None);
    }
    let b = fixture(&shape(
        42,
        100000,
        100000,
        "",
        &text("A", "baseline=\"30000\""),
    ));
    let i = read(&b);
    assert!(
        source_text_page::render(
            &i,
            &request(&i),
            &manifest,
            &mut NoText,
            &mut NoRaster,
            TextPageLimits::default(),
            &|| true
        )
        .is_err()
    );
}
