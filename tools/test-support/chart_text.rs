#![allow(dead_code)]
#[path = "chart_labels.rs"]
mod labels;
use labels::*;
use serde_json::Value;
pub fn cases() -> Vec<(String, Vec<u8>, Value, &'static str)> {
    let local = "<c:txPr><a:bodyPr/><a:p><a:pPr algn=\"r\"><a:defRPr sz=\"1200\" b=\"0\"><a:noFill/><a:latin typeface=\"Point Font\"/></a:defRPr></a:pPr></a:p></c:txPr>";
    let parent = "<c:txPr><a:bodyPr/><a:p><a:pPr rtl=\"0\"><a:defRPr sz=\"1800\" b=\"1\" i=\"1\"><a:solidFill><a:srgbClr val=\"FF0000\"/></a:solidFill><a:latin typeface=\"Group Font\"/><a:ea typeface=\"+mn-ea\"/></a:defRPr></a:pPr></a:p></c:txPr>";
    let rich = "<c:tx><c:rich><a:bodyPr/><a:p><a:pPr><a:defRPr i=\"0\"/></a:pPr><a:r><a:rPr sz=\"1600\"/><a:t>中文</a:t></a:r><a:br/><a:r><a:t>42</a:t></a:r><a:endParaRPr b=\"1\"/></a:p></c:rich></c:tx>";
    let mut out = vec![];
    for (name, local, inherited, expected) in [
        ("text-whole-slot", local.to_string(), parent, "cascaded"),
        (
            "text-rich-runs",
            format!("{rich}{local}"),
            parent,
            "cascaded",
        ),
        (
            "text-no-shape-default",
            "<c:txPr><a:bodyPr/><a:p/></c:txPr>".into(),
            "",
            "cascaded",
        ),
        (
            "text-list-style",
            local.replace("<a:p>", "<a:lstStyle><a:lvl1pPr/></a:lstStyle><a:p>"),
            parent,
            "unresolved",
        ),
        (
            "text-multiple-properties",
            local.replace("</c:txPr>", "<a:p/></c:txPr>"),
            parent,
            "unresolved",
        ),
        (
            "text-property-runs",
            local.replace("</a:p>", "<a:r><a:t>kept</a:t></a:r></a:p>"),
            parent,
            "unresolved",
        ),
        (
            "text-unknown-attribute",
            local.replace("sz=\"1200\"", "sz=\"1200\" future=\"1\""),
            parent,
            "unresolved",
        ),
    ] {
        let bytes = package(&xml(
            "doughnutChart",
            &series(
                7,
                &["1", "2", "3"],
                &group(
                    &point(0, &local),
                    &format!("{inherited}{}", flags(true, false, false)),
                ),
            ),
            "",
        ));
        let q = request(&bytes);
        out.push((name.into(), bytes, q, expected));
    }
    let base = xml(
        "doughnutChart",
        &series(7, &["1", "2", "3"], &group("", &flags(true, false, false))),
        "",
    );
    let bytes = package(&base.replace("</c:chartSpace>", &format!("{parent}</c:chartSpace>")));
    out.push((
        "text-chart-space".into(),
        bytes.clone(),
        request(&bytes),
        "cascaded",
    ));
    out
}
