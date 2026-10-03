use super::*;
use mo_common::Emu;
use mo_presentation_model::{Inherited, Size, Slide};

fn author() -> EditorDocumentInput {
    let value: serde_json::Value = serde_json::from_str(include_str!(
        "../../../../../fixtures/presentations/native-tables/request.json"
    ))
    .unwrap();
    let mut document = Document::empty(
        "document:test".to_owned().try_into().unwrap(),
        Size {
            width: Emu::new(9_144_000),
            height: Emu::new(6_858_000),
        },
    );
    // Deliberately contrary to map/lexical order, with a hidden slide included.
    for (name, hidden) in [("z:first", false), ("a:second", true)] {
        let id: SlideId = name.to_owned().try_into().unwrap();
        document.slide_order.push(id.clone());
        document.slides.insert(
            id.clone(),
            Slide {
                id,
                name: name.into(),
                hidden,
                objects: vec![],
                layout: None,
                background: Inherited::Inherit,
            },
        );
    }
    EditorDocumentInput::Author {
        document: Box::new(document),
        defaults: serde_json::from_value(value["defaults"].clone()).unwrap(),
    }
}
fn run(input: &EditorDocumentInput, bytes: &[u8]) -> EditorDocumentInfo {
    let mut session = EditorPageSession::default();
    let (answer, pixels) = session.dispatch(
        &EditorPageRequest::Inspect {
            input: Box::new(input.clone()),
        },
        bytes,
        &[],
        None,
        &|| false,
    );
    assert!(pixels.is_empty());
    let EditorPageResponse::Inspected { info } = answer else {
        panic!("{answer:?}")
    };
    assert!(
        session.current.is_none(),
        "discovery creates no rendered view"
    );
    *info
}
#[test]
fn model_order_hidden_and_semantic_binding_survive_reordering_and_defaults() {
    let mut input = author();
    let first = run(&input, &[]);
    assert_eq!(
        first
            .slides
            .iter()
            .map(|s| s.slide_id.as_ref().unwrap().as_str())
            .collect::<Vec<_>>(),
        ["z:first", "a:second"]
    );
    assert!(!first.slides[0].hidden);
    assert!(first.slides[1].hidden);
    let EditorDocumentInput::Author { document, defaults } = &mut input else {
        panic!()
    };
    assert_eq!(
        first.model.as_ref().unwrap().semantic_digest,
        document.semantic_digest().unwrap()
    );
    assert_ne!(
        &first.source_sha256,
        &first.model.as_ref().unwrap().semantic_digest
    );
    defaults.font_family = "Another explicit default".into();
    let changed = run(&input, &[]);
    assert_ne!(changed.source_sha256, first.source_sha256);
    assert_eq!(
        changed.model.unwrap().semantic_digest,
        first.model.unwrap().semantic_digest
    );
    let EditorDocumentInput::Author { document, .. } = &mut input else {
        panic!()
    };
    document.slide_order.reverse();
    let second = run(&input, &[]);
    assert_eq!(second.slides[0].slide_id, first.slides[1].slide_id);
    assert_eq!(second.slides[0].slide, first.slides[0].slide);
    assert_ne!(second.source_sha256, changed.source_sha256);
    let EditorDocumentInput::Author { document, .. } = &mut input else {
        panic!()
    };
    document.slides.clear();
    document.slide_order.clear();
    assert!(run(&input, &[]).slides.is_empty());
}
#[test]
fn raw_and_retained_discovery_use_the_same_verified_native_addresses() {
    let EditorDocumentInput::Author { document, defaults } = author() else {
        panic!()
    };
    let bytes = mo_pptx::export(
        &document,
        &defaults,
        &mo_pptx::NoResources,
        Default::default(),
        &|| false,
    )
    .unwrap();
    let raw = run(&EditorDocumentInput::Pptx {}, &bytes);
    assert!(raw.model.is_none());
    assert!(raw.slides.iter().all(|s| s.slide_id.is_none()));
    let package = package(&bytes, &|| false).unwrap();
    let document = mo_pptx::source::document::import_document(
        &package,
        document.id.clone(),
        "source:test".to_owned().try_into().unwrap(),
        Default::default(),
        &|| false,
    )
    .unwrap();
    let input = EditorDocumentInput::Retained {
        document: Box::new(document.clone()),
    };
    let retained = run(&input, &bytes);
    assert_ne!(retained.source_sha256, raw.source_sha256);
    for (i, slide) in retained.slides.iter().enumerate() {
        assert_eq!(slide.slide, raw.slides[i].slide);
        assert_eq!(slide.native_id, raw.slides[i].native_id);
        assert_eq!(slide.hidden, raw.slides[i].hidden);
        assert_eq!(slide.slide_id.as_ref(), Some(&document.slide_order[i]));
    }
    let mut changed = document;
    changed.title = "Supported title overlay".into();
    let next = run(
        &EditorDocumentInput::Retained {
            document: Box::new(changed.clone()),
        },
        &bytes,
    );
    assert_ne!(next.source_sha256, retained.source_sha256);
    assert_eq!(next.slides[0].slide_id, retained.slides[0].slide_id);
    changed.source_bindings.as_mut().unwrap().slides.insert(
        changed.slide_order[0].clone(),
        "/ppt/slides/forged.xml".into(),
    );
    assert!(
        inspect(
            &EditorDocumentInput::Retained {
                document: Box::new(changed)
            },
            &bytes,
            &|| false
        )
        .is_err()
    );
}
#[test]
fn invalid_channels_cancellation_and_document_are_rejected_without_components() {
    let input = author();
    let mut session = EditorPageSession::default();
    let request = EditorPageRequest::Inspect {
        input: Box::new(input.clone()),
    };
    for (material, fonts, stop) in [
        (&b"x"[..], &b""[..], false),
        (&b""[..], &b"x"[..], false),
        (&b""[..], &b""[..], true),
    ] {
        let (answer, pixels) = session.dispatch(&request, material, fonts, None, &|| stop);
        assert!(matches!(answer, EditorPageResponse::Error { .. }));
        assert!(pixels.is_empty());
        assert!(session.current.is_none());
    }
    assert!(inspect(&EditorDocumentInput::Pptx {}, b"invalid", &|| false).is_err());
    let EditorDocumentInput::Author {
        mut document,
        defaults,
    } = input
    else {
        panic!()
    };
    document.slides.clear();
    assert!(
        inspect(
            &EditorDocumentInput::Author { document, defaults },
            &[],
            &|| false
        )
        .is_err()
    );
    for json in [
        r#"{"operation":"inspect","input":{"kind":"pptx","extra":1}}"#,
        r#"{"operation":"inspect","operation":"clear"}"#,
    ] {
        let (answer, pixels) = session.dispatch_json(json, &[], &[], None, &|| false);
        assert_eq!(
            serde_json::from_str::<serde_json::Value>(&answer).unwrap()["status"],
            "error"
        );
        assert!(pixels.is_empty());
    }
}
