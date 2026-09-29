use mo_common::Digest;
use mo_pptx::source::{
    text::{cascade::CharacterProperty, fonts::*},
    *,
};
use mo_presentation_compile::source_text::*;
use mo_text::{TextError, backend::TextBackend, manifest::*};
use std::cell::Cell;

#[allow(dead_code)]
#[path = "../../../tools/test-support/source_glyphs.rs"]
mod support;
use support::*;

fn prepare_index(i: &SourceIndex) -> SourceTextPreparation {
    prepare(
        i,
        &i.source_sha256,
        &target(i),
        SourceTextLimits::default(),
        &|| false,
    )
    .unwrap()
}
fn prepared(b: &[u8]) -> PreparedSourceText {
    let i = read(b);
    let SourceTextPreparation::Prepared { text } = prepare_index(&i) else {
        panic!("{:?}", prepare_index(&i))
    };
    if let Some(directory) = std::env::var_os("MO_SOURCE_GLYPH_EVIDENCE_DIR") {
        let directory = std::path::PathBuf::from(directory);
        assert!(directory.is_absolute());
        std::fs::create_dir_all(&directory).unwrap();
        let stem = i.source_sha256.as_str();
        std::fs::write(directory.join(format!("{stem}.pptx")), b).unwrap();
        std::fs::write(
            directory.join(format!("{stem}.json")),
            serde_json::to_vec_pretty(
                &serde_json::json!({"source":text.source(),"paragraphs":text.paragraphs()}),
            )
            .unwrap(),
        )
        .unwrap();
    }
    text
}
fn issue(xml: &str) -> SourceTextIssue {
    match prepare_index(&read(&bytes(xml))) {
        SourceTextPreparation::Unresolved { issue } => issue,
        other => panic!("{other:?}"),
    }
}

