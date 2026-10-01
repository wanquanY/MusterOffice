use mo_common::{Emu, ObjectId};
use mo_presentation_model::*;

fn document() -> Document {
    serde_json::from_str(include_str!(
        "../../../fixtures/presentations/basic-shape.json"
    ))
    .unwrap()
}

fn style(document: &mut Document) -> &mut ParagraphStyle {
    let object = document
        .objects
        .get_mut(&ObjectId::new("shape:1").unwrap())
        .unwrap();
    let ObjectContent::Shape {
        text: Some(text), ..
    } = &mut object.content
    else {
        panic!("text fixture")
    };
    &mut text.paragraphs[0].style
}

#[test]
fn native_paragraph_layout_accepts_exact_values_and_rejects_lossy_or_invalid_values() {
    let mut d = document();
    for spacing in [
        ParagraphLineSpacing::Percent { value: 0 },
        ParagraphLineSpacing::Percent { value: 150_000 },
        ParagraphLineSpacing::Percent { value: 13_200_000 },
        ParagraphLineSpacing::Exact { height: Emu::ZERO },
        ParagraphLineSpacing::Exact {
            height: Emu::new(158_400 * 127),
        },
    ] {
        *style(&mut d) = ParagraphStyle {
            line_spacing: Some(spacing),
            left_margin: Some(Emu::new(51_206_400)),
            right_margin: Some(Emu::ZERO),
            indent: Some(Emu::new(-51_206_400)),
            ..Default::default()
        };
        assert!(validate(&d, Default::default()).is_valid());
    }
    for spacing in [
        ParagraphLineSpacing::Percent { value: 13_200_001 },
        ParagraphLineSpacing::Exact {
            height: Emu::new(-127),
        },
        ParagraphLineSpacing::Exact {
            height: Emu::new(128),
        },
        ParagraphLineSpacing::Exact {
            height: Emu::new(158_401 * 127),
        },
    ] {
        style(&mut d).line_spacing = Some(spacing);
        let report = validate(&d, Default::default());
        assert!(!report.is_valid());
        assert!(
            report
                .issues
                .iter()
                .any(|i| i.path.ends_with("/style/lineSpacing"))
        );
    }
    for (left, right, indent) in [
        (-1, 0, 0),
        (0, -1, 0),
        (51_206_401, 0, 0),
        (0, 0, -51_206_401),
        (0, 0, 51_206_401),
    ] {
        *style(&mut d) = ParagraphStyle {
            left_margin: Some(Emu::new(left)),
            right_margin: Some(Emu::new(right)),
            indent: Some(Emu::new(indent)),
            ..Default::default()
        };
        assert!(!validate(&d, Default::default()).is_valid());
    }
}

#[test]
fn omitted_layout_fields_keep_old_document_identity() {
    let d = document();
    let value = serde_json::to_value(&d).unwrap();
    let p = &value["objects"]["shape:1"]["content"]["text"]["paragraphs"][0]["style"];
    for name in ["lineSpacing", "leftMargin", "rightMargin", "indent"] {
        assert!(p.get(name).is_none());
    }
    let roundtrip: Document = serde_json::from_value(value).unwrap();
    assert_eq!(
        d.semantic_digest().unwrap(),
        roundtrip.semantic_digest().unwrap()
    );
    let mut explicit = d.clone();
    style(&mut explicit).indent = Some(Emu::ZERO);
    assert_ne!(
        d.semantic_digest().unwrap(),
        explicit.semantic_digest().unwrap()
    );
}
