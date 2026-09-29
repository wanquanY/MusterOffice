//! Owned generic Set fixtures. No entrance/exit preset or implicit initial state.
#[allow(dead_code)]
#[path = "source_playback.rs"]
mod playback;
pub use playback::*;
pub fn visibility(source: &[u8], target: u32, value: &str, fill: &str) -> Vec<u8> {
    let timing = visibility_timing(target, value, fill);
    rewrite(source, SLIDE, |s| {
        s.replace("</p:sld>", &format!("{timing}</p:sld>"))
    })
}
pub fn hidden(source: &[u8], target: u32) -> Vec<u8> {
    rewrite(source, SLIDE, |s| {
        s.replace(
            &format!("<p:cNvPr id=\"{target}\""),
            &format!("<p:cNvPr hidden=\"1\" id=\"{target}\""),
        )
    })
}
