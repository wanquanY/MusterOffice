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