#[test]
fn physical_runs_coalesce_without_losing_scalar_origins_or_insertion_style() {
    let b = bytes(&format!(
        "<a:p>{}{}{}<a:endParaRPr sz=\"6000\"/></a:p>",
        run("A"),
        run("\u{301}"),
        run(" A")
    ));
    let text = prepared(&b);
    let p = &text.paragraphs()[0];
    assert_eq!(p.text, "A\u{301} A");
    assert_eq!(p.spans.len(), 1);
    assert_eq!(
        p.sources
            .iter()
            .map(|r| (r.start, r.end))
            .collect::<Vec<_>>(),
        [(0, 1), (1, 2), (2, 4)]
    );
    assert_eq!(p.geometry[p.end_style as usize].font_size.get(), 6000 * 127);
    assert_ne!(p.end_style, p.spans[0].style);
    assert_eq!(p.styles[p.spans[0].style as usize].features[0].value, 0);
    for (run, source) in text.source().paragraphs[0].runs.iter().zip(&p.sources) {
        assert_eq!(run.source_ordinal, source.source_ordinal);
    }
    assert!(p.fonts.last().unwrap().run.is_none());
}
#[test]
fn grapheme_style_changes_do_not_split_combining_sequence() {
    assert!(matches!(
        issue(
            "<a:p><a:r><a:t>A</a:t></a:r><a:r><a:rPr sz=\"4000\"/><a:t>\u{301}</a:t></a:r></a:p>"
        ),
        SourceTextIssue::GraphemeStyleConflict { boundary: 1, .. }
    ));
}
#[test]
fn mixed_scripts_resolve_native_slots_and_rtl_without_host_locale() {
    let text = prepared(&bytes(
        "<a:p><a:pPr rtl=\"1\"/><a:r><a:rPr lang=\"zh-CN\"/><a:t>A中ع</a:t></a:r></a:p>",
    ));
    let p = &text.paragraphs()[0];
    assert_eq!(
        p.fonts
            .iter()
            .take(3)
            .map(|f| (f.script.as_str(), f.slot, f.font.typeface.as_str()))
            .collect::<Vec<_>>(),
        [
            ("Latn", NativeFontSlot::Latin, FONT),
            ("Hani", NativeFontSlot::EastAsian, "EA"),
            ("Arab", NativeFontSlot::ComplexScript, "CS")
        ]
    );
    assert_eq!(p.fonts[1].theme_script.as_deref(), Some("Hans"));
    assert!(matches!(
        p.direction,
        mo_unicode::bidi::ParagraphDirection::RightToLeft
    ));
}
#[test]
fn explicit_language_binds_mixed_latin_and_ideographic_punctuation_without_guessing() {
    for (language, script) in [("zh-CN", "Hani"), ("ja", "Kana"), ("ko-KR", "Hang")] {
        let text = prepared(&bytes(&format!(
            "<a:p><a:r><a:rPr lang=\"{language}\"/><a:t>PPT。</a:t></a:r></a:p>"
        )));
        let p = &text.paragraphs()[0];
        assert_eq!(p.text, "PPT。");
        assert_eq!(p.font_spans.len(), 2);
        let first = &p.fonts[p.font_spans[0].binding as usize];
        let punctuation = &p.fonts[p.font_spans[1].binding as usize];
        assert_eq!(first.script, "Latn");
        assert_eq!(first.slot, NativeFontSlot::Latin);
        assert_eq!(punctuation.script, script);
        assert_eq!(punctuation.slot, NativeFontSlot::EastAsian);
        assert_eq!(punctuation.font.typeface, "EA");
        assert_eq!(p.font_spans[1].start, 3);
        assert_eq!(p.font_spans[1].end, 4);
        assert_eq!(p.sources.len(), 1);
    }
    for language in ["en", "und", "en-x-Hani", "zh-Latn"] {
        assert!(matches!(issue(&format!(
            "<a:p><a:r><a:rPr lang=\"{language}\"/><a:t>PPT。</a:t></a:r></a:p>"
        )), SourceTextIssue::Itemization { notice, .. } if notice.kind == mo_text::itemize::ItemizationNoticeKind::AmbiguousScript));
    }
}
#[test]
fn theme_supplements_follow_explicit_language_and_missing_language_is_diagnostic() {
    let b = bytes(
        "<a:p><a:r><a:rPr lang=\"zh-Hant\"><a:ea typeface=\"+mn-ea\"/></a:rPr><a:t>中</a:t></a:r></a:p>",
    );
    let b = rewrite(&b, "/ppt/theme/theme2.xml", |mut s| {
        let at = s.find("<a:fontScheme").unwrap();
        let end = at + s[at..].find("</a:fontScheme>").unwrap() + "</a:fontScheme>".len();
        s.replace_range(at..end, &format!("<a:fontScheme name=\"Owned\"><a:majorFont><a:latin typeface=\"{FONT}\"/><a:ea typeface=\"\"/><a:cs typeface=\"\"/></a:majorFont><a:minorFont><a:latin typeface=\"{FONT}\"/><a:ea typeface=\"\"/><a:cs typeface=\"\"/><a:font script=\"Hans\" typeface=\"SC\"/><a:font script=\"Hant\" typeface=\"TC\"/></a:minorFont></a:fontScheme>"));
        s
    });
    let text = prepared(&b);
    assert_eq!(text.paragraphs()[0].fonts[0].font.typeface, "TC");
    let b = rewrite(&b, SLIDE, |s| s.replace("zh-Hant", "en"));
    assert!(matches!(
        prepare_index(&read(&b)),
        SourceTextPreparation::Unresolved {
            issue: SourceTextIssue::Typeface {
                reason: TypefaceUnresolved::ScriptRequired {},
                ..
            }
        }
    ));
}
#[test]
fn break_empty_run_and_empty_paragraph_keep_their_native_identity() {
    let text = prepared(&bytes(
        "<a:p><a:r><a:t>A</a:t></a:r><a:r><a:t></a:t></a:r><a:br/><a:r><a:t>A</a:t></a:r></a:p><a:p><a:endParaRPr sz=\"3200\"/></a:p>",
    ));
    let p = &text.paragraphs()[0];
    assert_eq!(p.text, "A\u{2028}A");
    assert_eq!((p.sources[1].start, p.sources[1].end), (1, 1));
    assert_eq!(p.sources[2].kind, SourceRunKind::Break);
    assert_eq!((p.sources[2].start, p.sources[2].end), (1, 2));
    let empty = &text.paragraphs()[1];
    assert!(empty.spans.is_empty() && empty.sources.is_empty());
    assert_eq!(
        empty.geometry[empty.end_style as usize].font_size.get(),
        3200 * 127
    );
}
#[test]
fn source_kerning_threshold_and_case_normalization_only_change_computation() {
    let text = prepared(&bytes(
        "<a:p><a:r><a:rPr lang=\"EN-US\" sz=\"1800\" kern=\"1800\"/><a:t>A</a:t></a:r><a:r><a:rPr lang=\"en-us\" sz=\"1800\" kern=\"0\"/><a:t>A</a:t></a:r><a:r><a:rPr sz=\"1799\" kern=\"1800\"/><a:t>A</a:t></a:r></a:p>",
    ));
    let p = &text.paragraphs()[0];
    assert_eq!(p.spans.len(), 2);
    assert_eq!(p.spans[0].end, 2);
    assert_eq!(p.styles[p.spans[0].style as usize].features[0].value, 1);
    assert_eq!(p.styles[p.spans[1].style as usize].features[0].value, 0);
    assert_eq!(
        text.source().paragraphs[0].runs[0]
            .style
            .attributes
            .language
            .as_deref(),
        Some("EN-US")
    );
}
#[test]
fn unsupported_native_effects_on_text_computation_never_silently_disappear() {
    for (attrs, property) in [
        ("cap=\"all\"", CharacterProperty::Caps),
        ("normalizeH=\"1\"", CharacterProperty::NormalizeHeight),
        ("kumimoji=\"1\"", CharacterProperty::Kumimoji),
    ] {
        assert!(
            matches!(issue(&format!("<a:p><a:r><a:rPr {attrs}/><a:t>A</a:t></a:r></a:p>")), SourceTextIssue::CharacterProperty { property: actual, .. } if actual == property)
        );
    }
    assert!(matches!(
        issue("<a:p><a:r><a:rPr><a:rtl val=\"1\"/></a:rPr><a:t>A</a:t></a:r></a:p>"),
        SourceTextIssue::DirectionOverride { .. }
    ));
    assert!(matches!(
        issue(
            "<a:p><a:fld id=\"{33333333-3333-3333-3333-333333333333}\" type=\"slidenum\"><a:t>1</a:t></a:fld></a:p>"
        ),
        SourceTextIssue::Field { .. }
    ));
    assert!(matches!(
        issue("<a:p><a:r><a:t>A&#10;A</a:t></a:r></a:p>"),
        SourceTextIssue::ParagraphControl { .. }
    ));
    assert!(matches!(
        issue("<a:p><a:r><a:t>𐐀</a:t></a:r></a:p>"),
        SourceTextIssue::Script { .. }
    ));
    let _zero = prepared(&bytes(
        "<a:p><a:r><a:rPr baseline=\"-0.00000000%\" spc=\"0.000pt\"/><a:t>A</a:t></a:r></a:p>",
    ));
}
#[test]
fn limits_cancel_every_checkpoint_and_stale_source_never_return_prepared_state() {
    let i = read(&bytes(&format!(
        "<a:p>{}</a:p><a:p>{}</a:p>",
        run("A中ع"),
        run("AA")
    )));
    let before = i.clone();
    let calls = Cell::new(0);
    let count = || {
        calls.set(calls.get() + 1);
        false
    };
    prepare(
        &i,
        &i.source_sha256,
        &target(&i),
        SourceTextLimits::default(),
        &count,
    )
    .unwrap();
    for at in 1..=calls.get() {
        let seen = Cell::new(0);
        let check = || {
            seen.set(seen.get() + 1);
            seen.get() == at
        };
        assert!(
            prepare(
                &i,
                &i.source_sha256,
                &target(&i),
                SourceTextLimits::default(),
                &check
            )
            .is_err(),
            "checkpoint {at}"
        );
    }
    let limits = SourceTextLimits {
        max_text_bytes: 1,
        ..SourceTextLimits::default()
    };
    assert!(matches!(
        prepare(&i, &i.source_sha256, &target(&i), limits, &|| false),
        Err(SourceTextError::Limit(_))
    ));
    let stale = Digest::try_from("0".repeat(64)).unwrap();
    assert!(
        prepare(
            &i,
            &stale,
            &target(&i),
            SourceTextLimits::default(),
            &|| false
        )
        .is_err()
    );
    assert_eq!(i, before);
    assert!(matches!(
        prepare_index(&i),
        SourceTextPreparation::Prepared { .. }
    ));
}
struct FailBackend(usize);

