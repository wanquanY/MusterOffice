mod support;
use mo_common::Ticks;
use mo_video_edit::{EditContract, Operation, Transaction};
use mo_video_model::{ClipId, TimeRange, TrackId};
use mo_video_operations::*;
use support::*;
#[test]
fn immutable_candidate_reopen_and_conditional_inverse_share_the_edit_kernel() {
    let p = project();
    let before = p.snapshot();
    let request = Transaction {
        contract: EditContract::V1,
        expected_digest: p.digest().clone(),
        operations: vec![Operation::SetTitle {
            title: "已修改".into(),
        }],
    };
    let a = p.prepare(&request, &|| false).unwrap();
    let b = p.prepare(&request, &|| false).unwrap();
    assert_eq!(a, b);
    assert_eq!(p.snapshot(), before);
    let accepted = VideoProject::open(a.snapshot.clone(), Limits::default(), &|| false).unwrap();
    assert_eq!(accepted.summary(&|| false).unwrap().title, "已修改");
    let undone = accepted
        .prepare_inverse(
            &InverseRequest {
                expected_digest: accepted.digest().clone(),
                inverse: a.inverse,
            },
            &|| false,
        )
        .unwrap();
    assert_eq!(undone.snapshot, before);
    let mut wrong = request;
    wrong.operations.push(Operation::SetEffects {
        clip: ClipId::new("missing").unwrap(),
        effects: vec![],
    });
    let failure = p.prepare(&wrong, &|| false).unwrap_err();
    assert_eq!(failure.operation, Some(1));
    assert_eq!(failure.code, FailureCode::NotFound);
    assert_eq!(p.snapshot(), before);
    let plan = p
        .frame(
            FrameRequest {
                expected_digest: p.digest().clone(),
                frame: Ticks::new(1),
            },
            &|| false,
        )
        .unwrap();
    assert_eq!(plan.document_digest, *p.digest());
    assert_eq!(p.summary(&|| false).unwrap().frame_count, Ticks::new(175));
}
#[test]
fn filtered_pages_pin_snapshot_query_and_physical_resume_position() {
    let p = project();
    let query = ListQuery::Clips {
        track: TrackId::new("v").unwrap(),
        range: Some(TimeRange {
            start: t(2),
            end: t(8),
        }),
    };
    let page = p
        .list(
            PageRequest {
                query: query.clone(),
                cursor: None,
                limit: 1,
            },
            &|| false,
        )
        .unwrap();
    assert!(matches!(&page.items[0],ProjectItem::Clip {clip} if clip.id.as_str()=="b"));
    assert_eq!(page.next.as_ref().unwrap().offset, 2);
    let next = p
        .list(
            PageRequest {
                query: query.clone(),
                cursor: page.next.clone(),
                limit: 1,
            },
            &|| false,
        )
        .unwrap();
    assert!(next.next.is_none());
    assert!(matches!(&next.items[0],ProjectItem::Clip {clip} if clip.id.as_str()=="c"));
    let wrong = p
        .list(
            PageRequest {
                query: ListQuery::Resources,
                cursor: page.next.clone(),
                limit: 1,
            },
            &|| false,
        )
        .unwrap_err();
    assert_eq!(wrong.code, FailureCode::InputInvalid);
    let changed = p
        .prepare(
            &Transaction {
                contract: EditContract::V1,
                expected_digest: p.digest().clone(),
                operations: vec![Operation::SetTitle {
                    title: "new".into(),
                }],
            },
            &|| false,
        )
        .unwrap();
    let latest = VideoProject::open(changed.snapshot, Limits::default(), &|| false).unwrap();
    assert_eq!(
        latest
            .list(
                PageRequest {
                    query,
                    cursor: page.next,
                    limit: 1
                },
                &|| false
            )
            .unwrap_err()
            .code,
        FailureCode::StaleSnapshot
    );
}
#[test]
fn corrupt_unknown_duplicate_budget_and_cancellation_do_not_produce_candidates() {
    let p = project();
    let mut record = p.snapshot();
    record.document.title = "tampered".into();
    assert_eq!(
        VideoProject::open(record, Limits::default(), &|| false)
            .err()
            .unwrap()
            .code,
        FailureCode::SnapshotCorrupt
    );
    for raw in [
        "{\"query\":{\"kind\":\"resources\"},\"limit\":1,\"limit\":2}",
        "{\"query\":{\"kind\":\"future\"},\"limit\":1}",
    ] {
        assert!(decode::<PageRequest>(raw, 1024, &|| false).is_err());
    }
    assert_eq!(
        decode::<CreateProject>("{}", 1, &|| false)
            .unwrap_err()
            .code,
        FailureCode::LimitExceeded
    );
    assert_eq!(
        p.summary(&|| true).unwrap_err().code,
        FailureCode::Cancelled
    );
    let small = VideoProject::open(
        p.snapshot(),
        Limits {
            query: mo_video_compile::QueryLimits {
                max_output_bytes: 64,
                ..Default::default()
            },
            ..Default::default()
        },
        &|| false,
    )
    .unwrap();
    assert_eq!(
        small
            .list(
                PageRequest {
                    query: ListQuery::Clips {
                        track: TrackId::new("v").unwrap(),
                        range: None
                    },
                    cursor: None,
                    limit: 3
                },
                &|| false
            )
            .unwrap_err()
            .code,
        FailureCode::LimitExceeded
    );
}

#[test]
fn candidate_snapshot_and_frame_obey_the_host_budget() {
    let p = project();
    let bytes = serde_json::to_vec(&p.snapshot()).unwrap().len();
    let bounded = VideoProject::open(
        p.snapshot(),
        Limits {
            max_snapshot_bytes: bytes,
            max_output_bytes: 1024,
            ..Default::default()
        },
        &|| false,
    )
    .unwrap();
    let request = Transaction {
        contract: EditContract::V1,
        expected_digest: p.digest().clone(),
        operations: vec![Operation::SetTitle {
            title: "x".repeat(500),
        }],
    };
    assert_eq!(
        bounded.prepare(&request, &|| false).unwrap_err().code,
        FailureCode::LimitExceeded
    );
    let tiny = VideoProject::open(
        p.snapshot(),
        Limits {
            max_output_bytes: 1,
            ..Default::default()
        },
        &|| false,
    )
    .unwrap();
    assert_eq!(
        tiny.frame(
            FrameRequest {
                expected_digest: p.digest().clone(),
                frame: Ticks::new(0)
            },
            &|| false
        )
        .unwrap_err()
        .code,
        FailureCode::LimitExceeded
    );
    assert_eq!(bounded.snapshot(), p.snapshot());
}
