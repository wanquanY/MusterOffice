//! Owned PPTX controls for background restoration, independent of the scene IR.
#[allow(dead_code)]
#[path = "source_group_images.rs"]
mod group;
pub use group::*;

pub struct BackgroundCase {
    pub name: &'static str,
    pub source: Vec<u8>,
    pub background: Vec<u8>,
    pub mask: Vec<u8>,
    pub clear: [u8; 4],
    pub captures: u32,
    pub windows: u32,
    pub images: u32,
}
fn page(shapes: &str, fill: &str) -> Vec<u8> {
    rewrite(&image_fixture(shapes), SLIDE, |s| {
        s.replacen(
            &format!("<p:bgPr>{}</p:bgPr>", solid("FFFFFF")),
            &format!("<p:bgPr>{fill}</p:bgPr>"),
            1,
        )
    })
}
fn window(id: u32, bounds: [i64; 4], orientation: &str) -> String {
    receiver(id, bounds, orientation, "<a:noFill/>").replacen("<p:sp>", "<p:sp useBgFill=\"1\">", 1)
}
pub fn cases() -> Vec<BackgroundCase> {
    let white = solid("FFFFFF");
    let red_alpha =
        "<a:solidFill><a:srgbClr val=\"FF0000\"><a:alpha val=\"50000\"/></a:srgbClr></a:solidFill>";
    let image = blip("owned-image", "", STRETCH);
    let clipped = blip(
        "owned-image",
        "",
        "<a:stretch><a:fillRect l=\"25000\" r=\"25000\"/></a:stretch>",
    );
    let tiled = blip(
        "owned-image",
        "dpi=\"9144\"",
        "<a:tile tx=\"25000\" ty=\"-12500\" sx=\"10000000\" sy=\"10000000\" flip=\"xy\" algn=\"ctr\"/>",
    );
    let foreground = receiver(40, [0, 0, 1600000, 1200000], "", &solid("00AA00"));
    let ordinary = window(42, [100000, 200000, 1000000, 600000], "");
    let mut result = Vec::new();
    for (name, fill, clear, captures, images) in [
        ("opaque-white", white.as_str(), [255; 4], 0, 0),
        ("alpha-opaque", red_alpha, [255; 4], 1, 0),
        ("alpha-clear", red_alpha, [0; 4], 1, 0),
        ("empty-clear", "<a:noFill/>", [0; 4], 1, 0),
        ("image-clear", image.as_str(), [0; 4], 1, 1),
        ("clipped-image", clipped.as_str(), [0; 4], 1, 1),
        ("tiled-image", tiled.as_str(), [0; 4], 1, 1),
        ("rotated-ellipse", image.as_str(), [0; 4], 1, 1),
        ("multiple-windows", image.as_str(), [0; 4], 1, 1),
        ("hidden-window", image.as_str(), [0; 4], 0, 1),
    ] {
        let mut windows = ordinary.clone();
        let mut count = 1;
        if name == "rotated-ellipse" {
            windows = window(42, [100000, 200000, 1000000, 600000], "rot=\"1800000\"")
                .replace("prst=\"rect\"", "prst=\"ellipse\"");
        }
        let mut paints = windows.clone();
        if name == "multiple-windows" {
            let second = window(43, [600000, 300000, 900000, 600000], "");
            let orange = receiver(44, [700000, 400000, 200000, 200000], "", &solid("FF8800"));
            paints += &(orange + &second);
            windows += &second;
            count = 2;
        }
        if name == "hidden-window" {
            paints = paints.replace("id=\"42\"", "hidden=\"1\" id=\"42\"");
            windows.clear();
            count = 0;
        }
        let mask = windows
            .replace(" useBgFill=\"1\"", "")
            .replace("<a:noFill/><a:ln>", &(white.clone() + "<a:ln>"));
        result.push(BackgroundCase {
            name,
            source: page(&(foreground.clone() + &paints), fill),
            background: page("", fill),
            mask: page(&mask, &solid("000000")),
            clear,
            captures,
            windows: count,
            images,
        });
    }
    result
}
