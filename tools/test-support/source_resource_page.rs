//! Original native image/shape/text page probes; no application-owned assets.
#[allow(dead_code)]
#[path = "source_text_page.rs"]
mod text;
use mo_opc::{Package, PackageBuilder, PackageLimits, PartName, Relationship, RelationshipSource};
use mo_pptx::source::images::ImageSourceSelection;
use mo_presentation_compile::source_resource_page::ResourcePageOptions;
use mo_raster::ImageSampling;
pub use text::*;
pub const PNG: &[u8] = include_bytes!("../../fixtures/presentations/resource-page/transparent.png");
pub const CYAN: &[u8] = include_bytes!("../../fixtures/presentations/resource-page/cyan.png");
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
    let bytes = fixture(shapes);
    let source = Package::open(
        bytes.as_slice(),
        bytes.len() as u64,
        PackageLimits::default(),
        &|| false,
    )
    .unwrap();
    let mut builder = PackageBuilder::new();
    for (part, info) in source.parts() {
        if mo_opc::relationship_source(part).unwrap().is_some() {
            continue;
        }
        builder
            .add_part(
                part.clone(),
                info.content_type.clone(),
                source.read_part(part, 1 << 20, &|| false).unwrap(),
            )
            .unwrap();
    }
    for (n, bytes) in [
        ("owned-image", PNG),
        ("owned-copy", PNG),
        ("owned-cyan", CYAN),
    ] {
        builder
            .add_part(
                PartName::new(format!("/ppt/media/{n}.png")).unwrap(),
                "image/png".into(),
                bytes,
            )
            .unwrap();
    }
    for (part, relations) in source.relationships() {
        let mut relations = relations.clone();
        // Give every drawing surface its own declaring relationship owner so
        // master/layout inheritance and useBgFill can be probed independently.
        if matches!(part, RelationshipSource::Part(p) if p.as_str().ends_with(".xml")) {
            for id in ["owned-image", "owned-copy", "owned-cyan"] {
                relations.push(
                    Relationship::new(
                        part,
                        id.into(),
                        "http://schemas.openxmlformats.org/officeDocument/2006/relationships/image"
                            .into(),
                        format!("/ppt/media/{id}.png"),
                        false,
                    )
                    .unwrap(),
                );
            }
            relations.push(
                Relationship::new(
                    part,
                    "owned-external".into(),
                    "http://schemas.openxmlformats.org/officeDocument/2006/relationships/image"
                        .into(),
                    "https://example.invalid/explicit-image.png".into(),
                    true,
                )
                .unwrap(),
            );
        }
        builder.set_relationships(part.clone(), relations).unwrap();
    }
    builder
        .to_bytes(PackageLimits::default(), &|| false)
        .unwrap()
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
