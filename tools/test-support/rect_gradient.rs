//! Owned native rectangular gradients; source declarations remain editable.
#[allow(dead_code)]
#[path = "source_gradient_page.rs"]
mod gradient_fixture;
pub use gradient_fixture::*;
pub fn rectangular(rect: &str, tile: &str, stops: &str) -> String {
    format!(
        "<a:gradFill><a:gsLst>{stops}</a:gsLst><a:path path=\"rect\"><a:fillToRect {rect}/></a:path>{tile}</a:gradFill>"
    )
}
pub fn rect_cases() -> Vec<GradientCase> {
    let mut cases = Vec::new();
    for (name, rect, tile, stops) in [
        (
            "center-point",
            "l=\"50000\" t=\"50000\" r=\"50000\" b=\"50000\"",
            "",
            STOPS,
        ),
        (
            "off-center",
            "l=\"80000\" t=\"25000\" r=\"20000\" b=\"75000\"",
            "",
            STOPS,
        ),
        (
            "focus-line",
            "l=\"15000\" t=\"20000\" r=\"85000\" b=\"20000\"",
            "",
            STOPS,
        ),
        (
            "outset",
            "l=\"25000\" t=\"75000\" r=\"25000\" b=\"-25000\"",
            "",
            STOPS,
        ),
        (
            "focus-area",
            "l=\"10000\" t=\"30000\" r=\"80000\" b=\"20000\"",
            "",
            STOPS,
        ),
        (
            "zero-edge",
            "l=\"25000\" t=\"25000\" r=\"25000\" b=\"0\"",
            "",
            STOPS,
        ),
        ("whole-focus", "l=\"0\" t=\"0\" r=\"0\" b=\"0\"", "", STOPS),
        (
            "three-percent",
            "l=\"3000\" t=\"3000\" r=\"3000\" b=\"3000\"",
            "",
            STOPS,
        ),
        (
            "tiny-margin",
            "l=\"0.0000000001%\" t=\"50%\" r=\"50%\" b=\"50%\"",
            "",
            STOPS,
        ),
        (
            "decimal-point",
            "l=\"33.333333333333333333%\" t=\"50%\" r=\"66.666666666666666667%\" b=\"50%\"",
            "",
            STOPS,
        ),
        (
            "tiled",
            "l=\"20%\" t=\"30%\" r=\"25%\" b=\"30%\"",
            "<a:tileRect l=\"25%\" t=\"25%\" r=\"25%\" b=\"25%\"/>",
            STOPS,
        ),
        (
            "alpha",
            "l=\"50%\" t=\"50%\" r=\"50%\" b=\"50%\"",
            "",
            "<a:gs pos=\"0\"><a:srgbClr val=\"FF0000\"><a:alpha val=\"0\"/></a:srgbClr></a:gs><a:gs pos=\"100000\"><a:srgbClr val=\"0000FF\"/></a:gs>",
        ),
        (
            "hard-stops",
            "l=\"50%\" t=\"50%\" r=\"50%\" b=\"50%\"",
            "",
            "<a:gs pos=\"0\"><a:srgbClr val=\"FF0000\"/></a:gs><a:gs pos=\"40000\"><a:srgbClr val=\"FF0000\"/></a:gs><a:gs pos=\"40000\"><a:srgbClr val=\"0000FF\"/></a:gs><a:gs pos=\"100000\"><a:srgbClr val=\"0000FF\"/></a:gs>",
        ),
    ] {
        let fill = rectangular(rect, tile, stops);
        cases.push(GradientCase {
            name: name.into(),
            source: image_fixture(&receiver(42, [100000, 200000, 1200000, 600000], "", &fill)),
            equivalent: None,
        });
    }
    let fill = rectangular("l=\"25%\" t=\"25%\" r=\"25%\" b=\"0\"", "", STOPS);
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
    cases.push(GradientCase {
        name: "group-inherited".into(),
        source: image_fixture(&group(90, &xf, &fill, &children)),
        equivalent: Some(image_fixture(&group(
            90,
            &xf,
            &fill,
            &children.replace(INHERIT, &fill),
        ))),
    });
    cases.push(GradientCase {
        name: "rotated-ellipse".into(),
        source: image_fixture(
            &receiver(
                42,
                [100000, 200000, 1200000, 600000],
                "rot=\"1800000\"",
                &fill,
            )
            .replace("prst=\"rect\"", "prst=\"ellipse\""),
        ),
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
    cases.push(GradientCase {
        name: "background".into(),
        source: background(""),
        equivalent: None,
    });
    cases.push(GradientCase {
        name: "background-window".into(),
        source: background(
            &(receiver(42, [0, 0, 1600000, 1200000], "", &solid("00AA00")) + &window),
        ),
        equivalent: None,
    });
    cases
}