#[test]
fn preparation_budget_is_shared_across_paragraphs_and_symbol_fonts_are_explicit() {
    let i = read(&bytes(
        "<a:p><a:r><a:t>A</a:t></a:r></a:p><a:p><a:r><a:t>A</a:t></a:r></a:p>",
    ));
    // Two bindings per paragraph (run and insertion style); a fresh per-paragraph
    // counter would incorrectly accept a budget of three for this object.
    let limits = SourceTextLimits {
        max_font_bindings: 3,
        ..SourceTextLimits::default()
    };
    assert!(matches!(
        prepare(&i, &i.source_sha256, &target(&i), limits, &|| false),
        Err(SourceTextError::Limit("source object font bindings"))
    ));
    let limits = SourceTextLimits {
        max_plan_bytes: 1,
        ..SourceTextLimits::default()
    };
    assert!(matches!(
        prepare(&i, &i.source_sha256, &target(&i), limits, &|| false),
        Err(SourceTextError::Limit("source plan bytes"))
    ));
    for decl in [
        "<a:sym typeface=\"Wingdings\"/>",
        "<a:latin typeface=\"Wingdings\" charset=\"2\"/>",
    ] {
        assert!(matches!(
            issue(&format!(
                "<a:p><a:r><a:rPr>{decl}</a:rPr><a:t>A</a:t></a:r></a:p>"
            )),
            SourceTextIssue::SymbolFont { .. }
        ));
    }
}

