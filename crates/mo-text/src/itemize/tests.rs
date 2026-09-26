use super::*;
use mo_unicode::bidi::ParagraphDirection;
fn request(text: &str) -> ItemizationRequest {
    ItemizationRequest {
        text: text.into(),
        direction: ParagraphDirection::AutoLeftToRight,
        spans: if text.is_empty() {
            vec![]
        } else {
            vec![StyleSpan {
                end: text.chars().count() as u32,
                style: 0,
            }]
        },
    }
}
fn plan(text: &str) -> ItemizationResult {
    itemize(&request(text), ItemizationLimits::default(), &|| false).unwrap()
}
fn ranges(text: &str) -> Vec<(u32, u32, &'static str)> {
    plan(text)
        .items
        .iter()
        .map(|i| (i.start.scalar_offset, i.end.scalar_offset, i.script.tag()))
        .collect()
}
#[test]
fn paired_punctuation_uses_outer_script_across_script_and_direction_changes() {
    assert_eq!(
        ranges("A(γ)B"),
        vec![(0, 2, "Latn"), (2, 3, "Grek"), (3, 5, "Latn")]
    );
    assert_eq!(
        ranges("A(אב)B"),
        vec![(0, 2, "Latn"), (2, 4, "Hebr"), (4, 6, "Latn")]
    );
    assert_eq!(ranges("(γ) A"), vec![(0, 4, "Grek"), (4, 5, "Latn")]);
    assert_eq!(
        ranges("A（γ）B"),
        vec![(0, 2, "Latn"), (2, 3, "Grek"), (3, 5, "Latn")]
    );
}
#[test]
fn script_extensions_constrain_common_symbols_and_shared_explicit_digits() {
    assert_eq!(ranges("Aーア"), vec![(0, 1, "Latn"), (1, 3, "Kana")]);
    assert_eq!(ranges("あー"), vec![(0, 2, "Hira")]);
    assert_eq!(ranges("\u{11800}\u{0967}"), vec![(0, 2, "Dogr")]);
    assert_eq!(ranges("\u{0710}\u{0640}"), vec![(0, 2, "Syrc")]);
    let isolated = plan("ー");
    assert_eq!(isolated.items[0].script.tag(), "Hira");
    assert_eq!(
        isolated.notices[0].kind,
        ItemizationNoticeKind::AmbiguousScript
    );
}
#[test]
fn marks_and_emoji_are_never_split_and_foreign_marks_are_diagnosed() {
    let r = plan("A\u{05b0}");
    assert_eq!(r.items.len(), 1);
    assert_eq!(r.items[0].script.tag(), "Latn");
    assert_eq!(r.notices[0].kind, ItemizationNoticeKind::MixedScriptCluster);
    assert_eq!(ranges("\u{25cc}\u{05b0}"), vec![(0, 2, "Hebr")]);
    let emoji = plan("👩🏽‍💻");
    assert_eq!(emoji.items.len(), 1);
    assert_eq!(emoji.items[0].end.utf16_offset, 7);
    assert_eq!(emoji.items[0].end.utf8_offset, 15);
    let prepend = plan("\u{0600}A");
    assert_eq!(prepend.items.len(), 1);
    assert!(
        prepend
            .notices
            .iter()
            .any(|n| n.kind == ItemizationNoticeKind::MixedLevelCluster)
    );
}
#[test]
fn isolates_do_not_leak_script_context_or_bracket_pairs() {
    let r = plan("A(\u{2067}אב(γ)\u{2069})B");
    let script_at = |at| {
        r.items
            .iter()
            .find(|i| i.start.scalar_offset <= at && at < i.end.scalar_offset)
            .unwrap()
            .script
            .tag()
    };
    assert_eq!(script_at(1), "Latn");
    assert_eq!(script_at(5), "Hebr");
    assert_eq!(script_at(6), "Grek");
    assert_eq!(script_at(7), "Hebr");
    assert_eq!(script_at(9), "Latn");
    assert_eq!(
        r.items
            .iter()
            .filter(|i| i.kind == TextItemKind::BidiControl)
            .count(),
        2
    );
}
#[test]
fn controls_and_style_changes_keep_exhaustive_logical_coordinates() {
    let mut q = request("A\tאב\u{2028}B\r\n");
    q.spans = vec![
        StyleSpan { end: 1, style: 7 },
        StyleSpan { end: 8, style: 9 },
    ];
    let r = itemize(&q, ItemizationLimits::default(), &|| false).unwrap();
    assert_eq!(
        r.items.iter().map(|i| i.kind).collect::<Vec<_>>(),
        vec![
            TextItemKind::Text,
            TextItemKind::Tab,
            TextItemKind::Text,
            TextItemKind::LineBreak,
            TextItemKind::Text,
            TextItemKind::ParagraphBreak
        ]
    );
    assert_eq!(r.items[0].style, 7);
    assert_eq!(r.items.last().unwrap().end.scalar_offset, 8);
    for pair in r.items.windows(2) {
        assert_eq!(pair[0].end, pair[1].start);
    }
    let mut q = request("fi");
    q.spans = vec![
        StyleSpan { end: 1, style: 0 },
        StyleSpan { end: 2, style: 0 },
    ];
    assert_eq!(
        itemize(&q, ItemizationLimits::default(), &|| false)
            .unwrap()
            .items
            .len(),
        1
    );
    q.spans[1].style = 1;
    assert_eq!(
        itemize(&q, ItemizationLimits::default(), &|| false)
            .unwrap()
            .items
            .len(),
        2
    );
    assert!(plan("").items.is_empty());
}
#[test]
fn invalid_spans_paragraphs_budgets_and_every_cancel_checkpoint_fail_atomically() {
    for q in [
        ItemizationRequest {
            spans: vec![
                StyleSpan { end: 1, style: 0 },
                StyleSpan { end: 2, style: 0 },
            ],
            ..request("A\u{301}")
        },
        ItemizationRequest {
            spans: vec![],
            ..request("A")
        },
        request("A\nB"),
    ] {
        assert!(matches!(
            itemize(&q, ItemizationLimits::default(), &|| false),
            Err(TextError::Invalid(_))
        ));
    }
    let q = request("A(γ)\u{2067}אב\u{2069}");
    for limit in [
        ItemizationLimits {
            max_styles: 0,
            ..ItemizationLimits::default()
        },
        ItemizationLimits {
            max_items: 1,
            ..ItemizationLimits::default()
        },
    ] {
        assert!(matches!(
            itemize(&q, limit, &|| false),
            Err(TextError::Limit(_))
        ));
    }
    assert!(matches!(
        itemize(
            &request("ー"),
            ItemizationLimits {
                max_notices: 0,
                ..ItemizationLimits::default()
            },
            &|| false
        ),
        Err(TextError::Limit(_))
    ));
    let checks = std::cell::Cell::new(0);
    itemize(&q, ItemizationLimits::default(), &|| {
        checks.set(checks.get() + 1);
        false
    })
    .unwrap();
    for stop in 1..=checks.get() {
        let n = std::cell::Cell::new(0);
        assert!(matches!(
            itemize(&q, ItemizationLimits::default(), &|| {
                n.set(n.get() + 1);
                n.get() == stop
            }),
            Err(TextError::Cancelled)
        ));
    }
}
#[test]
fn deep_and_unmatched_brackets_preserve_all_text_without_depth_truncation() {
    let text = format!(
        "A{}{}γ{}B",
        "(".repeat(8000),
        "]".repeat(8000),
        ")".repeat(8000)
    );
    let r = plan(&text);
    assert_eq!(r.items.first().unwrap().start.scalar_offset, 0);
    assert_eq!(
        r.items.last().unwrap().end.scalar_offset,
        text.chars().count() as u32
    );
    assert_eq!(r.items.last().unwrap().script.tag(), "Latn");
}
#[test]
fn script_data_preserves_unknown_and_full_extensions_without_locale_guessing() {
    use mo_unicode::script::*;
    for (c, primary, ext) in [
        ('A', "Latn", vec!["Latn"]),
        ('ー', "Zyyy", vec!["Hira", "Kana"]),
        ('\u{05b0}', "Hebr", vec!["Hebr"]),
        ('\u{e000}', "Zzzz", vec!["Zzzz"]),
        ('\u{0378}', "Zzzz", vec!["Zzzz"]),
    ] {
        assert_eq!(script(c).tag(), primary);
        assert_eq!(
            script_extensions(c)
                .iter()
                .map(|s| s.tag())
                .collect::<Vec<_>>(),
            ext
        );
    }
    assert!(Script::from_tag("latin").is_none());
    assert!(Script::from_tag("latn").is_none());
    assert_eq!(
        serde_json::from_str::<Script>("\"Latn\"").unwrap().tag(),
        "Latn"
    );
    assert!(serde_json::from_str::<Script>("\"xxxx\"").is_err());
}
