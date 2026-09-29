//! A consumer needs only the public facade for typed editorial authoring.
use mo_embedded_sdk::{
    Presentation, common::*, edit::*, model::Document, operation::FailureCode, timeline::*,
};
use std::cell::Cell;

#[test]
fn typed_sequence_edits_are_atomic_at_every_observed_cancellation_boundary() {
    let document: Document = serde_json::from_str(include_str!(
        "../../../fixtures/presentations/basic-shape.json"
    ))
    .unwrap();
    let slide = document.slide_order[0].clone();
    let target = document.slides[&slide].objects[0].clone();
    let original = Presentation::create(document, &|| false)
        .unwrap()
        .into_snapshot();
    let operations = vec![OperationEntry {
        operation_id: OperationId::new("sequence").unwrap(),
        operation: Operation::SetPresentationSequence {
            slide: slide.clone(),
            sequence: PresentationSequence {
                groups: vec![PresentationGroup {
                    start: PresentationGroupStart::Automatic,
                    batches: vec![PresentationBatch {
                        delay: RationalTime::new(0, 1).unwrap(),
                        effects: vec![PresentationEffect {
                            id: TimingNodeId::new("spin").unwrap(),
                            delay: RationalTime::new(0, 1).unwrap(),
                            duration: RationalTime::new(2, 1).unwrap(),
                            repeat_milli: 1000.into(),
                            repeat_duration: None,
                            fill: FillMode::Hold,
                            time_transform: None,
                            effect: Effect::Rotation {
                                composition: Default::default(),
                                target,
                                from: 0,
                                to: 21600000,
                            },
                        }],
                    }],
                }],
            },
        },
    }];
    let mut expected = Presentation::from_snapshot(original.clone()).unwrap();
    let calls = Cell::new(0);
    expected
        .edit(
            RequestId::new("author").unwrap(),
            operations.clone(),
            &|| {
                calls.set(calls.get() + 1);
                false
            },
        )
        .unwrap();
    assert_eq!(expected.document().timelines[&slide].node_count(), 5);
    for stop in 1..=calls.get() {
        let mut deck = Presentation::from_snapshot(original.clone()).unwrap();
        let count = Cell::new(0);
        let failure = deck
            .edit(
                RequestId::new("author").unwrap(),
                operations.clone(),
                &|| {
                    count.set(count.get() + 1);
                    count.get() == stop
                },
            )
            .unwrap_err();
        assert_eq!(failure.code, FailureCode::Cancelled, "at {stop}");
        assert_eq!(deck.snapshot(), &original);
    }
}
