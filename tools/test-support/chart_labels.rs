#![allow(dead_code)]
#[path = "../../crates/mo-presentation-source/tests/support/charts.rs"]
mod charts;
pub use charts::{A, C};
use mo_opc::{Package, PackageLimits};
use serde_json::{Value, json};
pub fn flags(value: bool, category: bool, percent: bool) -> String {
    format!(
        r#"<c:showLegendKey val="0"/><c:showVal val="{}"/><c:showCatName val="{}"/><c:showSerName val="0"/><c:showPercent val="{}"/><c:showBubbleSize val="0"/>"#,
        u8::from(value),
        u8::from(category),
        u8::from(percent)
    )
}
pub fn tx(size: u32) -> String {
    format!(
        r#"<c:txPr><a:bodyPr/><a:p><a:pPr><a:defRPr sz="{size}"><a:latin typeface="Arial"/></a:defRPr></a:pPr></a:p></c:txPr>"#
    )
}
pub fn group(points: &str, settings: &str) -> String {
    format!("<c:dLbls>{points}{settings}</c:dLbls>")
}
pub fn point(idx: u32, settings: &str) -> String {
    format!(r#"<c:dLbl><c:idx val="{idx}"/>{settings}</c:dLbl>"#)
}
pub const GENERAL: &str = r#"<c:numFmt formatCode="General" sourceLinked="0"/>"#;
pub fn series(index: u32, values: &[&str], labels: &str) -> String {
    let values = values
        .iter()
        .enumerate()
        .rev()
        .map(|(i, v)| format!(r#"<c:pt idx="{i}"><c:v>{v}</c:v></c:pt>"#))
        .collect::<String>();
    format!(
        r#"<c:ser><c:idx val="{index}"/><c:order val="{index}"/><c:tx><c:v>收入</c:v></c:tx>{labels}<c:cat><c:strLit><c:ptCount val="3"/><c:pt idx="0"><c:v>华东</c:v></c:pt><c:pt idx="1"><c:v>华南</c:v></c:pt><c:pt idx="2"><c:v>海外</c:v></c:pt></c:strLit></c:cat><c:val><c:numLit><c:formatCode>0.000</c:formatCode><c:ptCount val="3"/>{values}</c:numLit></c:val></c:ser>"#
    )
}
pub fn xml(kind: &str, series: &str, plot_labels: &str) -> String {
    format!(
        r#"<c:chartSpace xmlns:c="{C}" xmlns:a="{A}"><c:chart><c:plotArea><c:{kind}>{series}{plot_labels}</c:{kind}></c:plotArea></c:chart></c:chartSpace>"#
    )
}
pub fn package(xml: &str) -> Vec<u8> {
    charts::package(xml, &charts::frame(2), charts::CT, "chart", false)
}
pub fn request(bytes: &[u8]) -> Value {
    let p = Package::open(bytes, bytes.len() as u64, PackageLimits::default(), &|| {
        false
    })
    .unwrap();
    json!({"profile":"source-chart-label-bindings-v1-draft","expectedSourceSha256":p.sha256(),"object":{"part":"/ppt/slides/slide1.xml","nativeId":2},"plotSourceOrdinal":3,"negativeWeights":"reject","targets":[{"seriesIndex":7,"pointIndex":0},{"seriesIndex":7,"pointIndex":1},{"seriesIndex":7,"pointIndex":2}]})
}
pub fn actual_like() -> String {
    let points = (0..3)
        .map(|i| {
            point(
                i,
                &format!("{GENERAL}{}{}", tx(1200), flags(true, false, false)),
            )
        })
        .collect::<String>();
    xml(
        "doughnutChart",
        &series(
            7,
            &["48", "32", "20"],
            &group(
                &points,
                &format!("{GENERAL}{}{}", tx(1800), flags(false, true, true)),
            ),
        ),
        "",
    )
}
pub fn wide(count: u32) -> (Vec<u8>, Value) {
    let points = (0..count)
        .rev()
        .map(|idx| {
            point(
                idx,
                if idx % 2 == 0 {
                    "<c:showVal val=\"0\"/>"
                } else {
                    ""
                },
            )
        })
        .collect::<String>();
    let values = (0..count)
        .rev()
        .map(|idx| format!(r#"<c:pt idx="{idx}"><c:v>{idx}</c:v></c:pt>"#))
        .collect::<String>();
    let labels = group(&points, &format!("{GENERAL}{}", flags(true, false, false)));
    let series = format!(
        r#"<c:ser><c:idx val="7"/><c:order val="0"/>{labels}<c:val><c:numLit><c:ptCount val="{count}"/>{values}</c:numLit></c:val></c:ser>"#
    );
    let bytes = package(&xml("doughnutChart", &series, ""));
    let mut q = request(&bytes);
    q["targets"] = json!(
        (0..count)
            .map(|idx| json!({"seriesIndex":7,"pointIndex":idx}))
            .collect::<Vec<_>>()
    );
    (bytes, q)
}
pub fn cases() -> Vec<(String, Vec<u8>, Value, &'static str)> {
    let base = actual_like();
    let full = xml(
        "doughnutChart",
        &series(
            7,
            &["0.1", "0.2", "0.7"],
            &group("", &format!("{GENERAL}{}", flags(true, true, true))),
        ),
        "",
    );
    let pct = xml(
        "pieChart",
        &series(7, &["1", "2", "3"], &group("", &flags(false, true, true))),
        "",
    );
    let sparse = xml(
        "doughnutChart",
        &series(7, &["48", "32", "20"], &group("", "<c:showVal/>")),
        "",
    );
    let unlinked = group(
        &point(1, r#"<c:numFmt formatCode="0.0"/>"#),
        &format!("{GENERAL}{}", flags(true, false, false)),
    );
    let custom = group(
        &point(
            0,
            r#"<c:tx><c:rich><a:bodyPr/><a:p><a:r><a:t>自定义</a:t></a:r></a:p></c:rich></c:tx>"#,
        ),
        &flags(true, false, false),
    );
    let metadata = base.replacen(
        "<c:idx val=\"0\"/>",
        "<c:idx val=\"0\"/><c:layout><c:manualLayout><c:xMode val=\"factor\"/><c:x val=\"0.25\"/></c:manualLayout></c:layout><c:dLblPos val=\"outEnd\"/>",
        1,
    ).replace("</c:dLbls>", "<c:showLeaderLines val=\"true\"/></c:dLbls>");
    let numeric_categories = series(7, &["1", "2", "3"], &group("", &flags(false, true, false))).replace(
        "<c:strLit><c:ptCount val=\"3\"/><c:pt idx=\"0\"><c:v>华东</c:v></c:pt><c:pt idx=\"1\"><c:v>华南</c:v></c:pt><c:pt idx=\"2\"><c:v>海外</c:v></c:pt></c:strLit>",
        "<c:numLit><c:formatCode>0.0</c:formatCode><c:ptCount val=\"3\"/><c:pt idx=\"0\"><c:v>0.1</c:v></c:pt><c:pt idx=\"1\" formatCode=\"0.00\"><c:v>0.2</c:v></c:pt><c:pt idx=\"2\"><c:v>0.3</c:v></c:pt></c:numLit>",
    );
    let bubble_labels = flags(true, true, false)
        .replace("showSerName val=\"0\"", "showSerName val=\"1\"")
        .replace("showBubbleSize val=\"0\"", "showBubbleSize val=\"1\"");
    let bubble = series(7, &["1", "2", "3"], &group("", &bubble_labels))
        .replace("<c:val>", "<c:yVal>").replace("</c:val>", "</c:yVal>")
        .replace("<c:tx><c:v>收入</c:v></c:tx>", "<c:tx><c:strRef><c:f>Sheet1!$A$1</c:f><c:strCache><c:ptCount val=\"1\"/><c:pt idx=\"0\"><c:v>收入</c:v></c:pt></c:strCache></c:strRef></c:tx>")
        .replace("</c:ser>", "<c:bubbleSize><c:numLit><c:formatCode>0.00</c:formatCode><c:ptCount val=\"3\"/><c:pt idx=\"0\"><c:v>4</c:v></c:pt><c:pt idx=\"1\"><c:v>5</c:v></c:pt><c:pt idx=\"2\"><c:v>6</c:v></c:pt></c:numLit></c:bubbleSize></c:ser>");
    let many = vec![
        ("actual-like", base.clone(), "planned"),
        ("label-metadata", metadata, "planned"),
        (
            "numeric-categories",
            xml("doughnutChart", &numeric_categories, ""),
            "planned",
        ),
        (
            "bubble-and-series-reference",
            xml("bubbleChart", &bubble, ""),
            "planned",
        ),
        ("decimal-ratios", full.clone(), "planned"),
        ("pie-separator", pct.clone(), "planned"),
        (
            "doughnut-separator",
            pct.replace("pieChart", "doughnutChart"),
            "planned",
        ),
        (
            "empty-separator",
            pct.replace("</c:dLbls>", "<c:separator/></c:dLbls>"),
            "planned",
        ),
        ("unresolved-flags", sparse.clone(), "planned"),
        (
            "schema-boolean-default",
            sparse.replace("<c:showVal/>", "<c:showVal val=\"1\"/>"),
            "planned",
        ),
        (
            "format-attribute-default",
            xml("doughnutChart", &series(7, &["1", "2", "3"], &unlinked), ""),
            "planned",
        ),
        (
            "custom-rich",
            xml("doughnutChart", &series(7, &["1", "2", "3"], &custom), ""),
            "planned",
        ),
        (
            "plot-inheritance",
            xml(
                "doughnutChart",
                &series(
                    7,
                    &["1", "2", "3"],
                    &group(&point(1, "<c:showVal val=\"0\"/><c:showCatName/>"), GENERAL),
                ),
                &group("", &flags(true, false, false)),
            ),
            "planned",
        ),
        (
            "group-delete",
            xml(
                "doughnutChart",
                &series(
                    7,
                    &["1", "2", "3"],
                    &group(&point(0, "<c:delete val=\"0\"/>"), "<c:delete/>"),
                ),
                &group("", &flags(true, false, false)),
            ),
            "planned",
        ),
        (
            "legend-key-alone",
            xml(
                "doughnutChart",
                &series(
                    7,
                    &["1", "2", "3"],
                    &group(
                        "",
                        &flags(false, false, false)
                            .replace("showLegendKey val=\"0\"", "showLegendKey val=\"1\""),
                    ),
                ),
                "",
            ),
            "planned",
        ),
        (
            "unknown-flag",
            base.replace("<c:showVal val=\"1\"/>", "<c:showVal val=\"unknown\"/>"),
            "error",
        ),
        (
            "unknown-attribute",
            base.replace(
                "<c:showVal val=\"1\"/>",
                "<c:showVal val=\"1\" future=\"1\"/>",
            ),
            "error",
        ),
        (
            "forbidden-plot-format",
            xml(
                "doughnutChart",
                &series(7, &["1", "2", "3"], ""),
                &group("", &format!("{GENERAL}{}", flags(true, false, false))),
            ),
            "error",
        ),
        (
            "unsupported-extension",
            base.replace("</c:dLbls>", "<c:extLst/></c:dLbls>"),
            "error",
        ),
        (
            "negative-reject",
            full.replace("<c:v>0.1</c:v>", "<c:v>-0.1</c:v>"),
            "error",
        ),
        (
            "zero-denominator",
            full.replace("<c:v>0.1</c:v>", "<c:v>0</c:v>")
                .replace("<c:v>0.2</c:v>", "<c:v>0</c:v>")
                .replace("<c:v>0.7</c:v>", "<c:v>0</c:v>"),
            "error",
        ),
        (
            "sparse-denominator",
            full.replace("<c:pt idx=\"1\"><c:v>0.2</c:v></c:pt>", ""),
            "error",
        ),
        ("missing-value", base.replace("<c:v>48</c:v>", ""), "error"),
        (
            "error-value",
            base.replace("<c:v>48</c:v>", "<c:v>#N/A</c:v>"),
            "error",
        ),
        (
            "invalid-format-link",
            base.replace("sourceLinked=\"0\"", "sourceLinked=\"yes\""),
            "error",
        ),
    ];
    let mut out = many
        .into_iter()
        .map(|(name, xml, status)| {
            let bytes = package(&xml);
            let q = request(&bytes);
            (name.into(), bytes, q, status)
        })
        .collect::<Vec<_>>();
    let negative = package(&full.replace("<c:v>0.1</c:v>", "<c:v>-0.1</c:v>"));
    let mut q = request(&negative);
    q["negativeWeights"] = json!("absoluteMagnitude");
    out.push(("negative-absolute".into(), negative, q, "planned"));
    let tiny = package(&full.replace("<c:v>0.1</c:v>", "<c:v>1e-60</c:v>"));
    out.push((
        "sub-angle-resolution".into(),
        tiny.clone(),
        request(&tiny),
        "planned",
    ));
    let bytes = package(&base);
    let mut q = request(&bytes);
    q["targets"][1] = q["targets"][0].clone();
    out.push(("duplicate-target".into(), bytes, q, "error"));
    out
}
