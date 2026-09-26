//! Owned native rotation fixtures, shared by computation boundary tests.
#[allow(dead_code)]
#[path = "source_resource_page.rs"]
mod resource;
pub use resource::*;
pub fn timing(target: u32, from: i32, to: i32, fill: &str) -> String {
    format!(
        "<p:timing><p:tnLst><p:par><p:cTn id=\"1\" dur=\"indefinite\" restart=\"never\" nodeType=\"tmRoot\"><p:childTnLst><p:animRot from=\"{from}\" to=\"{to}\"><p:cBhvr additive=\"repl\" accumulate=\"none\" xfrmType=\"pt\"><p:cTn id=\"2\" dur=\"1000\" repeatCount=\"1000\" restart=\"never\" fill=\"{fill}\"><p:stCondLst><p:cond delay=\"0\"/></p:stCondLst></p:cTn><p:tgtEl><p:spTgt spid=\"{target}\"/></p:tgtEl><p:attrNameLst><p:attrName>r</p:attrName></p:attrNameLst></p:cBhvr></p:animRot></p:childTnLst></p:cTn></p:par></p:tnLst></p:timing>"
    )
}
pub fn animated(source: &[u8], target: u32, from: i32, to: i32, fill: &str) -> Vec<u8> {
    rewrite(source, SLIDE, |s| {
        s.replace(
            "</p:sld>",
            &format!("{}</p:sld>", timing(target, from, to, fill)),
        )
    })
}
