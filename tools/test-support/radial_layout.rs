//! Owned source geometry for radial layout; no preview screenshot substitutions.
#[allow(dead_code)]
#[path = "rect_gradient.rs"]
mod rect_fixture;
use mo_pptx::source::fill::resolve::FillTarget;
pub use rect_fixture::*;

pub struct RadialCase {
    pub name: String,
    pub source: Vec<u8>,
    pub targets: Vec<FillTarget>,
}
pub fn circle(rect: &str) -> String {
    rectangular(rect, "", STOPS).replace("path=\"rect\"", "path=\"circle\"")
}
pub fn custom(paths: &str) -> String {
    format!(
        "<a:custGeom><a:avLst/><a:gdLst/><a:ahLst/><a:cxnLst/><a:rect l=\"0\" t=\"0\" r=\"w\" b=\"h\"/><a:pathLst>{paths}</a:pathLst></a:custGeom>"
    )
}
pub fn quadratic() -> String {
    custom(
        "<a:path><a:moveTo><a:pt x=\"100000\" y=\"100000\"/></a:moveTo><a:quadBezTo><a:pt x=\"500000\" y=\"900000\"/><a:pt x=\"900000\" y=\"100000\"/></a:quadBezTo><a:close/></a:path>",
    )
}
pub fn radial_cases() -> Vec<RadialCase> {
    let mut out = vec![];
    for c in rect_cases() {
        let targets = if c.name.starts_with("background") {
            vec![FillTarget::Background {}]
        } else if c.name == "group-inherited" {
            vec![
                FillTarget::Object { native_id: 42 },
                FillTarget::Object { native_id: 43 },
            ]
        } else {
            vec![FillTarget::Object { native_id: 42 }]
        };
        let rewrite_circle =
            |b: &[u8]| rewrite(b, SLIDE, |s| s.replace("path=\"rect\"", "path=\"circle\""));
        out.push(RadialCase {
            name: c.name.clone(),
            source: rewrite_circle(&c.source),
            targets: targets.clone(),
        });
        if let Some(control) = c.equivalent {
            out.push(RadialCase {
                name: c.name + "-control",
                source: rewrite_circle(&control),
                targets,
            });
        }
    }
    let fill = circle("l=\"10%\" t=\"30%\" r=\"80%\" b=\"20%\"");
    for preset in ["blockArc", "chord", "teardrop", "wedgeRoundRectCallout"] {
        let shape = receiver(42, [100000, 200000, 1200000, 600000], "", &fill)
            .replace("prst=\"rect\"", &format!("prst=\"{preset}\""));
        out.push(RadialCase {
            name: preset.into(),
            source: image_fixture(&shape),
            targets: vec![FillTarget::Object { native_id: 42 }],
        });
    }
    let cubic = custom(
        "<a:path><a:moveTo><a:pt x=\"0\" y=\"0\"/></a:moveTo><a:cubicBezTo><a:pt x=\"0\" y=\"800000\"/><a:pt x=\"1000000\" y=\"800000\"/><a:pt x=\"1000000\" y=\"0\"/></a:cubicBezTo><a:close/></a:path>",
    );
    let multi = custom(
        "<a:path><a:moveTo><a:pt x=\"-100000\" y=\"200000\"/></a:moveTo><a:lnTo><a:pt x=\"100000\" y=\"400000\"/></a:lnTo><a:close/></a:path><a:path fill=\"none\"><a:moveTo><a:pt x=\"400000\" y=\"-200000\"/></a:moveTo><a:lnTo><a:pt x=\"700000\" y=\"500000\"/></a:lnTo></a:path><a:path><a:moveTo><a:pt x=\"9000000000\" y=\"9000000000\"/></a:moveTo></a:path>",
    );
    for (name, geometry) in [
        ("quadratic", quadratic()),
        ("cubic", cubic),
        ("multi-path", multi),
    ] {
        out.push(RadialCase {
            name: name.into(),
            source: image_fixture(
                &receiver(
                    42,
                    [100000, 200000, 1200000, 600000],
                    "rot=\"1800000\"",
                    &fill,
                )
                .replace("<a:prstGeom prst=\"rect\"/>", &geometry),
            ),
            targets: vec![FillTarget::Object { native_id: 42 }],
        });
    }
    for (name, rect, extra) in [
        (
            "equal-width-offset",
            "l=\"10%\" t=\"25%\" r=\"-10%\" b=\"25%\"",
            "",
        ),
        (
            "expanded-focus",
            "l=\"-50%\" t=\"-25%\" r=\"-50%\" b=\"-25%\"",
            "",
        ),
        (
            "near-equal-width",
            "l=\"10%\" t=\"25%\" r=\"-9.999999999999999999%\" b=\"25%\"",
            "",
        ),
        (
            "stationary",
            "l=\"50%\" t=\"50%\" r=\"50%\" b=\"50%\"",
            "rotWithShape=\"0\"",
        ),
    ] {
        let fill = circle(rect).replace("<a:gradFill>", &format!("<a:gradFill {extra}>"));
        out.push(RadialCase {
            name: name.into(),
            source: image_fixture(&receiver(
                42,
                [100000, 200000, 1200000, 600000],
                "rot=\"1800000\"",
                &fill,
            )),
            targets: vec![FillTarget::Object { native_id: 42 }],
        });
    }
    out
}
