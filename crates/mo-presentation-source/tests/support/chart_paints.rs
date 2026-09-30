#![allow(dead_code)]
#[path = "charts.rs"]
pub mod charts;
use charts::*;
use mo_opc::{Package, PackageBuilder, PackageLimits, PartName, Relationship, RelationshipSource};
pub const MAP: &str = r#"bg1="lt1" tx1="dk1" bg2="lt2" tx2="dk2" accent1="accent1" accent2="accent2" accent3="accent3" accent4="accent4" accent5="accent5" accent6="accent6" hlink="hlink" folHlink="folHlink""#;
pub fn theme(red: &str) -> String {
    let mut slots = String::new();
    for name in [
        "dk1", "lt1", "dk2", "lt2", "accent1", "accent2", "accent3", "accent4", "accent5",
        "accent6", "hlink", "folHlink",
    ] {
        let color = if name == "accent1" {
            red
        } else {
            "<a:srgbClr val=\"204080\"/>"
        };
        slots.push_str(&format!("<a:{name}>{color}</a:{name}>"));
    }
    format!(
        r#"<a:themeOverride xmlns:a="{A}"><a:clrScheme name="Owned">{slots}</a:clrScheme></a:themeOverride>"#
    )
}
pub fn fixture(
    styles: &str,
    map: Option<&str>,
    slide_theme: Option<&str>,
    chart_theme: Option<&str>,
) -> Vec<u8> {
    let s = series().replace("</c:ser>", &format!("{styles}</c:ser>"));
    let mut chart = chart(&s);
    if let Some(map) = map {
        chart = chart.replace("<c:chart>", &format!("{map}<c:chart>"));
    }
    let original = package(&chart, &frame(2), CT, "chart", false);
    let p = Package::open(
        original.as_slice(),
        original.len() as u64,
        PackageLimits::default(),
        &|| false,
    )
    .unwrap();
    let mut b = PackageBuilder::new();
    for (name, info) in p.parts() {
        if name.as_str().ends_with(".rels") {
            continue;
        }
        let mut bytes = p.read_part(name, 1 << 24, &|| false).unwrap();
        if name.as_str() == "/ppt/slides/slide1.xml" {
            bytes = String::from_utf8(bytes)
                .unwrap()
                .replace(
                    "</p:sld>",
                    &format!(r#"<p:clrMapOvr><a:overrideClrMapping {MAP}/></p:clrMapOvr></p:sld>"#),
                )
                .into_bytes();
        }
        b.add_part(name.clone(), info.content_type.clone(), bytes)
            .unwrap();
    }
    let mut rels = p.relationships().clone();
    for (owner, xml, file) in [
        ("/ppt/slides/slide1.xml", slide_theme, "slide-theme.xml"),
        ("/ppt/charts/chart1.xml", chart_theme, "chart-theme.xml"),
    ] {
        if let Some(xml) = xml {
            let part = format!("/ppt/theme/{file}");
            b.add_part(
                PartName::new(&part).unwrap(),
                "application/vnd.openxmlformats-officedocument.themeOverride+xml".into(),
                xml.as_bytes().to_vec(),
            )
            .unwrap();
            let source = RelationshipSource::Part(PartName::new(owner).unwrap());
            rels.entry(source.clone()).or_default().push(
                Relationship::new(
                    &source,
                    "owned-theme".into(),
                    format!("{R}/themeOverride"),
                    part,
                    false,
                )
                .unwrap(),
            );
        }
    }
    for (owner, rels) in rels {
        b.set_relationships(owner, rels).unwrap();
    }
    b.to_bytes(PackageLimits::default(), &|| false).unwrap()
}
