mod support;
use mo_opc::{Package, PackageLimits, PartName};
use mo_pptx::{
    PptxLimits, export,
    source::{SourceLimits, inspect_source},
};

#[test]
fn decorative_objects_export_and_reimport_with_original_accessibility() {
    let (mut document, defaults) = support::input();
    for (i, object) in document.objects.values_mut().enumerate() {
        object.accessibility.decorative = i % 2 == 0;
        object.accessibility.title = format!("Title {i}");
        object.accessibility.description = format!("Description <{i}> & detail");
    }
    let bytes = export(
        &document,
        &defaults,
        &support::resources(),
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
    let source = inspect_source(&package, SourceLimits::default(), &|| false).unwrap();
    let mut count = 0;
    for surface in source.surfaces.values() {
        assert!(
            surface.visual_issues.is_empty(),
            "{:?}",
            surface.visual_issues
        );
        for object in &surface.objects {
            let authored =
                &document.objects[&mo_common::ObjectId::new(object.name.clone()).unwrap()];
            assert_eq!(object.accessibility, authored.accessibility);
            assert!(
                object.visual_issues.is_empty(),
                "{:?}",
                object.visual_issues
            );
            count += 1;
        }
    }
    assert_eq!(count, document.objects.len());
    let xml = package
        .read_part(
            &PartName::new("/ppt/slides/slide1.xml").unwrap(),
            1_000_000,
            &|| false,
        )
        .unwrap();
    let xml = std::str::from_utf8(&xml).unwrap();
    assert!(xml.contains("http://schemas.microsoft.com/office/drawing/2017/decorative"));
    assert!(xml.contains("{C183D7F6-B498-43B3-948B-1728B52AA6E4}"));
    let imported = mo_pptx::source::document::import_document(
        &package,
        document.id.clone(),
        mo_common::ResourceId::new("source:pptx").unwrap(),
        SourceLimits::default(),
        &|| false,
    )
    .unwrap();
    for original in document.objects.values() {
        assert!(
            imported
                .objects
                .values()
                .any(|o| o.accessibility == original.accessibility)
        );
    }
}
