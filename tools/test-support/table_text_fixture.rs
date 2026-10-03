//! Shared original table text fixture; use the parent module's source helpers.
use super::*;
pub fn fixture(bad_cell: bool) -> Vec<u8> {
    let input: serde_json::Value = serde_json::from_str(include_str!(
        "../../fixtures/presentations/native-tables/request.json"
    ))
    .unwrap();
    let bytes = mo_pptx::export(
        &serde_json::from_value(input["document"].clone()).unwrap(),
        &serde_json::from_value(input["defaults"].clone()).unwrap(),
        &mo_pptx::NoResources,
        Default::default(),
        &|| false,
    )
    .unwrap();
    rewrite(&bytes, SLIDE, |mut s| {
        let starts = s
            .match_indices("<a:txBody>")
            .map(|(at, _)| at)
            .collect::<Vec<_>>();
        for (cell, start) in starts.into_iter().enumerate().rev() {
            let end = start + s[start..].find("</a:txBody>").unwrap() + "</a:txBody>".len();
            let paragraphs = if bad_cell && cell == 5 {
                "<a:p><a:r><a:rPr cap=\"all\"/><a:t>A</a:t></a:r></a:p>".into()
            } else if cell == 8 {
                "<a:p/>".into()
            } else {
                "<a:p><a:r><a:t>A A</a:t></a:r></a:p>".repeat(if cell == 4 { 2 } else { 1 })
            };
            s.replace_range(start..end, &format!("<a:txBody><a:bodyPr/><a:lstStyle><a:lvl1pPr><a:defRPr sz=\"2200\" lang=\"en\"/></a:lvl1pPr></a:lstStyle>{paragraphs}</a:txBody>"));
        }
        s.replacen("</a:tblPr>", &format!("<a:tableStyle styleId=\"{{ABCDEF01-2345-6789-ABCD-EF0123456789}}\" styleName=\"Owned\"><a:wholeTbl><a:tcTxStyle><a:font><a:latin typeface=\"{FONT}\"/><a:ea typeface=\"{FONT}\"/><a:cs typeface=\"{FONT}\"/></a:font><a:srgbClr val=\"123456\"/></a:tcTxStyle></a:wholeTbl></a:tableStyle></a:tblPr>"), 1)
    })
}
