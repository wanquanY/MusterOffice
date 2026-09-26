use mo_opc::{Package, PackageLimits, PartName, RelationshipSource};
use mo_pptx::source::{SourceLimits, inspect_source};
use mo_pptx::*;
use mo_presentation_model::*;

mod support;
use support::{input, resources};

#[test]
fn static_rotation_is_canonical_in_new_packages_without_mutating_author_intent() {
    for angle in [
        i32::MIN,
        -43_200_001,
        -21_600_000,
        -8_100_000,
        -1,
        0,
        1,
        21_600_000,
        24_300_000,
        i32::MAX,
    ] {
        let (mut document, defaults) = input();
        for object in document.objects.values_mut() {
            if !matches!(object.content, ObjectContent::Connector { .. })
                || angle.rem_euclid(21_600_000) == 0
            {
                object.transform.as_mut().unwrap().rotation = angle;
            }
        }
        let before = serde_json::to_vec(&document).unwrap();
        let bytes = export(
            &document,
            &defaults,
            &resources(),
            PptxLimits::default(),
            &|| false,
        )
        .unwrap();
        assert_eq!(serde_json::to_vec(&document).unwrap(), before);
        let mut equivalent = document.clone();
        for object in equivalent.objects.values_mut() {
            object.transform.as_mut().unwrap().rotation =
                object.transform.as_ref().unwrap().normalized_rotation();
        }
        assert_eq!(
            bytes,
            export(
                &equivalent,
                &defaults,
                &resources(),
                PptxLimits::default(),
                &|| false,
            )
            .unwrap()
        );
        let package = Package::open(
            bytes.as_slice(),
            bytes.len() as u64,
            PackageLimits::default(),
            &|| false,
        )
        .unwrap();
        let index = inspect_source(&package, SourceLimits::default(), &|| false).unwrap();
        for object in index.surfaces.values().flat_map(|s| &s.objects) {
            let original = &document.objects[&mo_common::ObjectId::new(&object.name).unwrap()];
            assert_eq!(
                object.transform.as_ref().unwrap().rotation,
                Some(
                    original
                        .transform
                        .as_ref()
                        .unwrap()
                        .rotation
                        .rem_euclid(21_600_000)
                )
            );
        }
    }
}

#[test]
fn authored_deck_exports_deterministically_with_native_parts_and_relationships() {
    let (document, defaults) = input();
    let first = export(
        &document,
        &defaults,
        &resources(),
        PptxLimits::default(),
        &|| false,
    )
    .unwrap();
    assert_eq!(
        first,
        export(
            &document,
            &defaults,
            &resources(),
            PptxLimits::default(),
            &|| false
        )
        .unwrap()
    );
    let package = Package::open(
        first.as_slice(),
        first.len() as u64,
        PackageLimits::default(),
        &|| false,
    )
    .unwrap();
    let part = PartName::new("/ppt/slides/slide1.xml").unwrap();
    let xml = package.read_part(&part, 1024 * 1024, &|| false).unwrap();
    let mut text = String::new();
    mo_xml::scan(&xml, mo_xml::XmlLimits::default(), |event| {
        if let mo_xml::XmlEvent::Text { text: value, .. } = event {
            text.push_str(value);
        }
        Ok(())
    })
    .unwrap();
    assert!(text.contains("中文 / العربية / 🚀 / é"));
    assert!(
        package.relationships()[&RelationshipSource::Part(part)]
            .iter()
            .any(|r| r.relationship_type.ends_with("/image"))
    );
    assert_eq!(
        package
            .read_part(
                &PartName::new("/ppt/media/image1.png").unwrap(),
                4096,
                &|| false
            )
            .unwrap(),
        resources().0
    );
}

#[test]
fn invalid_or_unimplemented_mapping_never_silently_flattens_content() {
    let (mut document, defaults) = input();
    document
        .layouts
        .values_mut()
        .next()
        .unwrap()
        .default_text
        .bold = Inherited::Value(true);
    assert!(matches!(
        export(
            &document,
            &defaults,
            &resources(),
            PptxLimits::default(),
            &|| false
        ),
        Err(PptxError::Unsupported(_))
    ));
    let (mut document, defaults) = input();
    if let ObjectContent::Shape {
        text: Some(body), ..
    } = &mut document
        .objects
        .get_mut(&mo_common::ObjectId::new("title:1").unwrap())
        .unwrap()
        .content
    {
        body.style.size = Inherited::Value(mo_common::Emu::new(12701));
    }
    assert!(matches!(
        export(
            &document,
            &defaults,
            &resources(),
            PptxLimits::default(),
            &|| false
        ),
        Err(PptxError::Value { .. })
    ));
    let (mut document, defaults) = input();
    document.resources.values_mut().next().unwrap().sha256 = "0".repeat(64).try_into().unwrap();
    assert!(matches!(
        export(
            &document,
            &defaults,
            &resources(),
            PptxLimits::default(),
            &|| false
        ),
        Err(PptxError::Opc(mo_opc::OpcError::Preservation(_)))
    ));
}

#[test]
fn missing_resources_cancellation_and_output_budgets_are_enforced() {
    let (document, defaults) = input();
    assert!(matches!(
        export(
            &document,
            &defaults,
            &NoResources,
            PptxLimits::default(),
            &|| false
        ),
        Err(PptxError::ResourceRequired(_))
    ));
    assert!(matches!(
        export(
            &document,
            &defaults,
            &resources(),
            PptxLimits::default(),
            &|| true
        ),
        Err(PptxError::Cancelled)
    ));
    let limits = PptxLimits {
        package: PackageLimits {
            xml: mo_xml::XmlLimits {
                max_bytes: 200,
                ..Default::default()
            },
            ..Default::default()
        },
        ..Default::default()
    };
    assert!(matches!(
        export(&document, &defaults, &resources(), limits, &|| false),
        Err(PptxError::Limit(_))
    ));
}
