//! Owned native linear gradients. The cases retain editable DrawingML fills.
#[allow(dead_code)]
#[path = "source_group_images.rs"]
mod base;
pub use base::*;
pub struct GradientCase {
    pub name: String,
    pub source: Vec<u8>,
    pub equivalent: Option<Vec<u8>>,
}
pub fn linear(angle: u32, scaled: bool, extra: &str, tile: &str, stops: &str) -> String {
    format!(
        "<a:gradFill {extra}><a:gsLst>{stops}</a:gsLst><a:lin ang=\"{angle}\" scaled=\"{}\"/>{tile}</a:gradFill>",
        u8::from(scaled)
    )
}
pub const STOPS: &str = "<a:gs pos=\"0\"><a:srgbClr val=\"FF0000\"/></a:gs><a:gs pos=\"100000\"><a:srgbClr val=\"0000FF\"/></a:gs>";
pub fn cases() -> Vec<GradientCase> {
    let mut out = Vec::new();
    for (name, angle, scaled, extra, tile, stops) in [
        ("right", 0, false, "", "", STOPS),
        ("down", 5400000, false, "", "", STOPS),
        ("left", 10800000, false, "", "", STOPS),
        ("up", 16200000, false, "", "", STOPS),
        ("scaled-30", 1800000, true, "", "", STOPS),
        ("unscaled-30", 1800000, false, "", "", STOPS),
        ("scaled-225", 13500000, true, "", "", STOPS),
        (
            "tile-none",
            2700000,
            true,
            "flip=\"none\"",
            "<a:tileRect l=\"25000\" t=\"25000\" r=\"25000\" b=\"25000\"/>",
            STOPS,
        ),
        (
            "tile-x",
            2700000,
            true,
            "flip=\"x\"",
            "<a:tileRect l=\"25000\" t=\"25000\" r=\"25000\" b=\"25000\"/>",
            STOPS,
        ),
        (
            "tile-xy",
            2700000,
            true,
            "flip=\"xy\"",
            "<a:tileRect l=\"25000\" t=\"25000\" r=\"25000\" b=\"25000\"/>",
            STOPS,
        ),
        (
            "hard-stop",
            1800000,
            true,
            "",
            "",
            "<a:gs pos=\"0\"><a:srgbClr val=\"FF0000\"/></a:gs><a:gs pos=\"40000\"><a:srgbClr val=\"FF0000\"/></a:gs><a:gs pos=\"40000\"><a:srgbClr val=\"0000FF\"/></a:gs><a:gs pos=\"100000\"><a:srgbClr val=\"0000FF\"/></a:gs>",
        ),
        (
            "alpha",
            0,
            false,
            "",
            "",
            "<a:gs pos=\"0\"><a:srgbClr val=\"FF0000\"><a:alpha val=\"0\"/></a:srgbClr></a:gs><a:gs pos=\"100000\"><a:srgbClr val=\"0000FF\"/></a:gs>",
        ),
    ] {
        let fill = linear(angle, scaled, extra, tile, stops);
        out.push(GradientCase {
            name: name.into(),
            source: image_fixture(&receiver(42, [100000, 200000, 1200000, 600000], "", &fill)),
            equivalent: None,
        });
    }
    let fill = linear(1800000, true, "", "", STOPS);
    let children = receiver(
        42,
        [100000, 200000, 700000, 600000],
        "rot=\"1800000\"",
        INHERIT,
    ) + &receiver(43, [850000, 250000, 400000, 600000], "flipH=\"1\"", INHERIT);
    let xf = transform(
        "rot=\"1200000\" flipV=\"1\"",
        [180000, 100000, 1300000, 1000000],
        [100000, -50000, 1600000, 1000000],
    );
    out.push(GradientCase {
        name: "group-inherited".into(),
        source: image_fixture(&group(90, &xf, &fill, &children)),
        equivalent: Some(image_fixture(&group(
            90,
            &xf,
            &fill,
            &children.replace(INHERIT, &fill),
        ))),
    });
    let shape = receiver(
        42,
        [100000, 200000, 1200000, 600000],
        "rot=\"1800000\"",
        &fill,
    )
    .replace("prst=\"rect\"", "prst=\"ellipse\"");
    out.push(GradientCase {
        name: "rotated-ellipse".into(),
        source: image_fixture(&shape),
        equivalent: None,
    });
    let background = |shapes: &str| {
        rewrite(&image_fixture(shapes), SLIDE, |s| {
            s.replacen(
                &format!("<p:bgPr>{}</p:bgPr>", solid("FFFFFF")),
                &format!("<p:bgPr>{fill}</p:bgPr>"),
                1,
            )
        })
    };
    let window = receiver(43, [200000, 200000, 1000000, 600000], "", "<a:noFill/>").replacen(
        "<p:sp>",
        "<p:sp useBgFill=\"1\">",
        1,
    );
    out.push(GradientCase {
        name: "background".into(),
        source: background(""),
        equivalent: None,
    });
    out.push(GradientCase {
        name: "background-window".into(),
        source: background(
            &(receiver(42, [0, 0, 1600000, 1200000], "", &solid("00AA00")) + &window),
        ),
        equivalent: None,
    });
    out
}
