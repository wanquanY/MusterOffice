#[allow(dead_code)]
#[path = "../../../tools/test-support/source_text_page.rs"]
mod support;
use mo_geometry::Fixed;
use mo_harfbuzz_sys::NativeShaper;
use mo_presentation_compile::{
    source_page::SourcePageError,
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
fn decorated(text: &str, attributes: &str) -> String {
    colored(text, "2070C0").replace("<a:rPr>", &format!("<a:rPr {attributes}>"))
}
fn isolate(name: &str) -> bool {
    if std::env::var("MO_DECORATION_CHILD").ok().as_deref() == Some(name) {
        return false;
    }
    let r = std::process::Command::new(std::env::current_exe().unwrap())
        .args(["--exact", name, "--test-threads=1"])
        .env("MO_DECORATION_CHILD", name)
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
fn image(name: &str, source: &[u8]) -> SourceTextPagePlan {
    let source = rewrite(source, SLIDE, |s| {
        s.replace(
            &format!("<a:latin typeface=\"{FONT}\"/>"),
            &format!("<a:latin typeface=\"{FONT}\"/><a:cs typeface=\"{FONT}\"/>"),
        )
    });
    let source = source.as_slice();
    let i = read(source);
    let f = fonts();
    let manifest =
        PreparedManifest::load(&f, FONT_BYTES, ManifestLimits::default(), &|| false).unwrap();
    let q = request(&i);
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
    assert_eq!(
        r.info.text_work.component_calls,
        plan.text_work.component_calls
    );
    assert_eq!(r.pixels.len(), 400 * 300 * 4);
    assert!(plan.page.downstream_coordinate_error_bound <= Fixed::from_raw(1 << 24));
    if let Some(dir) = std::env::var_os("MO_DECORATION_EVIDENCE_DIR") {
        let dir = std::path::PathBuf::from(dir);
        assert!(dir.is_absolute());
        std::fs::create_dir_all(&dir).unwrap();
        for (suffix, bytes) in [
            ("pptx", source.to_vec()),
            ("plan.json", serde_json::to_vec_pretty(&plan).unwrap()),
            ("rgba", r.pixels),
            ("page.json", serde_json::to_vec(&q).unwrap()),
        ] {
            std::fs::write(dir.join(format!("{name}.{suffix}")), bytes).unwrap();
        }
    }
    plan
}
#[test]
fn native_decorations_draw_advance_spans_and_batch_variable_metrics() {
    if isolate("native_decorations_draw_advance_spans_and_batch_variable_metrics") {
        return;
    }
    for (name, attributes, count) in [
        ("underline", "u=\"sng\"", 1),
        ("strike", "strike=\"sngStrike\"", 1),
        ("both", "u=\"sng\" strike=\"sngStrike\"", 2),
    ] {
        let p = image(
            name,
            &fixture(&shape(
                42,
                100000,
                100000,
                "",
                &decorated("A A", attributes),
            )),
        );
        let t = &p.texts[0];
        assert_eq!(t.decorations.len(), count);
        assert!(t.decorations.iter().all(|d| d.clusters.len() == 3));
        assert_eq!(p.text_work.component_calls, 4);
        assert_eq!(p.decoration_sources.len(), count);
        assert!(
            p.decoration_sources
                .iter()
                .all(|s| s.instance > p.text_sources.last().unwrap().instance)
        );
    }
    let runs = decorated("A", "u=\"sng\"") + &decorated("A", "u=\"sng\"");
    let p = image(
        "split-runs",
        &fixture(&shape(42, 100000, 100000, "", &runs)),
    );
    assert_eq!(p.texts[0].decorations.len(), 1);
    let runs = decorated("A", "u=\"sng\"") + &decorated("\u{301}", "u=\"sng\"");
    let p = image("combining", &fixture(&shape(42, 100000, 100000, "", &runs)));
    assert_eq!(p.texts[0].decorations.len(), 1);
    assert_eq!(p.texts[0].clusters[0].runs, [0, 1]);
    let runs = decorated("A", "u=\"sng\" strike=\"sngStrike\"")
        + &decorated("A", "b=\"1\" u=\"sng\" strike=\"sngStrike\"");
    let p = image("variable", &fixture(&shape(42, 100000, 100000, "", &runs)));
    assert_eq!(p.texts[0].decorations.len(), 4);
    assert_ne!(
        p.texts[0].decorations[0].rect.min.y,
        p.texts[0].decorations[2].rect.min.y
    );
    // One extra metrics call contains both instances, independent of glyph count.
    let plain = runs.replace("u=\"sng\" strike=\"sngStrike\"", "");
    let base = image(
        "variable-plain",
        &fixture(&shape(42, 100000, 100000, "", &plain)),
    );
    assert_eq!(
        p.text_work.component_calls,
        base.text_work.component_calls + 1
    );
}
#[test]
fn decorations_follow_bidi_lines_transforms_and_visibility() {
    if isolate("decorations_follow_bidi_lines_transforms_and_visibility") {
        return;
    }
    for (name, text, transform, attrs) in [
        (
            "rotated",
            "A A",
            "rot=\"1800000\"",
            "u=\"sng\" strike=\"sngStrike\"",
        ),
        ("flipped", "A A", "flipH=\"1\" flipV=\"1\"", "u=\"sng\""),
        ("wrapped", "A A A A A A", "", "u=\"sng\""),
        ("bidi", "A אב A", "", "u=\"sng\" strike=\"sngStrike\""),
    ] {
        let p = image(
            name,
            &fixture(&shape(
                42,
                100000,
                100000,
                transform,
                &decorated(text, attrs),
            )),
        );
        assert!(!p.texts[0].decorations.is_empty());
        if name == "wrapped" {
            assert!(p.texts[0].decorations.iter().any(|d| d.line > 0));
        }
    }
    let runs = decorated("A", "u=\"sng\"")
        .replace("</a:solidFill>", "</a:solidFill><a:uLnTx/><a:uFillTx/>");
    image(
        "follow-text",
        &fixture(&shape(42, 100000, 100000, "", &runs)),
    );
    let runs = decorated("A", "u=\"sng\"").replace(
        "<a:solidFill><a:srgbClr val=\"2070C0\"/></a:solidFill>",
        "<a:noFill/>",
    );
    let p = image("invisible", &fixture(&shape(42, 100000, 100000, "", &runs)));
    assert!(p.decoration_sources.is_empty());
    assert_eq!(p.text_work.component_calls, 3);
    let runs = decorated("A", "u=\"sng\"") + &colored("A", "FF0000") + &decorated("A", "u=\"sng\"");
    let p = image("gap", &fixture(&shape(42, 100000, 100000, "", &runs)));
    assert_eq!(p.texts[0].decorations.len(), 2);
}
#[derive(Default)]
struct Count {
    inner: NativeShaper,
    calls: u32,
}
impl TextBackend for Count {
    fn shape_batch(&mut self, f: &[u8], q: &[u32]) -> Result<Vec<u32>, TextError> {
        self.calls += 1;
        self.inner.shape_batch(f, q)
    }
    fn measure_batch(&mut self, f: &[u8], q: &[u32]) -> Result<Vec<u32>, TextError> {
        self.calls += 1;
        self.inner.measure_batch(f, q)
    }
    fn outline_batch(&mut self, f: &[u8], q: &[u32]) -> Result<Vec<u32>, TextError> {
        self.calls += 1;
        self.inner.outline_batch(f, q)
    }
    fn invalidate(&mut self) {
        self.inner.invalidate();
    }
}
#[derive(Default)]
struct NoRaster;
impl RasterBackend for NoRaster {
    fn raster(&mut self, _: &[u32]) -> Result<BackendReply, RasterError> {
        panic!("failed page cannot rasterize")
    }
    fn invalidate(&mut self) {}
}
#[test]
fn unsupported_decorations_clusters_metrics_and_budgets_fail_atomically() {
    if isolate("unsupported_decorations_clusters_metrics_and_budgets_fail_atomically") {
        return;
    }
    let f = fonts();
    let manifest =
        PreparedManifest::load(&f, FONT_BYTES, ManifestLimits::default(), &|| false).unwrap();
    for runs in [
        decorated("A", "u=\"wavy\""),
        decorated("A", "strike=\"dblStrike\""),
        decorated("A", "u=\"sng\"").replace("</a:solidFill>", "</a:solidFill><a:uLn w=\"1000\"/>"),
        decorated("A", "u=\"sng\"").replace(
            "</a:solidFill>",
            "</a:solidFill><a:uFill><a:pattFill prst=\"pct5\"/></a:uFill>",
        ),
    ] {
        let i = read(&fixture(&shape(42, 0, 0, "", &runs)));
        let mut b = Count::default();
        assert!(
            source_text_page::render(
                &i,
                &request(&i),
                &manifest,
                &mut b,
                &mut NoRaster,
                TextPageLimits::default(),
                &|| false
            )
            .is_err()
        );
        assert_eq!(b.calls, 0);
    }
    let source = fixture(&shape(
        42,
        100000,
        100000,
        "",
        &(decorated("A", "u=\"sng\"") + &colored("\u{301}", "2070C0")),
    ));
    let i = read(&source);
    let e = source_text_page::render(
        &i,
        &request(&i),
        &manifest,
        &mut NativeShaper::default(),
        &mut NoRaster,
        TextPageLimits::default(),
        &|| false,
    )
    .err()
    .unwrap();
    assert!(
        matches!(e,SourcePageError::AtObject{error,..} if matches!(*error,SourcePageError::GlyphPaintConflict{..}))
    );
    let i = read(&fixture(&shape(42, 0, 0, "", &decorated("A", "u=\"sng\""))));
    let old = author();
    let old_manifest = PreparedManifest::load(
        &old.manifest,
        include_bytes!("../../../fixtures/fonts/owned.ttf"),
        ManifestLimits::default(),
        &|| false,
    )
    .unwrap();
    let e = source_text_page::render(
        &i,
        &request(&i),
        &old_manifest,
        &mut NativeShaper::default(),
        &mut NoRaster,
        TextPageLimits::default(),
        &|| false,
    )
    .err()
    .unwrap();
    assert!(
        matches!(e,SourcePageError::AtObject{error,..} if matches!(*error,SourcePageError::TextDecoration(_)))
    );
    let mut limits = TextPageLimits::default();
    limits.work.max_component_calls = 3;
    let mut b = Count::default();
    assert!(
        source_text_page::render(
            &i,
            &request(&i),
            &manifest,
            &mut b,
            &mut NoRaster,
            limits,
            &|| false
        )
        .is_err()
    );
    assert_eq!(b.calls, 3);
}

#[test]
fn decoration_paths_and_metric_cancellation_obey_atomic_page_limits() {
    if isolate("decoration_paths_and_metric_cancellation_obey_atomic_page_limits") {
        return;
    }
    let f = fonts();
    let manifest =
        PreparedManifest::load(&f, FONT_BYTES, ManifestLimits::default(), &|| false).unwrap();
    let i = read(&fixture(&shape(
        42,
        0,
        0,
        "",
        &decorated("A", "u=\"sng\" strike=\"sngStrike\""),
    )));
    let mut limits = TextPageLimits::default();
    limits.work.max_path_commands = 5;
    let mut b = Count::default();
    let e = source_text_page::render(
        &i,
        &request(&i),
        &manifest,
        &mut b,
        &mut NoRaster,
        limits,
        &|| false,
    )
    .err()
    .unwrap();
    assert!(
        matches!(e,SourcePageError::AtObject{error,..} if matches!(*error,SourcePageError::Raster(RasterError::Limit("frame decoration paths"))))
    );
    assert_eq!(b.calls, 4);
    struct CancelMetrics {
        inner: NativeShaper,
        measures: std::rc::Rc<std::cell::Cell<u32>>,
        invalid: bool,
    }
    impl TextBackend for CancelMetrics {
        fn shape_batch(&mut self, f: &[u8], q: &[u32]) -> Result<Vec<u32>, TextError> {
            self.inner.shape_batch(f, q)
        }
        fn measure_batch(&mut self, f: &[u8], q: &[u32]) -> Result<Vec<u32>, TextError> {
            let r = self.inner.measure_batch(f, q);
            self.measures.set(self.measures.get() + 1);
            r
        }
        fn outline_batch(&mut self, f: &[u8], q: &[u32]) -> Result<Vec<u32>, TextError> {
            self.inner.outline_batch(f, q)
        }
        fn invalidate(&mut self) {
            self.invalid = true;
            self.inner.invalidate();
        }
    }
    let measures = std::rc::Rc::new(std::cell::Cell::new(0));
    let mut b = CancelMetrics {
        inner: NativeShaper::default(),
        measures: measures.clone(),
        invalid: false,
    };
    let e = source_text_page::render(
        &i,
        &request(&i),
        &manifest,
        &mut b,
        &mut NoRaster,
        TextPageLimits::default(),
        &|| measures.get() == 2,
    )
    .err()
    .unwrap();
    assert!(
        matches!(e,SourcePageError::AtObject{error,..} if matches!(*error,SourcePageError::Text(mo_presentation_compile::source_frame::SourceFrameError::Text(TextError::Cancelled))))
    );
    assert!(b.invalid);
}
