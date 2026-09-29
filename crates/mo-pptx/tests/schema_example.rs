use mo_common::Emu;
use mo_opc::{Package, PackageLimits, PartName};
use mo_pptx::{
    ExportDefaults, NoResources, PptxLimits, export,
    source::{SourceLimits, inspect_source},
};
use mo_presentation_model::Document;

#[test]
fn discovered_document_example_exports_native_editable_text_without_external_assets() {
    let schema = serde_json::to_value(schemars::schema_for!(Document)).unwrap();
    let mut document: Document =
        serde_json::from_value(schema.pointer("/examples/0").unwrap().clone()).unwrap();
    let defaults: ExportDefaults = serde_json::from_value(
        serde_json::from_str::<serde_json::Value>(include_str!(
            "../../../fixtures/presentations/native-export/request.json"
        ))
        .unwrap()["defaults"]
            .clone(),
    )
    .unwrap();
    for title in ["Your title", "Edited title"] {
        let object = document.objects.values_mut().next().unwrap();
        let mo_presentation_model::ObjectContent::Shape {
            text: Some(text), ..
        } = &mut object.content
        else {
            panic!("example must contain editable text")
        };
        text.paragraphs[0].runs[0].content =
            mo_presentation_model::InlineContent::Text { text: title.into() };
        let bytes = export(
            &document,
            &defaults,
            &NoResources,
            PptxLimits::default(),
            &|| false,
        )
        .unwrap();
        let package = Package::open(
            bytes.as_slice(),
            bytes.len() as u64,
            PackageLimits::default(),
            &|| false,
        )
        .unwrap();
        let index = inspect_source(&package, SourceLimits::default(), &|| false).unwrap();
        assert_eq!(index.slides.len(), 1);
        assert_eq!(index.page_size.unwrap().width, Emu::new(12_192_000));
        let xml = String::from_utf8(
            package
                .read_part(
                    &PartName::new("/ppt/slides/slide1.xml").unwrap(),
                    100_000,
                    &|| false,
                )
                .unwrap(),
        )
        .unwrap();
        assert!(xml.contains(title));
        assert!(xml.contains("<p:sp>"));
        assert!(!xml.contains("<p:pic>"));
    }
}
