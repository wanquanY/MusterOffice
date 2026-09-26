mod support;
use mo_common::{Emu, ObjectId};
use mo_opc::{Package, PackageLimits};
use mo_pptx::{
    source::{
        text::{cascade::*, fonts::*},
        *,
    },
    *,
};
use mo_presentation_model::*;

fn roundtrip(document: &Document, defaults: &ExportDefaults) -> SourceIndex {
    let bytes = export(
        document,
        defaults,
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
    inspect_source(&package, SourceLimits::default(), &|| false).unwrap()
}
fn cascade(index: &SourceIndex, part: &str, id: u32) -> CascadedText {
    let result = mo_pptx::source::text::cascade::resolve(
        index,
        &index.source_sha256,
        &SourceObjectRef {
            part: part.into(),
            native_id: id,
        },
        Default::default(),
        &|| false,
    )
    .unwrap();
    let TextCascadeOutcome::Cascaded { text } = result else {
        panic!("{result:?}");
    };
    *text
}
#[test]
fn author_text_defaults_survive_native_export_and_office_style_cascade() {
    let (document, defaults) = support::input();
    let index = roundtrip(&document, &defaults);
    let mut checked = 0;
    for (part, surface) in &index.surfaces {
        for object in &surface.objects {
            let source = &document.objects[&ObjectId::new(&object.name).unwrap()];
            let ObjectContent::Shape {
                text: Some(body), ..
            } = &source.content
            else {
                continue;
            };
            let resolved = cascade(&index, part, object.native_id);
            assert_eq!(resolved.paragraphs.len(), body.paragraphs.len());
            let Inherited::Value(size) = body.style.size else {
                panic!("fixture has explicit body size");
            };
            for (p, paragraph) in resolved.paragraphs.iter().enumerate() {
                for (r, run) in paragraph.runs.iter().enumerate() {
                    assert_eq!(
                        run.style.attributes.size,
                        Some(i32::try_from(size.get() / 127).unwrap())
                    );
                    for slot in [
                        NativeFontSlot::Latin,
                        NativeFontSlot::EastAsian,
                        NativeFontSlot::ComplexScript,
                    ] {
                        let font = mo_pptx::source::text::fonts::resolve(
                            &index,
                            &resolved,
                            p as u32,
                            Some(r as u32),
                            slot,
                            None,
                            Default::default(),
                            &|| false,
                        )
                        .unwrap();
                        let TypefaceOutcome::Named { font } = font else {
                            panic!("{font:?}");
                        };
                        assert_eq!(font.typeface, defaults.font_family);
                        assert!(matches!(
                            font.declared_by.origin,
                            TextStyleOrigin::Presentation { .. }
                        ));
                    }
                    assert!(matches!(
                        run.style.origins[&CharacterProperty::Size],
                        TextStyleOrigin::Object { .. }
                    ));
                    assert!(run.style.declarations.contains_key(&CharacterSlot::Fill));
                    checked += 1;
                }
            }
        }
    }
    assert_eq!(checked, 8);
}
#[test]
fn native_matching_level_defaults_preserve_master_body_paragraph_and_run_precedence() {
    let (mut document, defaults) = support::input();
    let master = document.masters.values_mut().next().unwrap();
    master.default_text.size = Inherited::Value(Emu::new(254000));
    master.default_text.bold = Inherited::Value(true);
    for name in ["title:1", "footer:master"] {
        let ObjectContent::Shape {
            text: Some(body), ..
        } = &mut document
            .objects
            .get_mut(&ObjectId::new(name).unwrap())
            .unwrap()
            .content
        else {
            panic!()
        };
        body.style.size = Inherited::Inherit;
        body.style.italic = Inherited::Value(true);
        body.paragraphs[0].default_run_style.bold = Inherited::Value(false);
        body.paragraphs[0].runs[0].style.italic = Inherited::Value(false);
    }
    let index = roundtrip(&document, &defaults);
    for (part, surface) in &index.surfaces {
        for object in surface
            .objects
            .iter()
            .filter(|o| ["title:1", "footer:master"].contains(&o.name.as_str()))
        {
            let resolved = cascade(&index, part, object.native_id);
            let run = &resolved.paragraphs[0].runs[0].style;
            assert_eq!(run.attributes.size, Some(2000));
            assert_eq!(run.attributes.bold, Some(false));
            assert_eq!(run.attributes.italic, Some(false));
            assert!(matches!(
                run.origins[&CharacterProperty::Size],
                TextStyleOrigin::Master { .. }
            ));
        }
    }
}
