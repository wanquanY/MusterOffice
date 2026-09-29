use mo_opc::{Package, PartName, RewritePlan};
use mo_pptx::{ExportDefaults, NoResources, export, source::*};
use mo_presentation_model::Document;
pub const SLIDE: &str = "/ppt/slides/slide1.xml";
pub fn input() -> (Document, ExportDefaults) {
    let q: serde_json::Value = serde_json::from_str(include_str!(
        "../../fixtures/presentations/native-tables/request.json"
    ))
    .unwrap();
    (
        serde_json::from_value(q["document"].clone()).unwrap(),
        serde_json::from_value(q["defaults"].clone()).unwrap(),
    )
}
pub fn bytes() -> Vec<u8> {
    let (document, defaults) = input();
    export(
        &document,
        &defaults,
        &NoResources,
        Default::default(),
        &|| false,
    )
    .unwrap()
}
pub fn read(b: &[u8]) -> SourceIndex {
    let package = Package::open(b, b.len() as u64, Default::default(), &|| false).unwrap();
    inspect_source(&package, Default::default(), &|| false).unwrap()
}
pub fn rewrite(b: &[u8], f: impl FnOnce(String) -> String) -> Vec<u8> {
    let package = Package::open(b, b.len() as u64, Default::default(), &|| false).unwrap();
    let part = PartName::new(SLIDE).unwrap();
    let xml = String::from_utf8(
        package
            .read_part(&part, 16 * 1024 * 1024, &|| false)
            .unwrap(),
    )
    .unwrap();
    let mut plan = RewritePlan::new();
    plan.replace_part(part, f(xml).into_bytes()).unwrap();
    plan.to_bytes(&package, &|| false).unwrap()
}
pub fn target(i: &SourceIndex) -> SourceObjectRef {
    SourceObjectRef {
        part: SLIDE.into(),
        native_id: i.surfaces[SLIDE].objects[0].native_id,
    }
}
