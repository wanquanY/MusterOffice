use super::*;

#[test]
fn snapshots_deduplicate_prefixes_and_keep_draw_order() {
    let mut q = crate::tests::request();
    let mut capture = q.draws[0].clone();
    capture.brush = Brush::Snapshot {
        after_draws: 1,
        scope: SnapshotScope::Current,
    };
    capture.blend = BlendMode::Source;
    assert_eq!(
        serde_json::to_value(&capture.brush).unwrap(),
        serde_json::json!({"kind": "snapshot", "afterDraws": 1})
    );
    q.draws.extend([capture.clone(), capture]);
    let c = compile(&q, &|| false).unwrap();
    let w = c.work().compositing.as_ref().unwrap();
    assert_eq!(c.frame()[1], 8);
    assert_eq!((w.captures, w.snapshot_draws, w.source_draws), (1, 2, 2));
    assert_eq!(w.captured_bytes as usize, c.pixel_bytes());
    let last = q.draws.last_mut().unwrap();
    last.brush = Brush::Snapshot {
        after_draws: 0,
        scope: SnapshotScope::Current,
    };
    let c = compile(&q, &|| false).unwrap();
    assert_eq!(c.work().compositing.as_ref().unwrap().captures, 2);
    q.draws[0].brush = Brush::Snapshot {
        after_draws: 1,
        scope: SnapshotScope::Current,
    };
    assert!(matches!(
        compile(&q, &|| false),
        Err(RasterError::Invalid("snapshot must precede its draw"))
    ));
}

#[test]
fn snapshot_copy_budget_is_checked_before_any_backend() {
    let mut q = crate::tests::request();
    q.viewport.width = 8192;
    q.viewport.height = 2048;
    q.draws[0].brush = Brush::Snapshot {
        after_draws: 0,
        scope: SnapshotScope::Current,
    };
    q.draws.push(q.draws[0].clone());
    let c = compile(&q, &|| false).unwrap();
    assert_eq!(
        c.work().compositing.as_ref().unwrap().captured_bytes,
        64 * 1024 * 1024
    );
    q.draws[1].brush = Brush::Snapshot {
        after_draws: 1,
        scope: SnapshotScope::Current,
    };
    assert!(matches!(
        compile(&q, &|| false),
        Err(RasterError::Limit("snapshot pixel bytes"))
    ));
    q.viewport.width = 1;
    q.viewport.height = 1;
    q.draws.resize(65, q.draws[0].clone());
    for (i, d) in q.draws.iter_mut().enumerate() {
        d.brush = Brush::Snapshot {
            after_draws: i as u32,
            scope: SnapshotScope::Current,
        };
    }
    assert!(matches!(
        compile(&q, &|| false),
        Err(RasterError::Limit("snapshots"))
    ));
}

#[test]
fn source_blend_is_explicit_and_legacy_requests_stay_legacy() {
    let mut q = crate::tests::request();
    let old = compile(&q, &|| false).unwrap();
    assert_eq!(old.frame()[1], 4);
    assert!(old.work().compositing.is_none());
    let before = serde_json::to_string(&q).unwrap();
    assert!(!before.contains("blend"));
    q.draws[0].blend = BlendMode::Source;
    let c = compile(&q, &|| false).unwrap();
    let w = c.work().compositing.as_ref().unwrap();
    assert_eq!((c.frame()[1], w.captures, w.source_draws), (8, 0, 1));
    assert_eq!(w.captured_bytes, 0);
    assert!(
        serde_json::to_string(&q)
            .unwrap()
            .contains("\"blend\":\"source\"")
    );
    q.draws[0].blend = BlendMode::SourceOver;
    assert_eq!(compile(&q, &|| false).unwrap().frame(), old.frame());
}

#[test]
fn explicit_capture_identity_includes_scope_and_survives_group_close() {
    let mut q = crate::tests::request();
    q.draws.resize(5, q.draws[0].clone());
    q.opacity_groups = vec![OpacityGroup {
        first_draw: 1,
        end_draw: 4,
        opacity: 32768,
    }];
    for (i, scope) in [
        (2, SnapshotScope::Output),
        (3, SnapshotScope::Current),
        (4, SnapshotScope::Group { index: 0 }),
    ] {
        q.draws[i].brush = Brush::Snapshot {
            after_draws: 2,
            scope,
        };
    }
    let c = compile(&q, &|| false).unwrap();
    assert_eq!(c.frame()[1], 14);
    let work = c.work().compositing.as_ref().unwrap();
    // The current/group names resolve to the same immutable capture, while
    // output at exactly the same position is a distinct set of pixels.
    assert_eq!((work.captures, work.snapshot_draws), (2, 3));
    let at = c.frame().len() - q.draws.len() * 8 - 3 - 4;
    assert_eq!(&c.frame()[at..at + 4], &[2, 0, 2, 1]);
    q.draws[4].brush = Brush::Snapshot {
        after_draws: 4,
        scope: SnapshotScope::Group { index: 0 },
    };
    assert!(matches!(
        compile(&q, &|| false),
        Err(RasterError::Invalid(
            "snapshot group is not active at capture"
        ))
    ));
    q.draws[4].brush = Brush::Snapshot {
        after_draws: 0,
        scope: SnapshotScope::Group { index: u32::MAX },
    };
    assert!(matches!(
        compile(&q, &|| false),
        Err(RasterError::Invalid("snapshot group reference"))
    ));
}

#[test]
fn root_capture_without_groups_preserves_legacy_frame_and_wire_default() {
    let mut q = crate::tests::request();
    q.draws[0].brush =
        serde_json::from_value(serde_json::json!({"kind":"snapshot","afterDraws":0})).unwrap();
    let local = compile(&q, &|| false).unwrap();
    q.draws[0].brush = Brush::Snapshot {
        after_draws: 0,
        scope: SnapshotScope::Output,
    };
    assert_eq!(compile(&q, &|| false).unwrap().frame(), local.frame());
    assert_eq!(local.frame()[1], 8);
    q.opacity_groups.push(OpacityGroup {
        first_draw: 0,
        end_draw: 1,
        opacity: 1,
    });
    assert_eq!(compile(&q, &|| false).unwrap().frame()[1], 14);
    q.viewport.width = 8192;
    q.viewport.height = 2048;
    assert!(matches!(
        compile(&q, &|| false),
        Err(RasterError::Limit("opacity and snapshot pixel bytes"))
    ));
    assert!(matches!(compile(&q, &|| true), Err(RasterError::Cancelled)));
}
