#[allow(dead_code)]
#[path = "source_table.rs"]
pub mod table;
use mo_opc::{
    Package, PackageBuilder, PartName, Relationship, RelationshipSource, relationship_source,
};
use mo_pptx::source::{SourceIndex, inspect_source};
pub const ID: &str = "{ABCDEF01-2345-6789-ABCD-EF0123456789}";
pub const ID2: &str = "{ABCDEF02-2345-6789-ABCD-EF0123456789}";
pub const STYLES: &str = "/ppt/tableStyles.xml";
pub const A: &str = "http://schemas.openxmlformats.org/drawingml/2006/main";
pub const R: &str = "http://schemas.openxmlformats.org/officeDocument/2006/relationships";
pub const TYPE: &str =
    "application/vnd.openxmlformats-officedocument.presentationml.tableStyles+xml";
pub const REGIONS: [&str; 13] = [
    "wholeTbl", "band1H", "band2H", "band1V", "band2V", "lastCol", "firstCol", "lastRow", "seCell",
    "swCell", "firstRow", "neCell", "nwCell",
];
pub fn package(b: &[u8]) -> Package<&[u8]> {
    Package::open(b, b.len() as u64, Default::default(), &|| false).unwrap()
}
pub fn read(b: &[u8]) -> SourceIndex {
    inspect_source(&package(b), Default::default(), &|| false).unwrap()
}
pub fn evidence(name: &str, bytes: &[u8]) {
    if let Some(dir) = std::env::var_os("MO_TABLE_STYLE_EVIDENCE_DIR") {
        let dir = std::path::PathBuf::from(dir);
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join(format!("{name}.pptx"));
        let mut file = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(path)
            .unwrap();
        std::io::Write::write_all(&mut file, bytes).unwrap();
    }
}
pub fn style(root: &str, id: &str, body: &str) -> String {
    format!("<a:{root} styleId=\"{id}\" styleName=\"Owned test\">{body}</a:{root}>")
}
pub fn list(body: &str) -> String {
    format!("<a:tblStyleLst xmlns:a=\"{A}\" def=\"{ID}\">{body}</a:tblStyleLst>")
}
pub fn inline(b: &[u8], body: &str) -> Vec<u8> {
    table::rewrite(b, |s| {
        s.replacen("</a:tblPr>", &format!("{body}</a:tblPr>"), 1)
    })
}
pub fn catalog(b: &[u8], xml: &str) -> Vec<u8> {
    catalog_with(b, xml, TYPE, |_, _| ())
}
pub fn catalog_with(
    b: &[u8],
    xml: &str,
    content_type: &str,
    f: impl FnOnce(&RelationshipSource, &mut Vec<Relationship>),
) -> Vec<u8> {
    let p = package(b);
    let mut out = PackageBuilder::new();
    for (name, info) in p.parts() {
        if relationship_source(name).unwrap().is_none() {
            out.add_part(
                name.clone(),
                info.content_type.clone(),
                p.read_part(name, 1 << 24, &|| false).unwrap(),
            )
            .unwrap();
        }
    }
    out.add_part(
        PartName::new(STYLES).unwrap(),
        content_type.into(),
        xml.as_bytes(),
    )
    .unwrap();
    let main = RelationshipSource::Part(PartName::new("/ppt/presentation.xml").unwrap());
    let mut rels = p.relationships()[&main].clone();
    rels.push(
        Relationship::new(
            &main,
            "rTableStyles".into(),
            format!("{R}/tableStyles"),
            "tableStyles.xml".into(),
            false,
        )
        .unwrap(),
    );
    f(&main, &mut rels);
    for (source, rels) in p.relationships() {
        if source != &main {
            out.set_relationships(source.clone(), rels.clone()).unwrap();
        }
    }
    out.set_relationships(main, rels).unwrap();
    out.to_bytes(Default::default(), &|| false).unwrap()
}
pub fn full_style(root: &str, id: &str) -> String {
    let edges = [
        "left", "right", "top", "bottom", "insideH", "insideV", "tl2br", "tr2bl",
    ]
    .into_iter()
    .enumerate()
    .map(|(i, n)| {
        let v = if i % 2 == 0 {
            "<a:ln w=\"12700\"><a:solidFill><a:srgbClr val=\"234567\"/></a:solidFill></a:ln>"
        } else {
            "<a:lnRef idx=\"1\"><a:schemeClr val=\"accent2\"/></a:lnRef>"
        };
        format!("<a:{n}>{v}</a:{n}>")
    })
    .collect::<String>();
    let mut body = String::from(
        "<a:tblBg><a:fillRef idx=\"2\"><a:schemeClr val=\"accent1\"/></a:fillRef><a:effect><a:effectLst><a:glow rad=\"12700\"><a:srgbClr val=\"112233\"/></a:glow></a:effectLst></a:effect></a:tblBg>",
    );
    for (i, region) in REGIONS.iter().enumerate() {
        let fonts = if i == 0 {
            "<a:font><a:latin typeface=\"Owned Latin\"/><a:ea typeface=\"Owned East\"/><a:cs typeface=\"Owned Complex\"/><a:font script=\"Hans\" typeface=\"Owned Hans\"/><a:font script=\"Hans\" typeface=\"Second declaration\"/></a:font>"
        } else {
            "<a:fontRef idx=\"minor\"/>"
        };
        body += &format!(
            "<a:{region}><a:tcTxStyle b=\"on\" i=\"off\">{fonts}<a:schemeClr val=\"tx1\"><a:alpha val=\"50000\"/></a:schemeClr></a:tcTxStyle><a:tcStyle><a:tcBdr>{edges}</a:tcBdr><a:fill><a:solidFill><a:srgbClr val=\"ABCDEF\"/></a:solidFill></a:fill></a:tcStyle></a:{region}>"
        );
    }
    style(root, id, &body)
}
