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
    if std::env::var("MO_SPACING_CHILD").ok().as_deref() == Some(name) {
        return false;
    }
    let r = std::process::Command::new(std::env::current_exe().unwrap())
        .args(["--exact", name, "--test-threads=1"])
        .env("MO_SPACING_CHILD", name)
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
    if let Some(dir) = std::env::var_os("MO_SPACING_EVIDENCE_DIR") {
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

fn paragraph(properties: &str, runs: &str) -> String {
    let runs = runs.replace("<a:br/>", "<a:br><a:rPr><a:noFill/></a:rPr></a:br>");
    format!("<a:p><a:pPr>{properties}</a:pPr>{runs}</a:p>")
}
fn spaced(paragraphs: &str, body: &str) -> Vec<u8> {
    fixture(
        &shape(42, 100000, 100000, "", "")
            .replace("<a:p></a:p>", paragraphs)
            .replace("<a:bodyPr ", &format!("<a:bodyPr {body} ")),
    )
}
fn percentage(slot: &str, value: &str) -> String {
    format!("<a:{slot}><a:spcPct val=\"{value}\"/></a:{slot}>")
}
#[test]
fn percentage_line_heights_follow_actual_text_sizes_and_allow_explicit_zero() {
    if isolate("percentage_line_heights_follow_actual_text_sizes_and_allow_explicit_zero") {
        return;
    }
    let lines = text("A", "sz=\"1000\"") + "<a:br/>" + &text("A", "sz=\"2000\"");
    for (name, value, expected) in [
        ("line-hundred", "100000", [127000, 254000]),
        ("line-one-half", "150%", [190500, 381000]),
        ("line-zero", "0%", [0, 0]),
    ] {
        let p = image(
            name,
            &spaced(&paragraph(&percentage("lnSpc", value), &lines), ""),
            Default::default(),
        );
        let l = &p.texts[0].frame.paragraphs[0]
            .computed
            .geometry
            .precise
            .as_ref()
            .unwrap()
            .lines;
        assert_eq!(
            [
                l[0].height.wire().unwrap().get(),
                l[1].height.wire().unwrap().get()
            ],
            expected
        );
    }
    let p = image(
        "point-zero",
        &spaced(
            &paragraph("<a:lnSpc><a:spcPts val=\"0\"/></a:lnSpc>", &lines),
            "",
        ),
        Default::default(),
    );
    assert_eq!(p.texts[0].frame.content_height.raw(), 0);
    let fractional = text("A", "sz=\"1001\"") + "<a:br/>" + &text("A", "sz=\"1703\"");
    image(
        "line-fractional",
        &spaced(
            &paragraph(&percentage("lnSpc", "125.123456789%"), &fractional),
            "",
        ),
        Default::default(),
    );
    let mixed = text("A", "sz=\"1000\"") + &text("A", "sz=\"2400\"");
    let p = image(
        "mixed-line",
        &spaced(&paragraph(&percentage("lnSpc", "150%"), &mixed), ""),
        Default::default(),
    );
    assert_eq!(
        p.texts[0].frame.content_height.wire().unwrap().get(),
        457200
    );
    let p = image(
        "empty-percent",
        &spaced(
            &paragraph(&percentage("lnSpc", "200%"), "<a:endParaRPr sz=\"1800\"/>"),
            "",
        ),
        Default::default(),
    );
    assert_eq!(
        p.texts[0].frame.content_height.wire().unwrap().get(),
        457200
    );
}
#[test]
fn paragraph_percentages_use_boundary_lines_inheritance_and_outer_edge_policy() {
    if isolate("paragraph_percentages_use_boundary_lines_inheritance_and_outer_edge_policy") {
        return;
    }
    let rules = percentage("lnSpc", "100%")
        + &percentage("spcBef", "25.123456789%")
        + &percentage("spcAft", "50%");
    let runs = text("A", "sz=\"1000\"") + "<a:br/>" + &text("A", "sz=\"2000\"");
    let paragraphs = paragraph(&rules, &runs) + &paragraph(&rules, &text("A", "sz=\"1500\""));
    for (name, body) in [
        ("outer-suppressed", ""),
        ("outer-included", "spcFirstLastPara=\"1\""),
        ("center", "spcFirstLastPara=\"1\" anchor=\"ctr\""),
        ("bottom", "spcFirstLastPara=\"1\" anchor=\"b\""),
    ] {
        let p = image(name, &spaced(&paragraphs, body), Default::default());
        let first = &p.texts[0].frame.paragraphs[0];
        assert_eq!(first.applied_after.wire().unwrap().get(), 127000);
        assert_eq!(first.applied_before.raw() == 0, name == "outer-suppressed");
    }
    let p = paragraph("", &text("A", "sz=\"1000\""))
        + &paragraph(
            "<a:lnSpc><a:spcPts val=\"1200\"/></a:lnSpc>",
            &text("A", "sz=\"2000\""),
        );
    let s = shape(42, 100000, 100000, "", "")
        .replace("<a:p></a:p>", &p)
        .replace("<a:lvl1pPr>", &format!("<a:lvl1pPr>{rules}"));
    image("inherited", &fixture(&s), Default::default());
    let decorated = paragraph(
        &rules,
        &(text("A", "sz=\"1703\" baseline=\"30%\" u=\"sng\"")
            + "<a:br/>"
            + &text("A", "sz=\"1001\" strike=\"sngStrike\"")),
    );
    image(
        "decorated",
        &spaced(&decorated, "spcFirstLastPara=\"1\""),
        Default::default(),
    );
    let trailing = paragraph(
        &percentage("lnSpc", "150%"),
        &(text("A", "sz=\"1000\"") + "<a:br/><a:endParaRPr sz=\"1200\"/>"),
    );
    let p = image("trailing-empty", &spaced(&trailing, ""), Default::default());
    assert_eq!(
        p.texts[0].frame.content_height.wire().unwrap().get(),
        419100
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
fn malformed_or_excessive_spacing_is_source_bound_and_precedes_components() {
    if isolate("malformed_or_excessive_spacing_is_source_bound_and_precedes_components") {
        return;
    }
    let f = fonts();
    let manifest =
        PreparedManifest::load(&f, FONT_BYTES, ManifestLimits::default(), &|| false).unwrap();
    for (name, properties) in [
        (
            "lexical",
            percentage("lnSpc", &format!("0.{}%", "0".repeat(260))),
        ),
        (
            "unknown",
            "<a:lnSpc><a:spcPct val=\"150%\" unknown=\"x\"/></a:lnSpc>".into(),
        ),
        (
            "unknown-wrapper",
            "<a:lnSpc unknown=\"x\"><a:spcPct val=\"150%\"/></a:lnSpc>".into(),
        ),
    ] {
        let b = spaced(&paragraph(&properties, &text("A", "")), "");
        let i = read(&b);
        let q = request(&i);
        let e = source_text_page::render(
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
        assert!(matches!(e, SourcePageError::AtObject { .. }));
        save(name, &b, &q, None, None);
    }
}