impl TextBackend for FailBackend {
    fn shape_batch(&mut self, _: &[u8], _: &[u32]) -> Result<Vec<u32>, TextError> {
        self.0 += 1;
        Err(TextError::Host("injected source glyph failure"))
    }
    fn invalidate(&mut self) {}
}
#[test]
fn real_manifest_validation_precedes_backend_and_failure_preserves_prepared_text() {
    let fixture: ManifestParagraphRequest = serde_json::from_str(include_str!(
        "../../../fixtures/fonts/manifest-paragraph.json"
    ))
    .unwrap();
    let manifest = PreparedManifest::load(
        &fixture.manifest,
        include_bytes!("../../../fixtures/fonts/owned.ttf"),
        ManifestLimits::default(),
        &|| false,
    )
    .unwrap();
    let text = prepared(&bytes(&format!("<a:p>{}</a:p>", run("A"))));
    let before = serde_json::to_vec(text.paragraphs()).unwrap();
    let mut backend = FailBackend(0);
    assert!(
        text.shape_paragraph(0, &manifest, &mut backend, &|| false)
            .is_err()
    );
    assert_eq!(backend.0, 1);
    assert!(
        text.shape_paragraph(0, &manifest, &mut backend, &|| true)
            .is_err()
    );
    assert!(
        text.shape_paragraph(99, &manifest, &mut backend, &|| false)
            .is_err()
    );
    assert_eq!(backend.0, 1);
    let unavailable = prepared(&bytes("<a:p><a:r><a:rPr b=\"1\"/><a:t>A</a:t></a:r></a:p>"));
    assert!(
        unavailable
            .shape_paragraph(0, &manifest, &mut backend, &|| false)
            .is_err()
    );
    assert_eq!(backend.0, 1);
    assert_eq!(serde_json::to_vec(text.paragraphs()).unwrap(), before);
    assert_eq!(manifest.verified_faces(), 1);
}
