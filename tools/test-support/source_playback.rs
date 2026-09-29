//! Owned native rotation fixtures, shared by computation boundary tests.
#[allow(dead_code)]
#[path = "source_group_images.rs"]
mod group_fixtures;
pub use group_fixtures::*;
#[path = "source_timing.rs"]
mod source_timing;
pub use source_timing::*;

pub fn animated(source: &[u8], target: u32, from: i32, to: i32, fill: &str) -> Vec<u8> {
    rewrite(source, SLIDE, |s| {
        s.replace(
            "</p:sld>",
            &format!("{}</p:sld>", timing(target, from, to, fill)),
        )
    })
}

pub fn scaled(source: &[u8], target: u32, from: [u32; 2], to: [u32; 2], fill: &str) -> Vec<u8> {
    let timing = timing(target, 0, 0, fill)
        .replace("<p:animRot from=\"0\" to=\"0\">", "<p:animScale>")
        .replace(
            "<p:attrName>r</p:attrName>",
            "<p:attrName>ScaleX</p:attrName><p:attrName>ScaleY</p:attrName>",
        )
        .replace(
            "</p:animRot>",
            &format!(
                "<p:from x=\"{}\" y=\"{}\"/><p:to x=\"{}\" y=\"{}\"/></p:animScale>",
                from[0], from[1], to[0], to[1]
            ),
        );
    rewrite(source, SLIDE, |s| {
        s.replace("</p:sld>", &format!("{timing}</p:sld>"))
    })
}
