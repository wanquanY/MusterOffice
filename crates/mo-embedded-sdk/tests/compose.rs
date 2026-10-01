use mo_embedded_sdk::{Presentation, common::*, edit::*, operation::*};

#[test]
fn sdk_compact_creation_edits_and_restores_without_a_host() {
    let invocation = decode_invocation(include_str!(
        "../../../fixtures/presentations/compose/invocation.json"
    ))
    .unwrap();
    let DocumentAction::Compose { presentation } = invocation.request.action else {
        panic!()
    };
    let mut deck = Presentation::compose(*presentation, &|| false).unwrap();
    assert_eq!(deck.document().slide_order.len(), 3);
    let before = deck.snapshot().clone();
    deck.edit(
        RequestId::new("edit:composed-title").unwrap(),
        vec![OperationEntry {
            operation_id: OperationId::new("operation:title").unwrap(),
            operation: Operation::SetTitle {
                title: "Edited composed deck".into(),
            },
        }],
        &|| false,
    )
    .unwrap();
    assert_ne!(deck.snapshot().revision, before.revision);
    let restored = Presentation::from_snapshot(deck.into_snapshot()).unwrap();
    assert_eq!(restored.document().title, "Edited composed deck");
    assert_eq!(restored.document().objects, before.document.objects);
}

#[test]
fn paragraph_layout_survives_text_edit_and_snapshot_restore() {
    use mo_embedded_sdk::model::{ObjectContent, ParagraphLineSpacing};
    let mut invocation: serde_json::Value = serde_json::from_str(include_str!(
        "../../../fixtures/presentations/compose/invocation.json"
    ))
    .unwrap();
    let style = &mut invocation["request"]["action"]["presentation"]["slides"][1]["elements"][0]["text"]
        ["style"];
    style["lineSpacing"] = serde_json::json!({"kind":"percent","value":150_000});
    style["leftMargin"] = serde_json::json!("254000");
    style["rightMargin"] = serde_json::json!("127000");
    style["indent"] = serde_json::json!("-127000");
    let invocation = decode_invocation(&invocation.to_string()).unwrap();
    let DocumentAction::Compose { presentation } = invocation.request.action else {
        panic!()
    };
    let mut deck = Presentation::compose(*presentation, &|| false).unwrap();
    let object = ObjectId::new("object:content").unwrap();
    let ObjectContent::Shape {
        text: Some(text), ..
    } = &deck.document().objects[&object].content
    else {
        panic!()
    };
    let paragraph = &text.paragraphs[0];
    let expected_style = paragraph.style.clone();
    assert_eq!(
        expected_style.line_spacing,
        Some(ParagraphLineSpacing::Percent { value: 150_000 })
    );
    assert_eq!(expected_style.indent, Some(Emu::new(-127_000)));
    let operation = Operation::SpliceText {
        object: object.clone(),
        paragraph: paragraph.id.clone(),
        run: paragraph.runs[0].id.clone(),
        start: 0,
        delete: 4,
        insert: "修改后仍保留段落排版".into(),
    };
    deck.edit(
        RequestId::new("edit:paragraph").unwrap(),
        vec![OperationEntry {
            operation_id: OperationId::new("operation:paragraph").unwrap(),
            operation,
        }],
        &|| false,
    )
    .unwrap();
    let snapshot = serde_json::to_vec(&deck.into_snapshot()).unwrap();
    let restored = Presentation::from_snapshot(serde_json::from_slice(&snapshot).unwrap()).unwrap();
    let ObjectContent::Shape {
        text: Some(text), ..
    } = &restored.document().objects[&object].content
    else {
        panic!()
    };
    assert!(text.paragraphs.iter().all(|p| p.style == expected_style));
    assert!(matches!(&text.paragraphs[0].runs[0].content,
        mo_embedded_sdk::model::InlineContent::Text { text } if text == "修改后仍保留段落排版"));
}
