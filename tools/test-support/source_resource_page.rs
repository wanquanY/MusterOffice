//! Original native image/shape/text page probes; no application-owned assets.
#[allow(dead_code)]
#[path = "source_text_page.rs"]
mod text;
use mo_pptx::source::images::ImageSourceSelection;
use mo_presentation_compile::source_resource_page::ResourcePageOptions;
use mo_raster::ImageSampling;
pub use text::*;
#[path = "source_image_resources.rs"]
mod images;
pub const PNG: &[u8] = images::PNG;
pub const CYAN: &[u8] = images::CYAN;
pub fn options() -> ResourcePageOptions {
    ResourcePageOptions {
        selection: ImageSourceSelection::EmbeddedSnapshot,
        sampling: ImageSampling::Nearest,
        text_limits: Default::default(),
    }
}
pub fn blip(id: &str, extra: &str, mode: &str) -> String {
    format!("<a:blipFill {extra}><a:blip r:embed=\"{id}\"/>{mode}</a:blipFill>")
}
pub const STRETCH: &str = "<a:stretch><a:fillRect/></a:stretch>";
pub fn picture(id: u32, extra: &str, fill: &str, shape_fill: &str) -> String {
    let fill = fill.replace("a:blipFill", "p:blipFill");
    format!(
        "<p:pic><p:nvPicPr><p:cNvPr id=\"{id}\" name=\"Owned picture\"/><p:cNvPicPr/><p:nvPr/></p:nvPicPr>{fill}<p:spPr><a:xfrm {extra}><a:off x=\"0\" y=\"0\"/><a:ext cx=\"800000\" cy=\"800000\"/></a:xfrm><a:prstGeom prst=\"rect\"/>{shape_fill}<a:ln><a:noFill/></a:ln></p:spPr></p:pic>"
    )
}
pub fn solid(color: &str) -> String {
    format!("<a:solidFill><a:srgbClr val=\"{color}\"/></a:solidFill>")
}
pub fn image_fixture(shapes: &str) -> Vec<u8> {
    images::with_image_resources(&fixture(shapes))
}

pub fn drawing_shape(id: u32, fill: &str) -> String {
    let s = shape(id, 0, 0, "", "");
    let end = s.find("<p:txBody>").unwrap();
    (s[..end].to_owned() + "</p:sp>").replace(&solid("F4EADC"), fill)
}
pub fn background_image(bytes: &[u8]) -> Vec<u8> {
    rewrite(bytes, SLIDE, |s| {
        s.replace(
            &format!("<p:bgPr>{}</p:bgPr>", solid("FFFFFF")),
            &format!("<p:bgPr>{}</p:bgPr>", blip("owned-copy", "", STRETCH)),
        )
    })
}
