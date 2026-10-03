mod support;
use mo_common::*;
use mo_opc::{Package, PartName, RewritePlan};
use mo_pptx::{source::document::import_document, *};
use mo_presentation_model::*;

struct Inputs {
    original: Vec<u8>,
    images: support::Fixture,
}
impl Resources for Inputs {
    fn open(&self, id: &ResourceId) -> Result<ResourceData<'_>, PptxError> {
        if id.as_str() == "original" {
            Ok(ResourceData {
                reader: &self.original,
                byte_length: self.original.len() as u64,
            })
        } else {
            self.images.open(id)
        }
    }
}
fn package(bytes: &[u8]) -> Package<&[u8]> {
    Package::open(bytes, bytes.len() as u64, Default::default(), &|| false).unwrap()
}
fn import(bytes: &[u8]) -> Document {
    import_document(
        &package(bytes),
        DocumentId::new("imported").unwrap(),
        ResourceId::new("original").unwrap(),
        Default::default(),
        &|| false,
    )
    .unwrap()
}
fn fixture() -> (Document, ExportDefaults, Inputs) {
    let (authored, defaults) = support::input();
    let original = export(
        &authored,
        &defaults,
        &support::resources(),
        Default::default(),
        &|| false,
    )
    .unwrap();
    let source = package(&original);
    let part = PartName::new("/ppt/slides/slide1.xml").unwrap();
    let xml = String::from_utf8(source.read_part(&part, 1 << 24, &|| false).unwrap()).unwrap()
        .replace("</p:sld>", "<p:extLst><p:ext uri=\"owned-test\"><x:opaque xmlns:x=\"urn:owned\" value=\"preserve\"/></p:ext></p:extLst></p:sld>");
    let mut rewrite = RewritePlan::new();
    rewrite.replace_part(part, xml.into_bytes()).unwrap();
    let original = rewrite.to_bytes(&source, &|| false).unwrap();
    (
        import(&original),
        defaults,
        Inputs {
            original,
            images: support::resources(),
        },
    )
}
fn append(document: &mut Document) {
    let (mut fragment, _) = support::input();
    for slide in fragment.slides.values_mut() {
        slide.layout = None;
    }
    fragment.objects.retain(|_, object| {
        !matches!(
            object.parent,
            ContainerId::Master(_) | ContainerId::Layout(_)
        )
    });
    document.slide_order.extend(fragment.slide_order);
    document.slides.extend(fragment.slides);
    document.objects.extend(fragment.objects);
    document.resources.extend(fragment.resources);
}

#[test]
fn native_append_preserves_original_parts_and_supports_reopen_and_repeat_edit() {
    let (mut document, defaults, inputs) = fixture();
    let source = package(&inputs.original);
    let original_count = document.slides.len();
    append(&mut document);
    let output = export(&document, &defaults, &inputs, Default::default(), &|| false).unwrap();
    assert_eq!(
        output,
        export(&document, &defaults, &inputs, Default::default(), &|| false).unwrap()
    );
    let result = package(&output);
    let reopened = import(&output);
    assert_eq!(reopened.slides.len(), original_count + 2);
    assert_eq!(reopened.title, document.title);
    for (part, info) in source.parts() {
        if !matches!(
            part.as_str(),
            "/ppt/presentation.xml" | "/ppt/_rels/presentation.xml.rels"
        ) {
            assert_eq!(info.sha256, result.parts()[part].sha256, "{part}");
        }
    }
    for (owner, relations) in source.relationships() {
        for relation in relations {
            assert!(result.relationships()[owner].contains(relation));
        }
    }
    assert!(
        result
            .parts()
            .keys()
            .any(|part| part.as_str().starts_with("/ppt/musterOffice/"))
    );
    let mut roundtrip = reopened;
    let original_id = roundtrip.slide_order[0].clone();
    let new_id = roundtrip.slide_order.last().unwrap().clone();
    for slide in [original_id, new_id] {
        let object = roundtrip.objects.values_mut().find(|o| o.parent == ContainerId::Slide(slide.clone()) &&
            matches!(&o.content, ObjectContent::RetainedSource { paragraphs, .. } if paragraphs.iter().any(|p| !p.runs.is_empty()))).unwrap();
        if let ObjectContent::RetainedSource { paragraphs, .. } = &mut object.content {
            paragraphs
                .iter_mut()
                .find(|p| !p.runs.is_empty())
                .unwrap()
                .runs[0]
                .text = "second edit 中文".into();
        }
    }
    let second_inputs = Inputs {
        original: output,
        images: support::resources(),
    };
    let second = export(
        &roundtrip,
        &defaults,
        &second_inputs,
        Default::default(),
        &|| false,
    )
    .unwrap();
    assert_eq!(import(&second).slides.len(), original_count + 2);
    assert!(
        String::from_utf8(
            package(&second)
                .read_part(
                    &PartName::new("/ppt/slides/slide1.xml").unwrap(),
                    1 << 24,
                    &|| false
                )
                .unwrap()
        )
        .unwrap()
        .contains("value=\"preserve\"")
    );
}

#[test]
fn edit_retained_text_and_append_are_one_verified_plan() {
    let (mut document, defaults, inputs) = fixture();
    let target = document.objects.values_mut().find(|o| matches!(&o.content,
        ObjectContent::RetainedSource { paragraphs, .. } if paragraphs.iter().any(|p| !p.runs.is_empty()))).unwrap();
    if let ObjectContent::RetainedSource { paragraphs, .. } = &mut target.content {
        paragraphs
            .iter_mut()
            .find(|p| !p.runs.is_empty())
            .unwrap()
            .runs[0]
            .text = "edited before append".into();
    }
    document.title = "retained and appended".into();
    append(&mut document);
    let output = export(&document, &defaults, &inputs, Default::default(), &|| false).unwrap();
    let result = import(&output);
    assert_eq!(result.title, document.title);
    assert!(result.objects.values().any(
        |o| matches!(&o.content, ObjectContent::RetainedSource { paragraphs, .. }
        if paragraphs.iter().flat_map(|p| &p.runs).any(|r| r.text == "edited before append"))
    ));
}

#[test]
fn mixed_document_cannot_hide_source_mutations_or_inject_authored_children() {
    let (mut document, defaults, inputs) = fixture();
    append(&mut document);
    let mut forged = document.clone();
    forged
        .source_bindings
        .as_mut()
        .unwrap()
        .objects
        .values_mut()
        .next()
        .unwrap()
        .native_id += 999;
    assert!(export(&forged, &defaults, &inputs, Default::default(), &|| false).is_err());
    let mut injected = document.clone();
    injected
        .objects
        .get_mut(&ObjectId::new("title:1").unwrap())
        .unwrap()
        .parent = ContainerId::Slide(injected.slide_order[0].clone());
    assert!(export(&injected, &defaults, &inputs, Default::default(), &|| false).is_err());
    let mut legacy = document.clone();
    legacy.source_bindings.as_mut().unwrap().profile =
        SourceBindingProfile::PresentationmlRetainedFieldsV4;
    assert!(export(&legacy, &defaults, &inputs, Default::default(), &|| false).is_err());
    let mut reordered = document.clone();
    reordered.slide_order.swap(0, 2);
    assert!(
        export(&reordered, &defaults, &inputs, Default::default(), &|| {
            false
        })
        .is_err()
    );
    assert!(export(&document, &defaults, &inputs, Default::default(), &|| true).is_err());
}
