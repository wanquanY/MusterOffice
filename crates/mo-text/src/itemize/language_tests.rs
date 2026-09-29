use super::*;
use mo_unicode::bidi::ParagraphDirection;

fn request(text: &str) -> ItemizationRequest {
    ItemizationRequest {
        text: text.into(),
        direction: ParagraphDirection::AutoLeftToRight,
        spans: vec![StyleSpan {
            end: text.chars().count() as u32,
            style: 0,
        }],
    }
}

fn plan(text: &str, language: &str) -> ItemizationResult {
    let q = request(text);
    itemize_with_languages(
        &q,
        &[LanguageSpan {
            end: q.spans[0].end,
            language,
        }],
        ItemizationLimits::default(),
        &|| false,
    )
    .unwrap()
}

#[test]
fn authored_language_resolves_isolated_punctuation_but_never_changes_strong_text() {
    for (language, expected) in [("zh-CN", "Hani"), ("ja", "Kana"), ("ko", "Hang")] {
        let r = plan("PPT。", language);
        assert!(r.notices.is_empty(), "{language}: {:?}", r.notices);
        assert_eq!(r.items.len(), 2);
        assert_eq!(r.items[0].script.tag(), "Latn");
        assert_eq!(r.items[1].script.tag(), expected);
        assert_eq!(r.items[1].start.scalar_offset, 3);
        assert_eq!(r.items[1].end.scalar_offset, 4);
    }
    let r = plan("アー", "zh-CN");
    assert!(r.notices.is_empty());
    assert_eq!(r.items.len(), 1);
    assert_eq!(r.items[0].script.tag(), "Kana");
    // The hint is constrained by SCX. Han cannot decide the prolonged sound mark.
    assert_eq!(plan("ー", "zh-CN").notices.len(), 1);
    assert!(plan("ー", "ja").notices.is_empty());
    assert_eq!(plan("...", "zh-CN").items[0].script.tag(), "Zyyy");
}

#[test]
fn absent_invalid_or_incompatible_languages_keep_the_original_diagnostic() {
    for language in [
        "und",
        "en",
        "en-x-Hani",
        "en-u-sc-hani",
        "zh-Latn",
        "zh--CN",
    ] {
        let r = plan("PPT。", language);
        assert!(
            r.notices
                .iter()
                .any(|n| n.kind == ItemizationNoticeKind::AmbiguousScript)
        );
    }
    let q = request("PPT。");
    let legacy = itemize(&q, ItemizationLimits::default(), &|| false).unwrap();
    let empty = itemize_with_languages(&q, &[], ItemizationLimits::default(), &|| false).unwrap();
    assert_eq!(
        serde_json::to_value(legacy).unwrap(),
        serde_json::to_value(empty).unwrap()
    );
}

#[test]
fn language_boundaries_preserve_each_run_and_do_not_split_a_grapheme() {
    let q = request("。。");
    let r = itemize_with_languages(
        &q,
        &[
            LanguageSpan {
                end: 1,
                language: "zh-CN",
            },
            LanguageSpan {
                end: 2,
                language: "ja",
            },
        ],
        ItemizationLimits::default(),
        &|| false,
    )
    .unwrap();
    assert!(r.notices.is_empty());
    assert_eq!(
        r.items.iter().map(|i| i.script.tag()).collect::<Vec<_>>(),
        ["Hani", "Kana"]
    );
    let q = request("。\u{fe00}");
    for (second, ambiguous) in [("zh-CN", false), ("ja", true), ("und", true)] {
        let r = itemize_with_languages(
            &q,
            &[
                LanguageSpan {
                    end: 1,
                    language: "zh-CN",
                },
                LanguageSpan {
                    end: 2,
                    language: second,
                },
            ],
            ItemizationLimits::default(),
            &|| false,
        )
        .unwrap();
        assert_eq!(r.items.len(), 1);
        assert_eq!(r.items[0].end.scalar_offset, 2);
        assert_eq!(!r.notices.is_empty(), ambiguous);
    }
}

#[test]
fn language_hints_do_not_leak_out_of_bidi_isolates() {
    let q = request("。\u{2066}。\u{2069}。");
    let r = itemize_with_languages(
        &q,
        &[
            LanguageSpan {
                end: 2,
                language: "und",
            },
            LanguageSpan {
                end: 3,
                language: "zh-CN",
            },
            LanguageSpan {
                end: 5,
                language: "und",
            },
        ],
        ItemizationLimits::default(),
        &|| false,
    )
    .unwrap();
    assert_eq!(
        r.notices.iter().map(|n| n.start).collect::<Vec<_>>(),
        [0, 4]
    );
    assert_eq!(
        r.items
            .iter()
            .find(|i| i.start.scalar_offset == 2)
            .unwrap()
            .script
            .tag(),
        "Hani"
    );
}

#[test]
fn language_ranges_limits_and_cancellation_are_atomic() {
    let q = request("PPT。");
    for spans in [
        vec![LanguageSpan {
            end: 3,
            language: "zh",
        }],
        vec![LanguageSpan {
            end: 5,
            language: "zh",
        }],
        vec![LanguageSpan {
            end: 0,
            language: "zh",
        }],
    ] {
        assert!(matches!(
            itemize_with_languages(&q, &spans, ItemizationLimits::default(), &|| false),
            Err(TextError::Invalid(_))
        ));
    }
    let too_long = "a".repeat(256);
    assert!(matches!(
        itemize_with_languages(
            &q,
            &[LanguageSpan {
                end: 4,
                language: &too_long
            }],
            ItemizationLimits::default(),
            &|| false
        ),
        Err(TextError::Limit(_))
    ));
    let spans = [LanguageSpan {
        end: 4,
        language: "zh-CN",
    }];
    assert!(matches!(
        itemize_with_languages(
            &q,
            &spans,
            ItemizationLimits {
                max_items: 0,
                ..ItemizationLimits::default()
            },
            &|| false
        ),
        Err(TextError::Limit(_))
    ));
    let n = std::cell::Cell::new(0);
    itemize_with_languages(&q, &spans, ItemizationLimits::default(), &|| {
        n.set(n.get() + 1);
        false
    })
    .unwrap();
    for stop in 1..=n.get() {
        let at = std::cell::Cell::new(0);
        assert!(matches!(
            itemize_with_languages(&q, &spans, ItemizationLimits::default(), &|| {
                at.set(at.get() + 1);
                at.get() == stop
            }),
            Err(TextError::Cancelled)
        ));
    }
}
