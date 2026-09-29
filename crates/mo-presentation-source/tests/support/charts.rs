#![allow(dead_code)]
use mo_opc::{PackageBuilder, PackageLimits, PartName, Relationship, RelationshipSource};
pub const P: &str = "http://schemas.openxmlformats.org/presentationml/2006/main";
pub const A: &str = "http://schemas.openxmlformats.org/drawingml/2006/main";
pub const C: &str = "http://schemas.openxmlformats.org/drawingml/2006/chart";
pub const R: &str = "http://schemas.openxmlformats.org/officeDocument/2006/relationships";
pub const CT: &str = "application/vnd.openxmlformats-officedocument.drawingml.chart+xml";
pub const WORKBOOK_TYPE: &str = "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet";
pub fn frame(id: u32) -> String {
    format!(
        r#"<p:graphicFrame><p:nvGraphicFramePr><p:cNvPr id="{id}" name="Chart {id}"/><p:cNvGraphicFramePr/><p:nvPr/></p:nvGraphicFramePr><p:xfrm><a:off x="0" y="0"/><a:ext cx="3000000" cy="2000000"/></p:xfrm><a:graphic><a:graphicData uri="{C}"><c:chart r:id="chart"/></a:graphicData></a:graphic></p:graphicFrame>"#
    )
}
pub fn chart(series: &str) -> String {
    format!(
        r#"<c:chartSpace xmlns:c="{C}" xmlns:a="{A}" xmlns:r="{R}"><c:chart><c:plotArea><c:barChart><c:barDir val="col"/><c:grouping val="clustered"/>{series}<c:axId val="11"/><c:axId val="12"/></c:barChart></c:plotArea></c:chart><c:externalData r:id="data"><c:autoUpdate val="0"/></c:externalData></c:chartSpace>"#
    )
}
pub fn series() -> String {
    r#"<c:ser><c:idx val="0"/><c:order val="0"/><c:tx><c:strRef><c:f>Sheet1!$B$1</c:f><c:strCache><c:ptCount val="1"/><c:pt idx="0"><c:v>收入</c:v></c:pt></c:strCache></c:strRef></c:tx><c:cat><c:multiLvlStrRef><c:f>Sheet1!$A$2:$A$5</c:f><c:multiLvlStrCache><c:ptCount val="4"/><c:lvl><c:pt idx="0"><c:v>华东</c:v></c:pt><c:pt idx="2"><c:v>华南</c:v></c:pt></c:lvl><c:lvl><c:pt idx="0"><c:v>中国</c:v></c:pt></c:lvl></c:multiLvlStrCache></c:multiLvlStrRef></c:cat><c:val><c:numRef><c:f>Sheet1!$B$2:$B$5</c:f><c:numCache><c:formatCode>0.00</c:formatCode><c:ptCount val="4"/><c:pt idx="0"><c:v>0</c:v></c:pt><c:pt idx="1"><c:v/></c:pt><c:pt idx="2"/><c:pt idx="3" formatCode="General"><c:v>#N/A</c:v></c:pt></c:numCache></c:numRef></c:val></c:ser>"#.into()
}
pub fn package(
    chart: &str,
    frames: &str,
    chart_type: &str,
    relationship_type: &str,
    external: bool,
) -> Vec<u8> {
    let mut b = PackageBuilder::new();
    for (part,ty,bytes) in [
        ("/ppt/presentation.xml","application/vnd.openxmlformats-officedocument.presentationml.presentation.main+xml",format!(r#"<p:presentation xmlns:p="{P}" xmlns:r="{R}"><p:sldIdLst><p:sldId id="256" r:id="slide"/></p:sldIdLst><p:sldSz cx="9144000" cy="5143500"/></p:presentation>"#).into_bytes()),
        ("/ppt/slides/slide1.xml","application/vnd.openxmlformats-officedocument.presentationml.slide+xml",format!(r#"<p:sld xmlns:p="{P}" xmlns:a="{A}" xmlns:c="{C}" xmlns:r="{R}"><p:cSld><p:spTree><p:nvGrpSpPr><p:cNvPr id="1" name=""/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr><p:grpSpPr/>{frames}</p:spTree></p:cSld></p:sld>"#).into_bytes()),
        ("/ppt/charts/chart1.xml",chart_type,chart.as_bytes().to_vec()),
        // Deliberately opaque bytes: source inspection must not claim workbook validation.
        ("/ppt/embeddings/data.xlsx",WORKBOOK_TYPE,b"owned-unvalidated-workbook-bytes".to_vec())
    ]{b.add_part(PartName::new(part).unwrap(),ty.into(),bytes).unwrap();}
    for (source, id, kind, target, ext) in [
        (
            RelationshipSource::Package,
            "main",
            "officeDocument",
            "ppt/presentation.xml",
            false,
        ),
        (
            RelationshipSource::Part(PartName::new("/ppt/presentation.xml").unwrap()),
            "slide",
            "slide",
            "slides/slide1.xml",
            false,
        ),
        (
            RelationshipSource::Part(PartName::new("/ppt/slides/slide1.xml").unwrap()),
            "chart",
            relationship_type,
            "../charts/chart1.xml",
            false,
        ),
        (
            RelationshipSource::Part(PartName::new("/ppt/charts/chart1.xml").unwrap()),
            "data",
            "package",
            if external {
                "https://example.invalid/chart-data.xlsx"
            } else {
                "../embeddings/data.xlsx"
            },
            external,
        ),
    ] {
        let rel = Relationship::new(
            &source,
            id.into(),
            format!("{R}/{kind}"),
            target.into(),
            ext,
        )
        .unwrap();
        b.set_relationships(source, vec![rel]).unwrap();
    }
    b.to_bytes(PackageLimits::default(), &|| false).unwrap()
}
