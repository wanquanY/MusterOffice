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
const FONT_BYTES: &[u8] = include_bytes!("../../../fixtures/fonts/owned-tracking.ttf");
fn fonts() -> FontManifest {
    serde_json::from_str(include_str!(
        "../../../fixtures/fonts/tracking-manifest.json"
    ))
    .unwrap()
}
fn text(value: &str, attrs: &str) -> String {
    colored(value, "2070C0").replace("<a:rPr>", &format!("<a:rPr {attrs}>"))
}
fn isolate(name: &str) -> bool {
    if std::env::var("MO_TRACKING_CHILD").ok().as_deref() == Some(name) {
        return false;
    }
    let r = std::process::Command::new(std::env::current_exe().unwrap())
        .args(["--exact", name, "--test-threads=1"])
        .env("MO_TRACKING_CHILD", name)
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
    if let Some(dir) = std::env::var_os("MO_TRACKING_EVIDENCE_DIR") {
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
fn page(runs: &str) -> Vec<u8> {
    fixture(&shape(42, 100000, 100000, "", runs).replace(
        "</a:defRPr>",
        &format!("<a:cs typeface=\"{FONT}\"/></a:defRPr>"),
    ))
}
#[test]
fn native_tracking_reaches_positions_clusters_bidi_and_decorations() {
    if isolate("native_tracking_reaches_positions_clusters_bidi_and_decorations") {
        return;
    }
    for (name, value) in [
        ("positive", "200"),
        ("negative", "-200"),
        ("fractional", "0.123456789pt"),
        ("zero", "0pt"),
    ] {
        let p = image(
            name,
            &page(&text("AAA", &format!("spc=\"{value}\""))),
            Default::default(),
        );
        assert_eq!(
            p.texts[0].frame.glyphs.len(),
            if name == "zero" { 2 } else { 3 }
        );
    }
    image(
        "combining",
        &page(&text("A\u{301}A", "spc=\"200\"")),
        Default::default(),
    );
    image("rtl", &page(&text("אב", "spc=\"200\"")), Default::default());
    image(
        "mixed",
        &page(&(text("A", "spc=\"100\"") + &text("A", "spc=\"-100\""))),
        Default::default(),
    );
    image(
        "decorated",
        &page(&text(
            "A\u{301}A",
            "spc=\"200\" baseline=\"30%\" u=\"sng\" strike=\"sngStrike\"",
        )),
        Default::default(),
    );
    let p = image(
        "equivalent",
        &page(&(text("A", "spc=\"100\"") + &text("A", "spc=\"1pt\""))),
        Default::default(),
    );
    assert_eq!(p.texts[0].frame.inputs[0].spans.len(), 1);
    image(
        "tiny",
        &page(&text("AA", "spc=\"0.000000000000000000000001pt\"")),
        Default::default(),
    );
    let required = image(
        "required-ligature",
        &page(&text("αα", "spc=\"200\"")),
        Default::default(),
    );
    assert_eq!(required.texts[0].frame.glyphs.len(), 1);
}
#[test]
fn tracking_participates_in_wrapping_alignment_and_source_inheritance() {
    if isolate("tracking_participates_in_wrapping_alignment_and_source_inheritance") {
        return;
    }
    let runs = text("AA AA", "spc=\"200\"");
    let narrow = shape(42, 100000, 100000, "", &runs).replace("cx=\"1000000\"", "cx=\"500000\"");
    let p = image("wrapped", &fixture(&narrow), Default::default());
    assert!(
        p.texts[0].frame.paragraphs[0]
            .computed
            .geometry
            .precise
            .as_ref()
            .unwrap()
            .lines
            .len()
            > 1
    );
    for (name, align) in [("center", "ctr"), ("right", "r")] {
        let s = shape(42, 100000, 100000, "", &text("AA", "spc=\"-100\""))
            .replace("<a:p>", &format!("<a:p><a:pPr algn=\"{align}\"/>"));
        image(name, &fixture(&s), Default::default());
    }
    let s = shape(
        42,
        100000,
        100000,
        "",
        &(text("A", "") + &text("A", "spc=\"-200\"")),
    )
    .replace("<a:defRPr ", "<a:defRPr spc=\"100\" ");
    image("inherited", &fixture(&s), Default::default());
}
struct NoText;
impl TextBackend for NoText {
    fn shape_batch(&mut self, _: &[u8], _: &[u32]) -> Result<Vec<u32>, TextError> {
        panic!("preflight")
    }
    fn measure_batch(&mut self, _: &[u8], _: &[u32]) -> Result<Vec<u32>, TextError> {
        panic!("preflight")
    }
    fn outline_batch(&mut self, _: &[u8], _: &[u32]) -> Result<Vec<u32>, TextError> {
        panic!("preflight")
    }
    fn invalidate(&mut self) {
        panic!("preflight")
    }
}
struct NoRaster;
impl RasterBackend for NoRaster {
    fn raster(&mut self, _: &[u32]) -> Result<BackendReply, RasterError> {
        panic!("preflight")
    }
    fn invalidate(&mut self) {
        panic!("preflight")
    }
}
#[test]
fn tracking_limits_and_grapheme_conflicts_fail_before_components() {
    let fonts = fonts();
    let m =
        PreparedManifest::load(&fonts, FONT_BYTES, ManifestLimits::default(), &|| false).unwrap();
    for (name, runs, expected) in [
        (
            "lexical",
            text(
                "AA",
                &format!("spc=\"{}pt\"", "0.".to_owned() + &"0".repeat(260)),
            ),
            "native tracking lexical bytes",
        ),
        (
            "range",
            text("AA", &format!("spc=\"{}pt\"", "9".repeat(70))),
            "native tracking range",
        ),
        (
            "cluster-conflict",
            text("A", "spc=\"100\"") + &text("\u{301}", "spc=\"200\""),
            "GraphemeStyleConflict",
        ),
    ] {
        let source = page(&runs);
        let i = read(&source);
        let q = request(&i);
        let error = source_text_page::render(
            &i,
            &q,
            &m,
            &mut NoText,
            &mut NoRaster,
            TextPageLimits::default(),
            &|| false,
        )
        .err()
        .expect("preflight rejection");
        assert!(error.to_string().contains(expected), "{error}");
        save(name, &source, &q, None, None);
    }
}
