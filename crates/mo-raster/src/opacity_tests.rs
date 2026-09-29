use super::*;
fn group(first_draw: u32, end_draw: u32) -> OpacityGroup {
    OpacityGroup {
        first_draw,
        end_draw,
        opacity: 32768,
    }
}
fn request() -> PathRasterRequest {
    let mut q = crate::tests::request();
    q.draws.resize(4, q.draws[0].clone());
    q.opacity_groups = vec![group(0, 3), group(0, 1), group(1, 3), group(3, 4)];
    q
}
#[test]
fn nested_groups_are_isolated_intervals_and_old_requests_keep_their_wire() {
    let mut q = request();
    let frame = compile(&q, &|| false).unwrap();
    let work = frame.work().opacity_groups.as_ref().unwrap();
    assert_eq!(frame.frame()[1], 13);
    assert_eq!((work.groups, work.maximum_depth), (4, 2));
    assert_eq!(work.peak_pixel_bytes as usize, frame.pixel_bytes() * 2);
    assert_eq!(work.pixel_work as usize, frame.pixel_bytes() * 2);
    let wire = serde_json::to_string(&q).unwrap();
    assert_eq!(
        serde_json::from_str::<PathRasterRequest>(&wire)
            .unwrap()
            .opacity_groups,
        q.opacity_groups
    );
    q.opacity_groups.clear();
    let legacy = compile(&q, &|| false).unwrap();
    assert_eq!(legacy.frame()[1], 4);
    assert!(legacy.work().opacity_groups.is_none());
    assert!(!serde_json::to_string(&q).unwrap().contains("opacityGroups"));
}
#[test]
fn crossing_reversed_empty_or_out_of_range_groups_are_rejected() {
    for groups in [
        vec![group(0, 2), group(1, 3)],
        vec![group(1, 3), group(0, 4)],
        vec![group(0, 0)],
        vec![group(3, 2)],
        vec![group(0, 5)],
        vec![group(4, 5)],
    ] {
        let mut q = request();
        q.opacity_groups = groups;
        assert!(matches!(
            compile(&q, &|| false),
            Err(RasterError::Invalid(_))
        ));
    }
    let mut q = request();
    q.opacity_groups = vec![group(0, 4); 65];
    assert!(matches!(
        compile(&q, &|| false),
        Err(RasterError::Limit("opacity group depth"))
    ));
}
#[test]
fn snapshots_are_local_and_clip_state_is_rebuilt_after_scope_changes() {
    let mut q = request();
    q.clips.push(PathClip {
        parent: None,
        path: 0,
        origin: q.draws[0].origin,
    });
    for d in &mut q.draws {
        d.clip = Some(0);
    }
    q.opacity_groups = vec![group(1, 3)];
    q.draws[2].brush = Brush::Snapshot {
        after_draws: 1,
        scope: SnapshotScope::Current,
    };
    let c = compile(&q, &|| false).unwrap();
    assert_eq!(c.work().clips.as_ref().unwrap().applications, 3);
    q.draws[2].brush = Brush::Snapshot {
        after_draws: 0,
        scope: SnapshotScope::Current,
    };
    assert!(matches!(
        compile(&q, &|| false),
        Err(RasterError::Invalid("snapshot crosses opacity group scope"))
    ));
    q.draws[2].brush = q.draws[0].brush.clone();
    q.draws[3].brush = Brush::Snapshot {
        after_draws: 1,
        scope: SnapshotScope::Current,
    };
    assert!(compile(&q, &|| false).is_err());
}
#[test]
fn group_memory_is_shared_with_snapshots_and_total_pixel_work_is_bounded() {
    let mut q = request();
    q.viewport.width = 4096;
    q.viewport.height = 2048;
    q.opacity_groups = vec![group(0, 4), group(1, 3)];
    assert_eq!(
        compile(&q, &|| false)
            .unwrap()
            .work()
            .opacity_groups
            .as_ref()
            .unwrap()
            .peak_pixel_bytes,
        64 * 1024 * 1024
    );
    q.draws[2].brush = Brush::Snapshot {
        after_draws: 1,
        scope: SnapshotScope::Current,
    };
    assert!(matches!(
        compile(&q, &|| false),
        Err(RasterError::Limit("opacity and snapshot pixel bytes"))
    ));
    q.draws[2].brush = q.draws[0].brush.clone();
    q.draws.resize(17, q.draws[0].clone());
    q.opacity_groups = (0..17).map(|i| group(i, i + 1)).collect();
    assert!(matches!(
        compile(&q, &|| false),
        Err(RasterError::Limit("opacity group pixel work"))
    ));
}
#[test]
fn every_group_compilation_checkpoint_cancels_without_publishing_a_frame() {
    use std::cell::Cell;
    let q = request();
    let calls = Cell::new(0);
    compile(&q, &|| {
        calls.set(calls.get() + 1);
        false
    })
    .unwrap();
    for stop in 1..=calls.get() {
        let n = Cell::new(0);
        assert!(matches!(
            compile(&q, &|| {
                n.set(n.get() + 1);
                n.get() == stop
            }),
            Err(RasterError::Cancelled)
        ));
    }
}
