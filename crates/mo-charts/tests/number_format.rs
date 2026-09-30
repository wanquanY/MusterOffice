use mo_charts::{DecimalNumber, number_format::*};
use std::cell::Cell;
fn symbols() -> NumberSymbols {
    NumberSymbols {
        decimal_separator: ".".into(),
        group_separator: ",".into(),
    }
}
fn decimal(s: &str) -> DecimalNumber {
    s.to_owned().try_into().unwrap()
}
fn text(display: &NumberDisplay) -> String {
    display
        .fragments
        .iter()
        .map(|f| match f {
            NumberFragment::Text { value } => value.as_str(),
            _ => panic!("requires measured layout"),
        })
        .collect()
}
fn run(value: &str, code: &str) -> NumberDisplay {
    Formatter::new(&symbols(), Default::default(), &|| false)
        .unwrap()
        .decimal(&decimal(value), code)
        .unwrap()
}
#[test]
fn exact_decimal_rounding_scaling_grouping_and_overflow_are_not_binary_floats() {
    for (value, code, expected) in [
        ("2.675", "0.00", "2.68"),
        ("1.005", "0.00", "1.01"),
        ("-2.675", "0.00", "-2.68"),
        ("-0.004", "0.00", "0.00"),
        ("-0.005", "0.00", "-0.01"),
        ("0", "#", ""),
        ("0", "#.##", "."),
        ("12345.678", "#,##0.00", "12,345.68"),
        ("999.995", "#,##0.00", "1,000.00"),
        ("12.2", "0000.000", "0012.200"),
        ("12.2", "0.###", "12.2"),
        ("12", "0.0##", "12.0"),
        (
            "123456789012345678.125",
            "#,##0.00",
            "123,456,789,012,345,678.13",
        ),
        ("12200000", "0.0,,\"M\"", "12.2M"),
        ("12000", "0,", "12"),
        ("12000", "0,.0", "12.0"),
        ("0.005", "0.0%", "0.5%"),
        ("0.12345", "0.00%", "12.35%"),
        ("0.0001", "0%%", "1%%"),
        ("123", "000-00-0000", "000-00-0123"),
        ("-0.0", "0;[Red](0)", "0"),
        ("12", "0\";#%\"", "12;#%"),
        ("12", "0\\%", "12%"),
        ("12.345", "0.00\" 千元\"", "12.35 千元"),
    ] {
        assert_eq!(text(&run(value, code)), expected, "{value} {code}");
    }
    assert!(run("2.675", "0.00").rounded);
    assert!(!run("2.67", "0.00").rounded);
}
#[test]
fn positive_negative_zero_sections_color_and_empty_sections_are_preserved() {
    let code = "0.0;[Red](0.0);\"—\";@";
    let positive = run("2", code);
    assert_eq!(positive.section, 0);
    assert_eq!(text(&positive), "2.0");
    let negative = run("-2", code);
    assert_eq!(negative.section, 1);
    assert_eq!(negative.color, Some(NumberFormatColor::Red));
    assert_eq!(text(&negative), "(2.0)");
    let zero = run("0", code);
    assert_eq!(zero.section, 2);
    assert_eq!(text(&zero), "—");
    assert_eq!(text(&run("0", "0.0;[Red](0.0)")), "0.0");
    for v in ["12", "-12", "0"] {
        assert!(run(v, ";;;").fragments.is_empty());
    }
    assert_eq!(text(&run("-5", "0;")), "");
}
#[test]
fn scientific_and_engineering_notation_round_carry_and_extreme_exponents() {
    for (value, code, expected) in [
        ("12200000", "0.00E+00", "1.22E+07"),
        ("0.0012345", "0.00E-00", "1.23E-03"),
        ("12345678", "##0.0E+0", "12.3E+6"),
        ("0.00012345", "##0.0E+0", "123.5E-6"),
        ("999.95", "##0.0E+0", "1.0E+3"),
        ("9.995", "0.00E+00", "1.00E+01"),
        ("1e-4096", "0.00E+00", "1.00E-4096"),
        ("1e4096", "0.0e-0", "1.0e4096"),
        ("0", "0.00E+00", "0.00E+00"),
        ("-0.001", "0.0E+0", "-1.0E-3"),
        ("12", "0.0E+##", "1.2E+01"),
        ("0.00123", "0.0%E+0", "1.2%E-1"),
    ] {
        assert_eq!(text(&run(value, code)), expected, "{value} {code}");
    }
}
#[test]
fn question_placeholders_spacing_and_fill_remain_measurable_fragments() {
    let r = run("7", "??0.0??_)*.");
    assert_eq!(
        r.fragments,
        vec![
            NumberFragment::Reserve { glyph: '0' },
            NumberFragment::Reserve { glyph: '0' },
            NumberFragment::Text {
                value: "7.0".into()
            },
            NumberFragment::Reserve { glyph: '0' },
            NumberFragment::Reserve { glyph: '0' },
            NumberFragment::Reserve { glyph: ')' },
            NumberFragment::Fill { glyph: '.' }
        ]
    );
    let r = run("1.20", "0.?#0");
    assert_eq!(text(&r), "1.200");
    assert_eq!(
        run("1", "0*.* ").fragments,
        vec![
            NumberFragment::Text { value: "1".into() },
            NumberFragment::Fill { glyph: ' ' }
        ]
    );
    let symbols = NumberSymbols {
        decimal_separator: ",".into(),
        group_separator: "\u{202f}".into(),
    };
    let mut f = Formatter::new(&symbols, Default::default(), &|| false).unwrap();
    assert_eq!(
        text(&f.decimal(&decimal("1234.5"), "#,##0.00").unwrap()),
        "1\u{202f}234,50"
    );
}
#[test]
fn percentages_use_exact_source_ratio_and_never_sector_angles() {
    let symbols = symbols();
    let mut f = Formatter::new(&symbols, Default::default(), &|| false).unwrap();
    for (n, d, code, expected) in [
        ("1", "3", "0.00%", "33.33%"),
        ("2", "3", "0.00%", "66.67%"),
        ("1", "8", "0%", "13%"),
        ("1", "10000000000", "0.00000000%", "0.00000001%"),
        (
            "123456789012345678901",
            "1000000000000000000000",
            "0.000000000000000000%",
            "12.345678901234567890%",
        ),
    ] {
        assert_eq!(text(&f.ratio(n, d, code).unwrap()), expected);
    }
    assert!(matches!(f.ratio("1", "0", "0%"), Err(FormatError::Ratio)));
    assert!(matches!(f.ratio("-1", "2", "0%"), Err(FormatError::Ratio)));
}
#[test]
fn general_keeps_exact_small_values_and_requires_layout_for_precision_decisions() {
    for (value, expected) in [
        ("  +001.2300e2 ", "123"),
        ("-0", "0"),
        ("48", "48"),
        ("-0.00125", "-0.00125"),
    ] {
        assert_eq!(text(&run(value, "General")), expected);
    }
    for value in ["123456789012", "0.00000000001", "1.00000000001"] {
        let e = Formatter::new(&symbols(), Default::default(), &|| false)
            .unwrap()
            .decimal(&decimal(value), "General")
            .unwrap_err();
        assert!(matches!(
            e,
            FormatError::Unresolved(NumberFormatIssue {
                reason: NumberFormatReason::GeneralNeedsLayout,
                ..
            })
        ));
    }
}
#[test]
fn unsupported_semantics_are_located_in_original_code_and_not_replaced() {
    for (code, reason, offset) in [
        ("[Red][<0]0", NumberFormatReason::ConditionalSection, 5),
        ("[$-804]0", NumberFormatReason::LocaleDirective, 0),
        ("[Color3]0", NumberFormatReason::IndexedColor, 0),
        ("yyyy-mm-dd", NumberFormatReason::DateTime, 0),
        ("# ?/?", NumberFormatReason::Fraction, 3),
        ("0@", NumberFormatReason::TextPlaceholder, 1),
        ("\"公斤\"0x", NumberFormatReason::UnsupportedToken, 9),
    ] {
        let e = Formatter::new(&symbols(), Default::default(), &|| false)
            .unwrap()
            .decimal(&decimal("12"), code)
            .unwrap_err();
        assert!(
            matches!(e,FormatError::Unresolved(ref i) if i.reason==reason && i.offset==offset),
            "{e}"
        );
    }
    for code in [
        "0.0.0",
        "0E+",
        "0E+0E+0",
        "0;0;0;@;0",
        "\"bad",
        "0\\",
        "0,,0",
    ] {
        assert!(
            Formatter::new(&symbols(), Default::default(), &|| false)
                .unwrap()
                .decimal(&decimal("12"), code)
                .is_err(),
            "{code}"
        );
    }
}
#[test]
fn all_work_budgets_are_shared_across_cached_formats_and_cancel_is_polled() {
    let symbols = symbols();
    let limits = FormatLimits {
        max_formats: 1,
        ..Default::default()
    };
    let mut f = Formatter::new(&symbols, limits, &|| false).unwrap();
    f.decimal(&decimal("1"), "0").unwrap();
    f.decimal(&decimal("2"), "0").unwrap();
    assert!(matches!(
        f.decimal(&decimal("2"), "0.0"),
        Err(FormatError::Limit(_))
    ));
    for limits in [
        FormatLimits {
            max_output_bytes: 33,
            ..Default::default()
        },
        FormatLimits {
            max_total_numeric_digits: 12,
            ..Default::default()
        },
    ] {
        let mut f = Formatter::new(&symbols, limits, &|| false).unwrap();
        f.decimal(&decimal("1"), "0").unwrap();
        let mut failed = false;
        for _ in 0..20 {
            if matches!(f.decimal(&decimal("1"), "0"), Err(FormatError::Limit(_))) {
                failed = true;
                break;
            }
        }
        assert!(failed);
    }
    let cancel = Cell::new(false);
    let check = || cancel.get();
    let mut f = Formatter::new(&symbols, Default::default(), &check).unwrap();
    f.decimal(&decimal("1"), "0").unwrap();
    cancel.set(true);
    assert!(matches!(
        f.decimal(&decimal("2"), "0"),
        Err(FormatError::Cancelled)
    ));
    assert!(
        Formatter::new(
            &NumberSymbols {
                decimal_separator: ".".into(),
                group_separator: ".".into()
            },
            Default::default(),
            &|| false
        )
        .is_err()
    );
}
