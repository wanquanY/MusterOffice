//! Owned PPTX fixture construction shared by compiler and real Native tests.
use mo_common::ResourceId;
use mo_opc::{Package, PackageLimits, PartName, RewritePlan};
use mo_pptx::{source::*, *};
pub const SLIDE: &str = "/ppt/slides/slide1.xml";
pub const FONT: &str = "MusterOffice Synthetic";
struct Resources;
impl mo_pptx::Resources for Resources {
    fn open(&self, _: &ResourceId) -> Result<ResourceData<'_>, PptxError> {
        static BYTES: &[u8] =
            include_bytes!("../../fixtures/presentations/native-export/resources.bin");
        Ok(ResourceData {
            reader: &BYTES,
            byte_length: BYTES.len() as u64,
        })
    }
}
pub fn rewrite(bytes: &[u8], part: &str, f: impl FnOnce(String) -> String) -> Vec<u8> {
    let package = Package::open(bytes, bytes.len() as u64, PackageLimits::default(), &|| {
        false
    })
    .unwrap();
    let part = PartName::new(part).unwrap();
    let xml = String::from_utf8(package.read_part(&part, 1 << 20, &|| false).unwrap()).unwrap();
    let mut plan = RewritePlan::new();
    plan.replace_part(part, f(xml).into_bytes()).unwrap();
    plan.to_bytes(&package, &|| false).unwrap()
}
pub fn bytes(paragraphs: &str) -> Vec<u8> {
    let input: serde_json::Value = serde_json::from_str(include_str!(
        "../../fixtures/presentations/native-export/request.json"
    ))
    .unwrap();
    let b = export(
        &serde_json::from_value(input["document"].clone()).unwrap(),
        &serde_json::from_value(input["defaults"].clone()).unwrap(),
        &Resources,
        PptxLimits::default(),
        &|| false,
    )
    .unwrap();
    rewrite(&b, SLIDE, |mut s| {
        let start = s.find("<p:txBody>").unwrap();
        let end = start + s[start..].find("</p:txBody>").unwrap() + "</p:txBody>".len();
        s.replace_range(start..end, &format!("<p:txBody><a:bodyPr/><a:lstStyle><a:lvl1pPr><a:defRPr lang=\"en\"><a:latin typeface=\"{FONT}\"/><a:ea typeface=\"EA\"/><a:cs typeface=\"CS\"/></a:defRPr></a:lvl1pPr></a:lstStyle>{paragraphs}</p:txBody>"));
        s
    })
}
pub fn read(b: &[u8]) -> SourceIndex {
    inspect_source(
        &Package::open(b, b.len() as u64, PackageLimits::default(), &|| false).unwrap(),
        SourceLimits::default(),
        &|| false,
    )
    .unwrap()
}
pub fn target(i: &SourceIndex) -> SourceObjectRef {
    SourceObjectRef {
        part: SLIDE.into(),
        native_id: i.surfaces[SLIDE].objects[0].native_id,
    }
}
pub fn run(text: &str) -> String {
    format!("<a:r><a:t>{text}</a:t></a:r>")
}
