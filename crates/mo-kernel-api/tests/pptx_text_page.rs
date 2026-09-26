#[allow(dead_code)]
#[path = "../../../tools/test-support/source_text_page.rs"]
mod support;
use mo_kernel_api::*;
use mo_raster::{BackendReply, RasterBackend, RasterError};
use mo_text::{TextError, backend::TextBackend};
use support::*;

#[derive(Default)]
struct Text {
    calls: usize,
}
impl TextBackend for Text {
    fn shape_batch(&mut self, _: &[u8], _: &[u32]) -> Result<Vec<u32>, TextError> {
        self.calls += 1;
        Err(TextError::Host("unexpected text call"))
    }
    fn invalidate(&mut self) {}
}
#[derive(Default)]
struct Raster {
    calls: usize,
}
impl RasterBackend for Raster {
    fn raster(&mut self, _: &[u32]) -> Result<BackendReply, RasterError> {
        self.calls += 1;
        Err(RasterError::Host("unexpected raster call"))
    }
    fn invalidate(&mut self) {}
}
fn query(bytes: &[u8]) -> PptxTextPageRequest {
    PptxTextPageRequest {
        profile: PptxTextPageProfile::SolidTextDraftV1,
        page: request(&read(bytes)),
        fonts: author().manifest,
    }
}
fn reject(q: &str, source: &[u8], fonts: &[u8], check: &dyn Fn() -> bool) -> serde_json::Value {
    let (mut t, mut r) = (Text::default(), Raster::default());
    let (json, pixels) = render_pptx_text_page_json(q, source, fonts, &mut t, &mut r, check);
    assert_eq!((t.calls, r.calls), (0, 0));
    assert!(pixels.is_empty());
    let parsed: PptxTextPageRasterResponse = mo_common::from_json_str(&json).unwrap();
    assert!(matches!(parsed, PptxTextPageRasterResponse::Error { .. }));
    serde_json::from_str::<serde_json::Value>(&json).unwrap()["error"].clone()
}
static FONT_BYTES: &[u8] = include_bytes!("../../../fixtures/fonts/owned.ttf");
#[test]
fn text_page_request_resource_identity_and_cancel_fail_before_components() {
    let source = fixture(&shape(42, 0, 0, "", &colored("A", "FF0000")));
    let q = query(&source);
    let json = serde_json::to_string(&q).unwrap();
    let e = reject(&json, &source, FONT_BYTES, &|| true);
    assert_eq!(e["error"]["code"], "CANCELLED");
    let mut bad = serde_json::to_value(&q).unwrap();
    bad["profile"] = "html".into();
    let e = reject(&bad.to_string(), &source, FONT_BYTES, &|| false);
    assert_eq!(e["stage"], "request");
    let duplicate = json.replacen("\"fonts\":", "\"fonts\":{},\"fonts\":", 1);
    assert_eq!(
        reject(&duplicate, &source, FONT_BYTES, &|| false)["stage"],
        "request"
    );
    let mut bad = serde_json::to_value(&q).unwrap();
    bad["page"]["expectedSourceSha256"] = "0".repeat(64).into();
    let e = reject(&bad.to_string(), &source, &[], &|| false);
    assert_eq!(e["stage"], "source");
    assert_eq!(e["error"]["code"], "SOURCE_CONFLICT");
    let mut bytes = FONT_BYTES.to_vec();
    bytes[0] ^= 1;
    let e = reject(&json, &source, &bytes, &|| false);
    assert_eq!(e["stage"], "fonts");
    assert_eq!(e["error"]["code"], "RESOURCE_CONFLICT");
    let mut bad = serde_json::to_value(&q).unwrap();
    bad["fonts"]["faces"][0]["family"]["expected"] = "Unverified name".into();
    let e = reject(&bad.to_string(), &source, FONT_BYTES, &|| false);
    assert_eq!(e["stage"], "fonts");
}
#[test]
fn text_page_mapping_diagnostics_retain_object_and_typed_native_reason() {
    for (extra, runs, kind) in [
        (
            "",
            colored("A", "FF0000").replace("<a:rPr>", "<a:rPr u=\"wavy\">"),
            "paintProperty",
        ),
        ("", "<a:r><a:rPr/><a:t>A</a:t></a:r>".into(), "missingPaint"),
    ] {
        let source = fixture(&shape(42, 0, 0, extra, &runs));
        let source = if kind == "missingPaint" {
            without_default_text_style(&source)
        } else {
            source
        };
        let json = serde_json::to_string(&query(&source)).unwrap();
        let e = reject(&json, &source, FONT_BYTES, &|| false);
        assert_eq!(e["stage"], "page");
        assert_eq!(e["error"]["code"], "MAPPING_NOT_IMPLEMENTED");
        assert_eq!(e["error"]["location"]["part"], SLIDE);
        assert_eq!(e["error"]["location"]["object"], 42);
        assert_eq!(e["detail"]["kind"], kind);
    }
    let source = fixture(&shape(42, 0, 0, "", &colored("A", "FF0000")));
    let source = rewrite(&source, SLIDE, |s| {
        s.replace("<a:noAutofit/>", "<a:spAutoFit/>")
    });
    let json = serde_json::to_string(&query(&source)).unwrap();
    let e = reject(&json, &source, FONT_BYTES, &|| false);
    assert_eq!(e["detail"]["kind"], "frame");
    assert_eq!(e["detail"]["reason"]["kind"], "autofit");
    assert_eq!(e["error"]["location"]["object"], 42);
}

