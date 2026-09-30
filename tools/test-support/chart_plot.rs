#![allow(dead_code)]
#[path = "../../crates/mo-presentation-source/tests/support/charts.rs"]
mod charts;
use charts::*;
use mo_opc::{Package, PackageLimits};
use serde_json::{Value, json};
pub fn with_theme(bytes: &[u8], theme: &str, map: &str) -> Vec<u8> {
    use mo_opc::{PackageBuilder, PartName, Relationship, RelationshipSource};
    let p = Package::open(bytes, bytes.len() as u64, PackageLimits::default(), &|| {
        false
    })
    .unwrap();
    let mut b = PackageBuilder::new();
    for (part, info) in p.parts() {
        if part.as_str().ends_with(".rels") {
            continue;
        }
        let mut contents = p.read_part(part, 1 << 24, &|| false).unwrap();
        if part.as_str() == "/ppt/charts/chart1.xml" {
            contents = String::from_utf8(contents)
                .unwrap()
                .replace("<c:chart>", &format!("{map}<c:chart>"))
                .into_bytes();
        }
        b.add_part(part.clone(), info.content_type.clone(), contents)
            .unwrap();
    }
    let part = PartName::new("/ppt/theme/owned-chart.xml").unwrap();
    b.add_part(
        part.clone(),
        "application/vnd.openxmlformats-officedocument.themeOverride+xml".into(),
        theme.as_bytes().to_vec(),
    )
    .unwrap();
    let mut rels = p.relationships().clone();
    let owner = RelationshipSource::Part(PartName::new("/ppt/charts/chart1.xml").unwrap());
    rels.entry(owner.clone()).or_default().push(
        Relationship::new(
            &owner,
            "owned-theme".into(),
            format!("{R}/themeOverride"),
            part.to_string(),
            false,
        )
        .unwrap(),
    );
    for (owner, rels) in rels {
        b.set_relationships(owner, rels).unwrap();
    }
    b.to_bytes(PackageLimits::default(), &|| false).unwrap()
}
pub const BASE: &str = r#"<a:solidFill><a:srgbClr val="2468AC"/></a:solidFill><a:ln w="20000" cap="flat"><a:solidFill><a:srgbClr val="F9F9F9"/></a:solidFill><a:prstDash val="solid"/><a:round/></a:ln><a:effectLst/>"#;
pub fn fixture(style: &str, points: &str, values: &[&str]) -> Vec<u8> {
    let data = values
        .iter()
        .enumerate()
        .rev()
        .map(|(i, v)| format!(r#"<c:pt idx="{i}"><c:v>{v}</c:v></c:pt>"#))
        .collect::<String>();
    let xml = format!(
        r#"<c:chartSpace xmlns:c="{C}" xmlns:a="{A}"><c:chart><c:plotArea><c:doughnutChart><c:varyColors val="0"/><c:ser><c:idx val="7"/><c:order val="0"/><c:spPr>{style}</c:spPr>{points}<c:val><c:numLit><c:ptCount val="{}"/>{data}</c:numLit></c:val></c:ser><c:firstSliceAng val="0"/><c:holeSize val="50"/></c:doughnutChart></c:plotArea></c:chart></c:chartSpace>"#,
        values.len()
    );
    package(&xml, &frame(2), CT, "chart", false)
}
pub fn point(index: u32, properties: &str) -> String {
    format!(r#"<c:dPt><c:idx val="{index}"/><c:spPr>{properties}</c:spPr></c:dPt>"#)
}
pub fn request(bytes: &[u8]) -> Value {
    let p = Package::open(bytes, bytes.len() as u64, PackageLimits::default(), &|| {
        false
    })
    .unwrap();
    let index = mo_pptx::source::inspect_source(&p, Default::default(), &|| false).unwrap();
    let source = mo_pptx::source::charts::query(
        &p,
        &index,
        &mo_pptx::source::charts::SourceChartQuery {
            expected_source_sha256: p.sha256().clone(),
            surface: "/ppt/slides/slide1.xml".into(),
        },
        Default::default(),
        Default::default(),
        &|| false,
    )
    .unwrap();
    json!({
        "profile":"source-circular-declared-solid-plot-v1-draft",
        "geometry":{"expectedSourceSha256":p.sha256(),"object":{"part":"/ppt/slides/slide1.xml","nativeId":2},
            "plotSourceOrdinal":source.charts[0].plots[0].source_ordinal,"profile":"source-cache-declared-circular-plot-v1-draft",
            "center":{"x":"0","y":"0"},"outerRadius":(1_000_000i128<<32).to_string(),
            "coordinateTolerance":(1i128<<32).to_string(),"negativeWeights":"reject"},
        "colorContext":{"systemColors":{},"placeholder":null}
    })
}
pub fn cases() -> Vec<(String, Vec<u8>, &'static str)> {
    let mut result = vec![];
    for (name, local, status) in [
        ("series", "", "compiled"),
        (
            "point-fill",
            r#"<a:solidFill><a:srgbClr val="D4713D"/></a:solidFill>"#,
            "compiled",
        ),
        ("empty-solid-inherits", "<a:solidFill/>", "compiled"),
        ("point-no-fill", "<a:noFill/>", "compiled"),
        ("point-line-no-fill", "<a:ln><a:noFill/></a:ln>", "compiled"),
        ("point-width", r#"<a:ln w="60000"/>"#, "compiled"),
        (
            "point-line-color",
            r#"<a:ln><a:solidFill><a:srgbClr val="862040"/></a:solidFill></a:ln>"#,
            "compiled",
        ),
        (
            "empty-line-solid",
            "<a:ln><a:solidFill/></a:ln>",
            "compiled",
        ),
        ("point-bevel", "<a:ln><a:bevel/></a:ln>", "compiled"),
        (
            "point-alpha",
            r#"<a:solidFill><a:srgbClr val="D4713D"><a:alpha val="50000"/></a:srgbClr></a:solidFill>"#,
            "compiled",
        ),
        (
            "unknown-geometry",
            r#"<a:prstGeom prst="rect"><a:avLst/></a:prstGeom>"#,
            "error",
        ),
        (
            "point-dash",
            r#"<a:ln><a:prstDash val="dash"/></a:ln>"#,
            "error",
        ),
        ("point-double", r#"<a:ln cmpd="dbl"/>"#, "error"),
        ("point-alignment", r#"<a:ln algn="in"/>"#, "error"),
        (
            "point-miter",
            r#"<a:ln><a:miter lim="800000"/></a:ln>"#,
            "error",
        ),
        (
            "point-arrow",
            r#"<a:ln><a:headEnd type="triangle"/></a:ln>"#,
            "error",
        ),
        (
            "point-shadow",
            r#"<a:effectLst><a:outerShdw blurRad="100"><a:srgbClr val="000000"/></a:outerShdw></a:effectLst>"#,
            "error",
        ),
        ("empty-dag", "<a:effectDag/>", "error"),
        (
            "missing-system-color",
            r#"<a:solidFill><a:sysClr val="window"/></a:solidFill>"#,
            "error",
        ),
        (
            "pattern",
            r#"<a:pattFill prst="pct20"><a:fgClr><a:srgbClr val="193A62"/></a:fgClr><a:bgClr><a:srgbClr val="FFFFFF"/></a:bgClr></a:pattFill>"#,
            "error",
        ),
    ] {
        result.push((
            name.into(),
            fixture(BASE, &point(1, local), &["48", "32", "20"]),
            status,
        ));
    }
    for (name, style, values, status) in [
        ("zero-point",BASE.to_string(), vec!["0","1","2"],"compiled"),
        ("full-ring",BASE.to_string(), vec!["1","0","0"],"compiled"),
        ("all-zero",BASE.to_string(), vec!["0","0","0"],"compiled"),
        ("no-line",BASE.replace(r#"<a:ln w="20000" cap="flat"><a:solidFill><a:srgbClr val="F9F9F9"/></a:solidFill><a:prstDash val="solid"/><a:round/></a:ln>"#, "<a:ln><a:noFill/></a:ln>"),vec!["1","2","3"],"compiled"),
        ("automatic-fill",BASE.replace(r#"<a:solidFill><a:srgbClr val="2468AC"/></a:solidFill>"#,""),vec!["1","2","3"],"error"),
        ("automatic-width",BASE.replace(r#" w="20000""#,""),vec!["1","2","3"],"error"),
        ("automatic-effects",BASE.replace("<a:effectLst/>",""),vec!["1","2","3"],"error"),
        ("automatic-line",r#"<a:solidFill><a:srgbClr val="2468AC"/></a:solidFill><a:effectLst/>"#.into(),vec!["1","2","3"],"error"),
    ] {
        result.push((name.into(),fixture(&style,"",&values),status));
    }
    let scheme = [
        "dk1", "lt1", "dk2", "lt2", "accent1", "accent2", "accent3", "accent4", "accent5",
        "accent6", "hlink", "folHlink",
    ]
    .iter()
    .map(|n| {
        format!(
            r#"<a:{n}><a:srgbClr val="{}"/></a:{n}>"#,
            if *n == "accent1" { "193A62" } else { "804020" }
        )
    })
    .collect::<String>();
    let theme = format!(
        r#"<a:themeOverride xmlns:a="{A}"><a:clrScheme name="Owned">{scheme}</a:clrScheme></a:themeOverride>"#
    );
    let map = r#"<c:clrMapOvr bg1="lt1" tx1="dk1" bg2="lt2" tx2="dk2" accent1="accent2" accent2="accent1" accent3="accent3" accent4="accent4" accent5="accent5" accent6="accent6" hlink="hlink" folHlink="folHlink"/>"#;
    let base=BASE.replace(r#"<a:srgbClr val="2468AC"/>"#,r#"<a:schemeClr val="accent1"><a:lumMod val="50000"/><a:lumOff val="10000"/></a:schemeClr>"#);
    result.push((
        "inherited-chart-theme".into(),
        with_theme(
            &fixture(&base, &point(1, "<a:solidFill/>"), &["48", "32", "20"]),
            &theme,
            map,
        ),
        "compiled",
    ));
    result
}
