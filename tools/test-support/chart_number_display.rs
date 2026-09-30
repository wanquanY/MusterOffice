#![allow(dead_code)]
#[path = "chart_labels.rs"]
mod labels;
use labels::*;
use serde_json::{Value, json};
pub struct Case {
    pub name: String,
    pub source: Vec<u8>,
    pub request: Value,
    pub expected: [&'static str; 3],
    pub issue: Option<&'static str>,
}
fn attribute(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('"', "&quot;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}
pub fn cases() -> Vec<Case> {
    let mut cases = vec![];
    for (name, code, values, percent, expected, issue) in [
        (
            "general",
            "General",
            ["48", "32", "20"],
            false,
            ["48", "32", "20"],
            None,
        ),
        (
            "fixed",
            "0.00",
            ["2.675", "1.005", "-0.004"],
            false,
            ["2.68", "1.01", "0.00"],
            None,
        ),
        (
            "group",
            "#,##0.00",
            ["1234.5", "999.995", "-1000"],
            false,
            ["1,234.50", "1,000.00", "-1,000.00"],
            None,
        ),
        (
            "scale",
            "0.0,,\"M\"",
            ["12200000", "1000000", "-1500000"],
            false,
            ["12.2M", "1.0M", "-1.5M"],
            None,
        ),
        (
            "percent",
            "0.00%",
            ["0.12345", "0.005", "2.1"],
            false,
            ["12.35%", "0.50%", "210.00%"],
            None,
        ),
        (
            "ratio",
            "0.00%",
            ["1", "2", "0"],
            true,
            ["33.33%", "66.67%", "0.00%"],
            None,
        ),
        (
            "ratio-tie",
            "0%",
            ["1", "6", "1"],
            true,
            ["13%", "75%", "13%"],
            None,
        ),
        (
            "ratio-small",
            "0.00000000%",
            ["1", "9999999999", "0"],
            true,
            ["0.00000001%", "99.99999999%", "0.00000000%"],
            None,
        ),
        (
            "scientific",
            "0.00E+00",
            ["9.995", "0.001234", "0"],
            false,
            ["1.00E+01", "1.23E-03", "0.00E+00"],
            None,
        ),
        (
            "engineering",
            "##0.0E+0",
            ["12345678", "0.00012345", "999.95"],
            false,
            ["12.3E+6", "123.5E-6", "1.0E+3"],
            None,
        ),
        (
            "scientific-extreme",
            "0.0e-0",
            ["1e4096", "1e-4096", "-1e-300"],
            false,
            ["1.0e4096", "1.0e-4096", "-1.0e-300"],
            None,
        ),
        (
            "large-exact",
            "#,##0.00",
            ["123456789012345678.125", "100000000000000000.005", "-0.005"],
            false,
            [
                "123,456,789,012,345,678.13",
                "100,000,000,000,000,000.01",
                "-0.01",
            ],
            None,
        ),
        (
            "sections",
            "0.0;[Red](0.0);\"—\";@",
            ["2", "-2", "0"],
            false,
            ["2.0", "(2.0)", "—"],
            None,
        ),
        ("empty", ";;;", ["2", "-2", "0"], false, ["", "", ""], None),
        (
            "literal",
            "0\";#% 公斤\"",
            ["1", "2", "3"],
            false,
            ["1;#% 公斤", "2;#% 公斤", "3;#% 公斤"],
            None,
        ),
        (
            "zero-optional",
            "#.##",
            ["0", "0.5", "1.2"],
            false,
            [".", ".5", "1.2"],
            None,
        ),
        (
            "digit-mask",
            "000-00-0000",
            ["123", "123456789", "0"],
            false,
            ["000-00-0123", "123-45-6789", "000-00-0000"],
            None,
        ),
        (
            "general-layout",
            "General",
            ["123456789012", "0.00000000001", "1.00000000001"],
            false,
            ["", "", ""],
            Some("generalNeedsLayout"),
        ),
        (
            "condition",
            "[Red][<0]0",
            ["-1", "1", "0"],
            false,
            ["", "", ""],
            Some("conditionalSection"),
        ),
        (
            "date",
            "yyyy-mm-dd",
            ["1", "2", "3"],
            false,
            ["", "", ""],
            Some("dateTime"),
        ),
        (
            "fraction",
            "# ?/?",
            ["1.5", "2.25", "3"],
            false,
            ["", "", ""],
            Some("fraction"),
        ),
        (
            "locale",
            "[$-804]0",
            ["1", "2", "3"],
            false,
            ["", "", ""],
            Some("localeDirective"),
        ),
        (
            "invalid",
            "0.0.0",
            ["1", "2", "3"],
            false,
            ["", "", ""],
            Some("invalidSyntax"),
        ),
    ] {
        let fmt = format!(
            r#"<c:numFmt formatCode="{}" sourceLinked="0"/>"#,
            attribute(code)
        );
        let source = package(&xml(
            "doughnutChart",
            &series(
                7,
                &values,
                &group("", &format!("{fmt}{}", flags(!percent, false, percent))),
            ),
            "",
        ));
        let mut q = request(&source);
        q["numberSymbols"] = json!({"decimalSeparator":".","groupSeparator":","});
        cases.push(Case {
            name: name.into(),
            source,
            request: q,
            expected,
            issue,
        });
    }
    let mut localized = cases[2].request.clone();
    localized["numberSymbols"] = json!({"decimalSeparator":",","groupSeparator":"\u{202f}"});
    cases.push(Case {
        name: "explicit-symbols".into(),
        source: cases[2].source.clone(),
        request: localized,
        expected: ["1\u{202f}234,50", "1\u{202f}000,00", "-1\u{202f}000,00"],
        issue: None,
    });
    cases
}
