//! Raster corpus extends layout probes with visible multipath ink.
#[allow(dead_code)]
#[path = "radial_layout.rs"]
mod radial;
pub use radial::*;
pub fn elliptic_cases() -> Vec<RadialCase> {
    let mut cases = radial_cases();
    let original = cases.iter().find(|c| c.name == "multi-path").unwrap();
    let source = rewrite(&original.source, SLIDE, |s| {
        let needle = "<a:lnTo><a:pt x=\"100000\" y=\"400000\"/></a:lnTo>";
        assert!(s.contains(needle));
        s.replacen(
            needle,
            &format!("<a:lnTo><a:pt x=\"100000\" y=\"200000\"/></a:lnTo>{needle}"),
            1,
        )
        .replace(
            "l=\"10%\" t=\"30%\" r=\"80%\" b=\"20%\"",
            "l=\"50%\" t=\"50%\" r=\"50%\" b=\"50%\"",
        )
    });
    cases.push(RadialCase {
        name: "multi-path-visible".into(),
        source,
        targets: original.targets.clone(),
    });
    cases
}
