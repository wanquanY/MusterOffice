use super::*;
use std::cell::Cell;
fn input(text: &str) -> BidiParagraphRequest {
    BidiParagraphRequest {
        text: text.into(),
        direction: ParagraphDirection::LeftToRight,
        line_ends: vec![],
    }
}
fn analyze(q: &BidiParagraphRequest) -> BidiParagraphResult {
    analyze_paragraph(q, BidiLimits::default(), &|| false).unwrap()
}
#[test]
fn line_resets_are_separate_from_paragraph_resolution() {
    let mut q = input("אב  גד");
    q.line_ends = vec![4, 6];
    let r = analyze(&q);
    assert_eq!(r.resolved_levels, vec![Some(1); 6]);
    assert_eq!(r.lines[0].levels, vec![Some(1), Some(1), Some(0), Some(0)]);
    assert_eq!(r.lines[0].visual_order, vec![1, 0, 2, 3]);
    assert_eq!(r.lines[1].visual_order, vec![5, 4]);
    assert_eq!(
        r.lines[1].start,
        TextBoundary {
            scalar_offset: 4,
            utf8_offset: 6,
            utf16_offset: 4
        }
    );
}
#[test]
fn scalar_coordinates_preserve_supplementary_text_and_original_order() {
    let mut q = input("😀אA");
    q.direction = ParagraphDirection::RightToLeft;
    q.line_ends = vec![1, 3];
    let r = analyze(&q);
    assert_eq!(
        r.lines[1].start,
        TextBoundary {
            scalar_offset: 1,
            utf8_offset: 4,
            utf16_offset: 2
        }
    );
    assert_eq!(
        r.lines[1].end,
        TextBoundary {
            scalar_offset: 3,
            utf8_offset: 7,
            utf16_offset: 4
        }
    );
    assert_eq!(r.lines[1].visual_order, vec![2, 1]);
    assert_eq!(q.text, "😀אA");
}
#[test]
fn formatting_removal_is_an_index_mapping_not_content_deletion() {
    let q = input("A\u{202e}bc\u{202c}Z");
    let r = analyze(&q);
    assert_eq!(
        r.lines[0].levels,
        vec![Some(0), None, Some(1), Some(1), None, Some(0)]
    );
    assert_eq!(r.lines[0].visual_order, vec![0, 3, 2, 5]);
    assert_eq!(q.text.chars().count(), 6);
    assert!(analyze(&input("")).lines[0].visual_order.is_empty());
    assert_eq!(
        analyze(&input("א\r\n")).lines[0].levels,
        vec![Some(1), Some(0), Some(0)]
    );
}
#[test]
fn unicode18_brackets_and_cross_utf16_mirroring_are_explicit() {
    let r = analyze(&input("א \u{2e62}א A\u{2e63}"));
    assert_eq!(
        r.lines[0].levels,
        vec![
            Some(1),
            Some(0),
            Some(0),
            Some(1),
            Some(0),
            Some(0),
            Some(0)
        ]
    );
    assert_eq!(bracket('\u{2329}').unwrap().normalized_opening, 0x3008);
    assert_eq!(bracket('\u{232a}').unwrap().normalized_opening, 0x3008);
    assert_eq!(bidi_properties('\u{221d}').mirroring_glyph, Some(0x1db10));
    assert_eq!(bidi_properties('\u{1db10}').mirroring_glyph, Some(0x221d));
    assert!(bidi_properties('\u{2231}').mirrored);
    assert_eq!(bidi_properties('\u{2231}').mirroring_glyph, None);
}
#[test]
fn invalid_paragraphs_and_line_boundaries_are_rejected() {
    for q in [
        input("a\nb"),
        BidiParagraphRequest {
            line_ends: vec![1, 2],
            ..input("a\u{301}")
        },
        BidiParagraphRequest {
            line_ends: vec![1, 2],
            ..input("\r\n")
        },
        BidiParagraphRequest {
            line_ends: vec![0, 1],
            ..input("a")
        },
        BidiParagraphRequest {
            line_ends: vec![1, 1, 2],
            ..input("ab")
        },
        BidiParagraphRequest {
            line_ends: vec![1],
            ..input("ab")
        },
        BidiParagraphRequest {
            line_ends: vec![3],
            ..input("ab")
        },
    ] {
        assert!(matches!(
            analyze_paragraph(&q, BidiLimits::default(), &|| false),
            Err(BidiError::Invalid(_))
        ));
    }
}
#[test]
fn budgets_and_all_cancellation_checkpoints_are_effective() {
    for limits in [
        BidiLimits {
            max_scalars: 0,
            ..BidiLimits::default()
        },
        BidiLimits {
            max_bytes: 0,
            ..BidiLimits::default()
        },
        BidiLimits {
            max_lines: 0,
            ..BidiLimits::default()
        },
    ] {
        assert!(matches!(
            analyze_paragraph(&input("a"), limits, &|| false),
            Err(BidiError::Limit(_))
        ));
    }
    let q = input("A\u{2067}א(b)\u{2069}🙂");
    let count = Cell::new(0);
    analyze_paragraph(&q, BidiLimits::default(), &|| {
        count.set(count.get() + 1);
        false
    })
    .unwrap();
    for stop in 1..=count.get() {
        let calls = Cell::new(0);
        assert!(matches!(
            analyze_paragraph(&q, BidiLimits::default(), &|| {
                calls.set(calls.get() + 1);
                calls.get() >= stop
            }),
            Err(BidiError::Cancelled)
        ));
    }
}

#[test]
fn prepared_ranges_keep_paragraph_resolution_and_allow_terminal_empty_line() {
    let text = "A\u{2067}אב 12\u{2069}Z";
    let q = BidiParagraphRequest {
        text: text.into(),
        direction: ParagraphDirection::LeftToRight,
        line_ends: vec![1, 5, 9],
    };
    let full = analyze_paragraph(&q, BidiLimits::default(), &|| false).unwrap();
    let prepared =
        ResolvedParagraph::prepare(text, q.direction, BidiLimits::default(), &|| false).unwrap();
    assert_eq!(prepared.resolved_levels(), full.resolved_levels);
    let middle = prepared.line(1, 5, &|| false).unwrap();
    assert_eq!(middle.levels, full.lines[1].levels);
    assert_eq!(middle.visual_order, full.lines[1].visual_order);
    let empty = prepared.line(9, 9, &|| false).unwrap();
    assert!(empty.levels.is_empty());
    assert!(empty.visual_order.is_empty());
    assert_eq!(empty.start, empty.end);
}
#[test]
fn prepared_ranges_reject_split_graphemes_and_cancel_without_result() {
    let p = ResolvedParagraph::prepare(
        "A\u{301}B",
        ParagraphDirection::LeftToRight,
        BidiLimits::default(),
        &|| false,
    )
    .unwrap();
    for (start, end) in [(0, 1), (1, 3), (3, 2), (0, 4)] {
        assert!(matches!(
            p.line(start, end, &|| false),
            Err(BidiError::Invalid(_))
        ));
    }
    assert!(matches!(p.line(0, 3, &|| true), Err(BidiError::Cancelled)));
}
