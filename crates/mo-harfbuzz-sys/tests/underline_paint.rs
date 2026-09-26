#[allow(dead_code)]
#[path = "../../../tools/test-support/source_text_page.rs"]
mod support;
use mo_harfbuzz_sys::NativeShaper;
use mo_pptx::source::{
    color::ColorContext,
    text::paint::{TextPaint, UnderlinePaint},
};
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
fn fill(color: &str) -> String {
    format!("<a:solidFill><a:srgbClr val=\"{color}\"/></a:solidFill>")
}
fn painted(text: &str, glyph: &str, underline: &str) -> String {
    format!(
        "<a:r><a:rPr u=\"sng\">{glyph}<a:uFill>{underline}</a:uFill></a:rPr><a:t>{text}</a:t></a:r>"
    )
}
fn isolate(name: &str) -> bool {
    if std::env::var("MO_UNDERLINE_CHILD").ok().as_deref() == Some(name) {
        return false;
    }
    let r = std::process::Command::new(std::env::current_exe().unwrap())
        .args(["--exact", name, "--test-threads=1"])
        .env("MO_UNDERLINE_CHILD", name)
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
    if let Some(dir) = std::env::var_os("MO_UNDERLINE_EVIDENCE_DIR") {
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
fn independent_underline_color_and_visibility_reach_real_pixels() {
    if isolate("independent_underline_color_and_visibility_reach_real_pixels") {
        return;
    }
    for (name, glyph, underline, decos, glyphs) in [
        ("separate", fill("2070C0"), fill("C02040"), 1, 2),
        ("hidden-text", "<a:noFill/>".into(), fill("208040"), 1, 0),
        ("hidden-line", fill("2070C0"), "<a:noFill/>".into(), 0, 2),
        (
            "both-hidden",
            "<a:noFill/>".into(),
            "<a:noFill/>".into(),
            0,
            0,
        ),
    ] {
        let p = image(
            name,
            &fixture(&shape(
                42,
                100000,
                100000,
                "",
                &painted("A A", &glyph, &underline),
            )),
            Default::default(),
        );
        assert_eq!(p.decoration_sources.len(), decos);
        assert_eq!(p.text_sources.len(), glyphs);
        assert_eq!(p.text_work.component_calls, if decos == 0 { 3 } else { 4 });
        assert!(matches!(
            p.texts[0].paints[0][0].underline,
            Some(UnderlinePaint::Independent { .. })
        ));
    }
    let runs = painted("A A", &fill("2070C0"), "<a:noFill/>")
        .replace("u=\"sng\"", "u=\"sng\" strike=\"sngStrike\"");
    let p = image(
        "strike-visible",
        &fixture(&shape(42, 100000, 100000, "", &runs)),
        Default::default(),
    );
    assert_eq!(p.texts[0].decorations.len(), 1);
    assert_eq!(p.texts[0].decorations[0].kind, DecorationKind::Strike);
    let runs = painted("A A", &fill("2070C0"), &fill("C02040"));
    image(
        "rotated",
        &fixture(&shape(
            42,
            100000,
            100000,
            "rot=\"1800000\" flipH=\"1\"",
            &runs,
        )),
        Default::default(),
    );
}
#[test]
fn independent_fill_inheritance_clusters_and_theme_context_remain_source_bound() {
    if isolate("independent_fill_inheritance_clusters_and_theme_context_remain_source_bound") {
        return;
    }
    let runs = format!(
        "{}{}",
        colored("A", "2070C0"),
        colored("A", "208040").replace("</a:rPr>", "<a:uFillTx/></a:rPr>")
    );
    let mut s = shape(42, 100000, 100000, "", &runs)
        .replace("<a:defRPr sz=\"3000\"", "<a:defRPr u=\"sng\" sz=\"3000\"");
    s = s.replace(
        "<a:latin",
        &format!("<a:uFill>{}</a:uFill><a:latin", fill("C02040")),
    );
    let p = image("inherited", &fixture(&s), Default::default());
    assert_eq!(p.texts[0].decorations.len(), 2);
    assert_eq!(p.texts[0].decorations[0].rgba, [192, 32, 64, 255]);
    assert_eq!(p.texts[0].decorations[1].rgba, [32, 128, 64, 255]);
    let runs = painted("A", &fill("2070C0"), &fill("C02040"))
        + &painted("\u{301}", &fill("2070C0"), &fill("C02040"));
    let p = image(
        "same-cluster",
        &fixture(&shape(42, 100000, 100000, "", &runs)),
        Default::default(),
    );
    assert_eq!(p.texts[0].clusters[0].runs, [0, 1]);
    assert_eq!(p.texts[0].decorations.len(), 1);
    let runs = painted("A", &fill("2070C0"), &fill("C02040"))
        + &painted("A", &fill("2070C0"), &fill("208040"));
    let p = image(
        "split-colors",
        &fixture(&shape(42, 100000, 100000, "", &runs)),
        Default::default(),
    );
    assert_eq!(p.texts[0].decorations.len(), 2);
    let runs = painted(
        "A",
        &fill("2070C0"),
        "<a:solidFill><a:schemeClr val=\"phClr\"><a:shade val=\"50000\"/></a:schemeClr></a:solidFill>",
    );
    let s=shape(42,100000,100000,"",&runs).replace("</p:spPr>","</p:spPr><p:style><a:lnRef idx=\"0\"><a:srgbClr val=\"000000\"/></a:lnRef><a:fillRef idx=\"0\"><a:srgbClr val=\"000000\"/></a:fillRef><a:effectRef idx=\"0\"><a:srgbClr val=\"000000\"/></a:effectRef><a:fontRef idx=\"minor\"><a:srgbClr val=\"C08040\"/></a:fontRef></p:style>");
    let s = s.replace("</a:ln></p:spPr>", "</a:ln><a:effectLst/></p:spPr>");
    let p = image("placeholder", &fixture(&s), Default::default());
    assert!(matches!(
        p.texts[0].paints[0][0].underline.as_ref(),
        Some(UnderlinePaint::Independent {
            fill,
            ..
        }) if matches!(fill.as_ref(), TextPaint::Solid { placeholder: Some(_), .. })
    ));
    assert_ne!(p.texts[0].decorations[0].rgba, [32, 112, 192, 255]);
    let runs = painted(
        "A",
        &fill("2070C0"),
        "<a:solidFill><a:sysClr val=\"window\"/></a:solidFill>",
    );
    let context =
        serde_json::from_str(r#"{"systemColors":{"window":[32,128,64]},"placeholder":null}"#)
            .unwrap();
    image(
        "system",
        &fixture(&shape(42, 100000, 100000, "", &runs)),
        context,
    );
    // A dormant underline cannot demand context or override the glyph fill.
    let runs = runs.replace("u=\"sng\"", "u=\"none\"");
    let p = image(
        "inactive",
        &fixture(&shape(42, 100000, 100000, "", &runs)),
        Default::default(),
    );
    assert!(p.decoration_sources.is_empty());
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
fn independent_underline_errors_preflight_and_clusters_fail_without_partial_images() {
    if isolate("independent_underline_errors_preflight_and_clusters_fail_without_partial_images") {
        return;
    }
    let f = fonts();
    let manifest =
        PreparedManifest::load(&f, FONT_BYTES, ManifestLimits::default(), &|| false).unwrap();
    for (name, underline) in [
        (
            "missing-system",
            "<a:solidFill><a:sysClr val=\"window\"/></a:solidFill>",
        ),
        (
            "missing-placeholder",
            "<a:solidFill><a:schemeClr val=\"phClr\"/></a:solidFill>",
        ),
        ("pattern", "<a:pattFill prst=\"pct5\"/>"),
    ] {
        let runs = colored("A", "2070C0") + &painted("A", &fill("2070C0"), underline);
        let b = fixture(&shape(42, 100000, 100000, "", &runs));
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
        assert!(
            matches!(e,SourcePageError::AtObject{error,..} if matches!(*error,SourcePageError::TextPaint(_)))
        );
        save(name, &b, &q, None, None);
    }
    let runs = painted("A", &fill("2070C0"), &fill("C02040"))
        + &painted("\u{301}", &fill("2070C0"), &fill("208040"));
    let b = fixture(&shape(42, 100000, 100000, "", &runs));
    let i = read(&b);
    let q = request(&i);
    let e = source_text_page::render(
        &i,
        &q,
        &manifest,
        &mut NativeShaper::default(),
        &mut NoRaster,
        TextPageLimits::default(),
        &|| false,
    )
    .err()
    .unwrap();
    assert!(
        matches!(e,SourcePageError::AtObject{error,..} if matches!(*error,SourcePageError::GlyphPaintConflict{start:0,end:2,..}))
    );
    save("conflicting-cluster", &b, &q, None, None);
}
