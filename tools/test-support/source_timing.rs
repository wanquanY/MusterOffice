//! Owned native timing XML without renderer or package dependencies.
pub fn timing(target: u32, from: i32, to: i32, fill: &str) -> String {
    format!(
        "<p:timing><p:tnLst><p:par><p:cTn id=\"1\" dur=\"indefinite\" restart=\"never\" nodeType=\"tmRoot\"><p:childTnLst><p:animRot from=\"{from}\" to=\"{to}\"><p:cBhvr additive=\"repl\" accumulate=\"none\" xfrmType=\"pt\"><p:cTn id=\"2\" dur=\"1000\" repeatCount=\"1000\" restart=\"never\" fill=\"{fill}\"><p:stCondLst><p:cond delay=\"0\"/></p:stCondLst></p:cTn><p:tgtEl><p:spTgt spid=\"{target}\"/></p:tgtEl><p:attrNameLst><p:attrName>r</p:attrName></p:attrNameLst></p:cBhvr></p:animRot></p:childTnLst></p:cTn></p:par></p:tnLst></p:timing>"
    )
}
pub fn visibility_timing(target: u32, value: &str, fill: &str) -> String {
    timing(target, 0, 0, fill)
        .replace("<p:animRot from=\"0\" to=\"0\">", "<p:set>")
        .replace("<p:cond delay=\"0\"/>", "<p:cond delay=\"500\"/>")
        .replace(
            "<p:attrName>r</p:attrName>",
            "<p:attrName>style.visibility</p:attrName>",
        )
        .replace(
            "</p:animRot>",
            &format!("<p:to><p:strVal val=\"{value}\"/></p:to></p:set>"),
        )
}