#[test]
fn missing_font_preserves_all_merged_native_uses_and_insertion_style() {
    let source = fixture(&shape(
        42,
        0,
        0,
        "",
        &(colored("A", "FF0000") + &colored("A", "0000FF")),
    ));
    let mut q = query(&source);
    q.fonts.typefaces.clear();
    let e = reject(
        &serde_json::to_string(&q).unwrap(),
        &source,
        FONT_BYTES,
        &|| false,
    );
    assert_eq!(e["error"]["code"], "RESOURCE_REQUIRED");
    assert_eq!(e["detail"]["kind"], "fontSelection");
    let f = &e["detail"]["failure"];
    assert_eq!(f["sourceSha256"], q.page.expected_source_sha256.as_str());
    assert_eq!(f["object"]["nativeId"], 42);
    assert_eq!(f["paragraph"], 0);
    assert_eq!(f["selection"]["typeface"], FONT);
    assert_eq!(f["selection"]["fontStyle"], "regular");
    assert_eq!(f["selection"]["reason"], "unmappedTypeface");
    let runs: Vec<_> = f["uses"]
        .as_array()
        .unwrap()
        .iter()
        .map(|u| u["run"].clone())
        .collect();
    assert_eq!(
        runs,
        vec![
            serde_json::json!(0),
            serde_json::json!(1),
            serde_json::Value::Null
        ]
    );
    assert!(
        f["uses"]
            .as_array()
            .unwrap()
            .iter()
            .all(|u| u["font"]["typeface"] == FONT)
    );
}

#[test]
fn insertion_font_requirement_is_distinct_from_content_and_missing_style_is_explicit() {
    let source = fixture(&shape(42, 0, 0, "", &colored("A", "FF0000")));
    let source = rewrite(&source, SLIDE, |s| {
        s.replace("</a:p>", "<a:endParaRPr b=\"1\" i=\"1\"/></a:p>")
    });
    let q = query(&source);
    let e = reject(
        &serde_json::to_string(&q).unwrap(),
        &source,
        FONT_BYTES,
        &|| false,
    );
    let f = &e["detail"]["failure"];
    assert_eq!(e["error"]["code"], "RESOURCE_REQUIRED");
    assert_eq!(f["selection"]["fontStyle"], "boldItalic");
    assert_eq!(f["selection"]["reason"], "missingStyle");
    assert_eq!(f["uses"].as_array().unwrap().len(), 1);
    assert!(f["uses"][0]["run"].is_null());
}

#[test]
fn paint_failure_locates_the_later_native_run_including_unresolved_color() {
    let first = colored("A", "FF0000");
    for (second, kind) in [
        (
            colored("A", "0000FF").replace("<a:rPr>", "<a:rPr u=\"wavy\">"),
            "paintProperty",
        ),
        ("<a:r><a:rPr/><a:t>A</a:t></a:r>".into(), "missingPaint"),
        (
            colored("A", "0000FF")
                .replace("<a:srgbClr val=\"0000FF\"/>", "<a:sysClr val=\"window\"/>"),
            "color",
        ),
    ] {
        let source = fixture(&shape(42, 0, 0, "", &format!("{first}{second}")));
        let source = if kind == "missingPaint" {
            without_default_text_style(&source)
        } else {
            source
        };
        let q = query(&source);
        let e = reject(
            &serde_json::to_string(&q).unwrap(),
            &source,
            FONT_BYTES,
            &|| false,
        );
        assert_eq!(e["detail"]["kind"], kind);
        assert_eq!(e["paintLocation"]["paragraph"], 0);
        assert_eq!(e["paintLocation"]["run"], 1);
        assert!(e["paintLocation"]["sourceOrdinal"].as_u64().unwrap() > 0);
    }
}
