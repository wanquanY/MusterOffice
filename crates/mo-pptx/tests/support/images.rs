use crate::support::{input, resources};
use mo_opc::{
    Package, PackageBuilder, PackageLimits, PartName, Relationship, RelationshipSource,
    relationship_source,
};
use mo_pptx::{
    source::{fill::resolve::*, images::*, *},
    *,
};
use std::collections::BTreeMap;

pub const SLIDE: &str = "/ppt/slides/slide1.xml";
pub const LAYOUT: &str = "/ppt/slideLayouts/slideLayout2.xml";
pub const MASTER: &str = "/ppt/slideMasters/slideMaster2.xml";
pub const THEME: &str = "/ppt/theme/theme2.xml";
pub const REL: &str = "http://schemas.openxmlformats.org/officeDocument/2006/relationships";
pub fn package(bytes: &[u8]) -> Package<&[u8]> {
    Package::open(bytes, bytes.len() as u64, PackageLimits::default(), &|| {
        false
    })
    .unwrap()
}
pub fn base() -> Vec<u8> {
    let (d, defaults) = input();
    export(&d, &defaults, &resources(), PptxLimits::default(), &|| {
        false
    })
    .unwrap()
}
pub struct Fixture {
    parts: BTreeMap<PartName, (String, Vec<u8>)>,
    relationships: BTreeMap<RelationshipSource, Vec<Relationship>>,
}
impl Fixture {
    pub fn new() -> Self {
        let bytes = base();
        let p = package(&bytes);
        Self {
            parts: p
                .parts()
                .iter()
                .filter(|(n, _)| relationship_source(n).unwrap().is_none())
                .map(|(n, i)| {
                    (
                        n.clone(),
                        (
                            i.content_type.clone(),
                            p.read_part(n, 1 << 20, &|| false).unwrap(),
                        ),
                    )
                })
                .collect(),
            relationships: p.relationships().clone(),
        }
    }
    pub fn xml(&mut self, part: &str, change: impl FnOnce(String) -> String) {
        let data = &mut self.parts.get_mut(&PartName::new(part).unwrap()).unwrap().1;
        *data = change(String::from_utf8(data.clone()).unwrap()).into_bytes();
    }
    pub fn shape(&mut self, part: &str, name: &str, fill: &str, hierarchy: bool, theme: bool) {
        self.xml(part, |mut x| {
            let at = x.find(&format!("name=\"{name}\"")).unwrap();
            let start = x[..at].rfind("<p:sp>").unwrap();
            let end = at + x[at..].find("</p:sp>").unwrap() + 7;
            let mut s = x[start..end].to_owned();
            let a = s.find("<p:spPr>").unwrap();
            let b = a + s[a..].find("</p:spPr>").unwrap() + 9;
            s.replace_range(a..b, &format!("<p:spPr>{fill}</p:spPr>"));
            if hierarchy {
                s = s.replace("<p:nvPr/>", "<p:nvPr><p:ph type=\"body\" idx=\"7\"/></p:nvPr>");
            }
            if theme {
                s = s.replace("</p:spPr>", "</p:spPr><p:style><a:lnRef idx=\"0\"/><a:fillRef idx=\"1\"/><a:effectRef idx=\"0\"/><a:fontRef idx=\"minor\"/></p:style>");
            }
            x.replace_range(start..end, &s);
            x
        });
    }
    pub fn theme(&mut self, fill: &str) {
        self.xml(THEME, |mut x| {
            let a = x.find("<a:fillStyleLst>").unwrap() + 16;
            let b = a + x[a..].find("</a:fillStyleLst>").unwrap();
            x.replace_range(a..b, &format!("{fill}<a:noFill/><a:noFill/>"));
            x
        });
    }
    pub fn background(&mut self, part: &str, fill: &str) {
        self.xml(part, |mut x| {
            if let Some(a) = x.find("<p:bg>") {
                let b = a + x[a..].find("</p:bg>").unwrap() + 7;
                x.replace_range(a..b, "");
            }
            if !fill.is_empty() {
                x = x.replacen(
                    "<p:spTree>",
                    &format!("<p:bg><p:bgPr>{fill}</p:bgPr></p:bg><p:spTree>"),
                    1,
                );
            }
            x
        });
    }
    pub fn part(&mut self, part: &str, mime: &str, bytes: &[u8]) {
        self.parts
            .insert(PartName::new(part).unwrap(), (mime.into(), bytes.to_vec()));
    }
    pub fn relation(&mut self, owner: &str, id: &str, kind: &str, target: &str, external: bool) {
        let source = RelationshipSource::Part(PartName::new(owner).unwrap());
        let r = Relationship::new(
            &source,
            id.into(),
            format!("{REL}/{kind}"),
            target.into(),
            external,
        )
        .unwrap();
        let relationships = self.relationships.entry(source).or_default();
        relationships.retain(|r| r.id != id);
        relationships.push(r);
    }
    pub fn finish(self) -> Vec<u8> {
        let mut builder = PackageBuilder::new();
        for (n, (mime, data)) in self.parts {
            builder.add_part(n, mime, data).unwrap();
        }
        for (source, rels) in self.relationships {
            builder.set_relationships(source, rels).unwrap();
        }
        builder
            .to_bytes(PackageLimits::default(), &|| false)
            .unwrap()
    }
}
pub fn inspect(bytes: &[u8]) -> SourceIndex {
    inspect_source(&package(bytes), SourceLimits::default(), &|| false).unwrap()
}
pub fn object(index: &SourceIndex, part: &str, name: &str) -> u32 {
    index.surfaces[part]
        .objects
        .iter()
        .find(|o| o.name == name)
        .unwrap()
        .native_id
}
pub fn request(
    index: &SourceIndex,
    targets: Vec<FillTarget>,
    selection: ImageSourceSelection,
) -> SourceImageQuery {
    SourceImageQuery {
        fill: SourceFillQuery {
            expected_source_sha256: index.source_sha256.clone(),
            surface: SLIDE.into(),
            targets,
            profile: FillProfile::Drawingml2024DraftV1,
        },
        selection,
    }
}
pub fn execute(label: &str, bytes: &[u8], q: &SourceImageQuery) -> SourceImageResources {
    let p = package(bytes);
    let index = inspect_source(&p, SourceLimits::default(), &|| false).unwrap();
    let before = serde_json::to_value(&index).unwrap();
    let r = mo_pptx::source::images::query(&p, &index, q, SourceImageLimits::default(), &|| false)
        .unwrap();
    let bundle = extract(&p, &r, SourceImageLimits::default(), &|| false).unwrap();
    assert_eq!(serde_json::to_value(index).unwrap(), before);
    assert_eq!(bundle.len() as u64, r.bundle_byte_length.get());
    if let Some(dir) = std::env::var_os("MO_SOURCE_IMAGE_EVIDENCE_DIR") {
        let dir = std::path::PathBuf::from(dir);
        std::fs::create_dir_all(&dir).unwrap();
        for (suffix, data) in [
            ("pptx", bytes.to_vec()),
            ("request.json", serde_json::to_vec(q).unwrap()),
            ("projection.json", serde_json::to_vec(&r).unwrap()),
            ("encoded.bin", bundle),
        ] {
            std::fs::write(dir.join(format!("{label}.{suffix}")), data).unwrap();
        }
    }
    r
}
pub fn blip(attrs: &str) -> String {
    format!(
        "<a:blipFill><a:blip xmlns:r=\"{REL}\" {attrs}/><a:stretch><a:fillRect/></a:stretch></a:blipFill>"
    )
}
