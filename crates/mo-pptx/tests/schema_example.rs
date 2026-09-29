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

#[test]
fn discovered_example_accepts_complete_page_batches_and_exports_every_page() {
    use mo_presentation_edit::{Snapshot, Transaction, prepare};
    use mo_presentation_model::ValidationLimits;
    use serde_json::json;

    let schema = serde_json::to_value(schemars::schema_for!(Document)).unwrap();
    let example = schema.pointer("/examples/0").unwrap();
    let mut snapshot = Snapshot::new(
        serde_json::from_value(example.clone()).unwrap(),
        ValidationLimits::default(),
    )
    .unwrap();
    for page in 2..=3 {
        let slide_id = format!("slide:{page}");
        let object_id = format!("object:title:{page}");
        let mut slide = example["slides"]["slide:1"].clone();
        slide["id"] = json!(slide_id);
        slide["name"] = json!(format!("Page {page}"));
        slide["objects"] = json!([]);
        let mut object = example["objects"]["object:title"].clone();
        object["id"] = json!(object_id);
        object["parent"]["id"] = json!(slide_id);
        let paragraph = &mut object["content"]["text"]["paragraphs"][0];
        paragraph["id"] = json!(format!("paragraph:title:{page}"));
        paragraph["runs"][0]["id"] = json!(format!("run:title:{page}"));
        paragraph["runs"][0]["content"]["text"] = json!(format!("Page {page} title"));
        let transaction: Transaction = serde_json::from_value(json!({
            "documentId": snapshot.document().id,
            "requestId": format!("request:page:{page}"),
            "baseRevision": snapshot.revision(),
            "operations": [
                {"operationId": format!("operation:slide:{page}"),
                 "operation": {"kind":"insertSlide", "slide":slide, "index":page-1}},
                {"operationId": format!("operation:object:{page}"),
                 "operation": {"kind":"insertObject", "object":object, "index":0}}
            ]
        }))
        .unwrap();
        let before = snapshot.semantic_digest().clone();
        let prepared = prepare(&snapshot, &transaction, ValidationLimits::default()).unwrap();
        assert_eq!(snapshot.semantic_digest(), &before);
        snapshot = prepared.snapshot;
        assert_eq!(snapshot.document().slide_order.len(), page as usize);
        assert_eq!(snapshot.document().objects.len(), page as usize);
        assert!(prepare(&snapshot, &transaction, ValidationLimits::default()).is_err());
    }
    let defaults = serde_json::from_value(
        serde_json::from_str::<serde_json::Value>(include_str!(
            "../../../fixtures/presentations/native-export/request.json"
        ))
        .unwrap()["defaults"]
            .clone(),
    )
    .unwrap();
    let bytes = export(
        snapshot.document(),
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
    assert_eq!(
        inspect_source(&package, SourceLimits::default(), &|| false)
            .unwrap()
            .slides
            .len(),
        3
    );
    for (page, title) in [(1, "Your title"), (2, "Page 2 title"), (3, "Page 3 title")] {
        let xml = String::from_utf8(
            package
                .read_part(
                    &PartName::new(format!("/ppt/slides/slide{page}.xml")).unwrap(),
                    100_000,
                    &|| false,
                )
                .unwrap(),
        )
        .unwrap();
        assert_eq!(xml.matches("<p:sp>").count(), 1);
        assert!(xml.contains(title));
        assert!(!xml.contains("<p:pic>"));
    }
}
