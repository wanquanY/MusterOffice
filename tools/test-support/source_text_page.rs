//! Owned source-page fixtures, shared by library and runtime boundary tests.
#[allow(dead_code)]
#[path = "source_glyphs.rs"]
mod glyphs;
pub use glyphs::*;
use mo_geometry::{Fixed, Point};
use mo_presentation_compile::source_page::*;
use mo_raster::{PixelScale, RasterViewport};
use mo_text::manifest::ManifestParagraphRequest;
pub fn colored(text: &str, color: &str) -> String {
    format!(
        "<a:r><a:rPr><a:solidFill><a:srgbClr val=\"{color}\"/></a:solidFill></a:rPr><a:t>{text}</a:t></a:r>"
    )
}
pub fn shape(id: u32, x: i64, y: i64, extra: &str, runs: &str) -> String {
    format!(
        "<p:sp><p:nvSpPr><p:cNvPr id=\"{id}\" name=\"Owned native text\"/><p:cNvSpPr/><p:nvPr/></p:nvSpPr><p:spPr><a:xfrm {extra}><a:off x=\"{x}\" y=\"{y}\"/><a:ext cx=\"1000000\" cy=\"600000\"/></a:xfrm><a:prstGeom prst=\"rect\"/><a:solidFill><a:srgbClr val=\"F4EADC\"/></a:solidFill><a:ln><a:noFill/></a:ln></p:spPr><p:txBody><a:bodyPr lIns=\"0\" rIns=\"0\" tIns=\"0\" bIns=\"0\"><a:noAutofit/></a:bodyPr><a:lstStyle><a:lvl1pPr><a:defRPr sz=\"3000\" lang=\"en\"><a:latin typeface=\"{FONT}\"/></a:defRPr></a:lvl1pPr></a:lstStyle><a:p>{runs}</a:p></p:txBody></p:sp>"
    )
}
pub fn fixture(shapes: &str) -> Vec<u8> {
    let b = bytes(&run("A"));
    let b = rewrite(&b, SLIDE, |s| {
        let start = s.find("<p:cSld").unwrap();
        let end = s.find("</p:cSld>").unwrap() + "</p:cSld>".len();
        let mut s = s;
        s.replace_range(start..end,&format!("<p:cSld><p:bg><p:bgPr><a:solidFill><a:srgbClr val=\"FFFFFF\"/></a:solidFill></p:bgPr></p:bg><p:spTree><p:nvGrpSpPr><p:cNvPr id=\"1\" name=\"\"/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr><p:grpSpPr/>{shapes}</p:spTree></p:cSld>"));
        s.replacen("<p:sld ", "<p:sld showMasterSp=\"0\" ", 1)
    });
    rewrite(&b, "/ppt/presentation.xml", |mut s| {
        let start = s.find("<p:sldSz ").unwrap();
        let end = start + s[start..].find("/>").unwrap() + 2;
        s.replace_range(start..end, "<p:sldSz cx=\"1600000\" cy=\"1200000\"/>");
        s
    })
}
/// Build a genuinely missing-style case even when the writer supplies defaults.
pub fn without_default_text_style(source: &[u8]) -> Vec<u8> {
    rewrite(source, "/ppt/presentation.xml", |mut xml| {
        let start = xml.find("<p:defaultTextStyle>").unwrap();
        let end = start
            + xml[start..].find("</p:defaultTextStyle>").unwrap()
            + "</p:defaultTextStyle>".len();
        xml.replace_range(start..end, "");
        xml
    })
}
pub fn request(i: &mo_pptx::source::SourceIndex) -> SourcePageRequest {
    SourcePageRequest {
        expected_source_sha256: i.source_sha256.clone(),
        slide: SLIDE.into(),
        profile: SourcePageProfile::StaticSolidDraftV1,
        color_context: Default::default(),
        viewport: RasterViewport {
            width: 400,
            height: 300,
            origin: Point {
                x: Fixed::ZERO,
                y: Fixed::ZERO,
            },
            scale: PixelScale {
                numerator: 1,
                denominator: 4000,
            },
            coordinate_tolerance: Fixed::from_raw(1 << 24),
            background: [255; 4],
        },
    }
}
pub fn author() -> ManifestParagraphRequest {
    serde_json::from_str(include_str!("../../fixtures/fonts/manifest-paragraph.json")).unwrap()
}
