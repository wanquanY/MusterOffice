use mo_common::*;
use mo_kernel_api::*;
use mo_presentation_edit::Snapshot;
use mo_presentation_model::{Document, ValidationLimits};
use mo_timeline::*;
fn request() -> TimelineEvaluateRequest {
    let mut document: Document = serde_json::from_value(
        serde_json::from_str::<serde_json::Value>(include_str!(
            "../../../fixtures/presentations/native-export/request.json"
        ))
        .unwrap()["document"]
            .clone(),
    )
    .unwrap();
    let slide = document.slide_order[0].clone();
    document.timelines.insert(
        slide.clone(),
        Timeline {
            format: TimelineVersion::V01,
            tree: None,
            nodes: vec![TimingNode {
                id: TimingNodeId::new("a").unwrap(),
                start: StartCondition::At {
                    offset: RationalTime::new(0, 1).unwrap(),
                },
                duration: RationalTime::new(1, 1).unwrap(),
                end_conditions: vec![],
                repeat_milli: 1000.into(),
                repeat_duration: None,
                time_transform: None,
                fill: FillMode::Freeze,
                effect: Effect::Rotation {
                    target: document.slides[&slide].objects[0].clone(),
                    from: 0,
                    to: 100,
                },
            }],
        },
    );
    let snapshot = Snapshot::new(document, ValidationLimits::default())
        .unwrap()
        .into_record();
    TimelineEvaluateRequest {
        binding: PlaybackBinding {
            session: PlaybackSessionId::new("s").unwrap(),
            revision: snapshot.revision.clone(),
            generation: PlaybackGeneration::new(0),
        },
        snapshot,
        slide,
        at: RationalTime::new(1, 2).unwrap(),
        history: None,
    }
}
fn code(response: TimelineEvaluateResponse) -> String {
    let TimelineEvaluateResponse::Error { error } = response else {
        panic!("expected failure")
    };
    serde_json::to_string(&error.code).unwrap()
}
#[test]
fn binding_and_snapshot_integrity_fail_before_evaluation() {
    let q = request();
    let TimelineEvaluateResponse::Evaluated { frame } =
        evaluate_timeline(q.clone(), TimelineLimits::default(), &|| false)
    else {
        panic!("expected frame")
    };
    assert_eq!(
        frame.state.rotations.values().next().unwrap().numerator,
        "50"
    );
    let mut stale = q.clone();
    stale.binding.revision = Digest::from_sha256([0; 32]);
    assert_eq!(
        code(evaluate_timeline(stale, TimelineLimits::default(), &|| {
            false
        })),
        "\"REVISION_CONFLICT\""
    );
    let mut corrupt = q;
    corrupt.snapshot.document.title = "corrupt".into();
    assert_eq!(
        code(evaluate_timeline(
            corrupt,
            TimelineLimits::default(),
            &|| false
        )),
        "\"INPUT_INVALID\""
    );
}
#[test]
fn validation_and_execution_budgets_and_cancellation_remain_distinct() {
    let mut q = request();
    assert_eq!(
        code(evaluate_timeline(
            q.clone(),
            TimelineLimits::default(),
            &|| true
        )),
        "\"CANCELLED\""
    );
    assert_eq!(
        code(evaluate_timeline(
            q.clone(),
            TimelineLimits {
                max_nodes: 0,
                ..Default::default()
            },
            &|| false
        )),
        "\"LIMIT_EXCEEDED\""
    );
    let timeline = q.snapshot.document.timelines.get_mut(&q.slide).unwrap();
    timeline.nodes = vec![timeline.nodes[0].clone(); 10001];
    assert_eq!(
        code(evaluate_timeline(q, TimelineLimits::default(), &|| false)),
        "\"LIMIT_EXCEEDED\""
    );
}
#[test]
fn strict_json_boundary_rejects_ambiguous_members_without_partial_results() {
    let json = serde_json::to_string(&request()).unwrap();
    let duplicate = json.replacen("\"slide\":", "\"slide\":\"missing\",\"slide\":", 1);
    let r: serde_json::Value =
        serde_json::from_str(&evaluate_timeline_json(&duplicate, &|| false)).unwrap();
    assert_eq!(r["error"]["code"], "INPUT_INVALID");
    assert!(r.get("frame").is_none());
}
