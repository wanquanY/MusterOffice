use super::*;
use std::cell::Cell;
fn breaks(text: &str) -> Vec<(u32, BreakKind)> {
    analyze_line_breaks(text, UnicodeLimits::default(), &|| false)
        .unwrap()
        .opportunities
        .iter()
        .map(|b| (b.boundary.scalar_offset, b.kind))
        .collect()
}
#[test]
fn hard_breaks_and_empty_text_have_explicit_semantics() {
    use BreakKind::*;
    assert_eq!(breaks(""), []);
    assert_eq!(
        breaks("a\r\nb\u{85}c\u{2028}d"),
        [
            (3, Mandatory),
            (5, Mandatory),
            (7, Mandatory),
            (8, Mandatory)
        ]
    );
    assert_eq!(breaks("a b"), [(2, Allowed), (3, Mandatory)]);
}
#[test]
fn unicode18_hyphens_and_nonbreaking_spaces_follow_revision57() {
    use BreakKind::*;
    assert_eq!(line_break_properties('\u{ad}').class, Hh);
    assert_eq!(line_break_properties('\u{2013}').class, Ba);
    assert_eq!(breaks("x–\u{a0}y"), [(4, Mandatory)]);
    assert_eq!(breaks("\u{ad}\u{a0}A"), [(1, Allowed), (3, Mandatory)]);
    assert_eq!(breaks("-abc"), [(4, Mandatory)]);
    assert_eq!(breaks("x-abc"), [(2, Allowed), (5, Mandatory)]);
    assert_eq!(breaks("א-ב"), [(2, Allowed), (3, Mandatory)]);
    assert_eq!(breaks("א-x"), [(3, Mandatory)]);
}
#[test]
fn punctuation_numbers_and_emoji_are_not_arbitrarily_split() {
    use BreakKind::*;
    assert_eq!(
        breaks("中（文）文"),
        [(1, Allowed), (4, Allowed), (5, Mandatory)]
    );
    assert_eq!(breaks("$(1,234.50)%"), [(12, Mandatory)]);
    assert_eq!(breaks("🇨🇳🇺🇸"), [(2, Allowed), (4, Mandatory)]);
    assert_eq!(breaks("👩🏽‍💻"), [(4, Mandatory)]);
    assert_eq!(breaks("A\u{2060}中"), [(3, Mandatory)]);
}
#[test]
fn combining_context_and_dictionary_responsibility_are_preserved() {
    use BreakKind::*;
    // Default UAX14 can break inside an extended grapheme after SP. A higher
    // layout profile must explicitly resolve this, not hide a changed default.
    assert_eq!(breaks(" \u{308}A"), [(1, Allowed), (3, Mandatory)]);
    assert_eq!(breaks("\u{200b} \u{308}"), [(2, Allowed), (3, Mandatory)]);
    assert_eq!(breaks("🇨\u{308}🇳🇺\u{308}🇸"), [(3, Allowed), (6, Mandatory)]);
    let r = analyze_line_breaks("ไทย", UnicodeLimits::default(), &|| false).unwrap();
    assert_eq!(r.complex_context_scalars, [0, 1, 2]);
    assert_eq!(r.opportunities.len(), 1);
}
#[test]
fn scalar_utf8_and_utf16_positions_do_not_alias() {
    let r = analyze_line_breaks("😀 中", UnicodeLimits::default(), &|| false).unwrap();
    assert_eq!(
        r.opportunities[0].boundary,
        TextBoundary {
            scalar_offset: 2,
            utf8_offset: 5,
            utf16_offset: 3
        }
    );
    assert_eq!(
        r.end,
        TextBoundary {
            scalar_offset: 3,
            utf8_offset: 8,
            utf16_offset: 4
        }
    );
}
#[test]
fn resource_limits_and_each_cancellation_checkpoint_are_atomic() {
    for limits in [
        UnicodeLimits {
            max_bytes: 1,
            max_scalars: 9,
        },
        UnicodeLimits {
            max_bytes: 99,
            max_scalars: 1,
        },
    ] {
        assert!(matches!(
            analyze_line_breaks("ab", limits, &|| false),
            Err(UnicodeError::Limit(_))
        ));
    }
    let text = "“ ( A\u{301} ) ” 🇨🇳\r\nไทย";
    let n = Cell::new(0);
    analyze_line_breaks(text, UnicodeLimits::default(), &|| {
        n.set(n.get() + 1);
        false
    })
    .unwrap();
    for stop in 1..=n.get() {
        let at = Cell::new(0);
        assert!(matches!(
            analyze_line_breaks(text, UnicodeLimits::default(), &|| {
                at.set(at.get() + 1);
                at.get() == stop
            }),
            Err(UnicodeError::Cancelled)
        ));
    }
    let spaces = format!("({}x", " ".repeat(65534));
    let r = analyze_line_breaks(&spaces, UnicodeLimits::default(), &|| false).unwrap();
    assert_eq!(r.opportunities.len(), 1);
}
