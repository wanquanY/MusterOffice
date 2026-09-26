use super::*;
use mo_geometry::{Fixed, Point};
use std::cell::Cell;

fn request() -> PathRasterRequest {
    let mut q = tests::request();
    q.clips = vec![
        PathClip {
            parent: None,
            path: 0,
            origin: q.viewport.origin,
        },
        PathClip {
            parent: Some(0),
            path: 0,
            origin: q.viewport.origin,
        },
        PathClip {
            parent: Some(0),
            path: 0,
            origin: q.viewport.origin,
        },
    ];
    let d = q.draws[0].clone();
    q.draws = [Some(1), Some(1), Some(2), None, Some(1)]
        .into_iter()
        .map(|clip| PathDraw { clip, ..d.clone() })
        .collect();
    q
}

#[test]
fn common_ancestors_are_counted_once_until_a_branch_is_left() {
    let q = request();
    let c = compile(&q, &|| false).unwrap();
    assert_eq!(c.frame()[1], 7);
    let w = c.work().clips.as_ref().unwrap();
    assert_eq!(
        (
            w.nodes,
            w.maximum_depth,
            w.applications,
            w.applied_commands,
            w.placement_commands
        ),
        (3, 2, 5, 20, 12)
    );
    assert_eq!(&c.frame()[10..13], &[0, 0, 3]);
    let legacy = compile(&tests::request(), &|| false).unwrap();
    assert_eq!(legacy.frame()[1], 4);
    assert!(
        !serde_json::to_value(legacy.work())
            .unwrap()
            .as_object()
            .unwrap()
            .contains_key("clips")
    );
}

#[test]
fn invalid_and_unused_clip_resources_are_rejected() {
    for mutation in 0..5 {
        let mut q = request();
        match mutation {
            0 => q.clips[0].parent = Some(0),
            1 => q.clips[1].parent = Some(2),
            2 => q.clips[2].path = u32::MAX,
            3 => q.draws[0].clip = Some(3),
            _ => q.clips.push(PathClip {
                parent: None,
                path: 1,
                origin: q.viewport.origin,
            }),
        }
        assert!(matches!(
            compile(&q, &|| false),
            Err(RasterError::Invalid(_))
        ));
    }
}

#[test]
fn clip_work_is_bounded_even_for_empty_paths_and_reused_resources() {
    let mut q = request();
    q.paths[0].commands.clear();
    q.clips = (0u32..64)
        .map(|i| PathClip {
            parent: i.checked_sub(1),
            path: 0,
            origin: q.viewport.origin,
        })
        .collect();
    q.draws = vec![
        PathDraw {
            blend: Default::default(),
            clip: Some(63),
            ..q.draws[0].clone()
        };
        65536
    ];
    assert_eq!(
        compile(&q, &|| false)
            .unwrap()
            .work()
            .clips
            .as_ref()
            .unwrap()
            .applications,
        64
    );
    for d in q.draws.iter_mut().step_by(2) {
        d.clip = None;
    }
    assert!(matches!(
        compile(&q, &|| false),
        Err(RasterError::Limit("applied clip work"))
    ));
    q.draws.clear();
    q.clips.push(PathClip {
        parent: Some(63),
        path: 0,
        origin: q.viewport.origin,
    });
    assert!(matches!(
        compile(&q, &|| false),
        Err(RasterError::Limit("clip depth"))
    ));
    q.clips.resize(8193, q.clips[0].clone());
    assert!(matches!(
        compile(&q, &|| false),
        Err(RasterError::Limit("clip nodes"))
    ));
    let mut q = request();
    q.paths[0].commands = vec![
        mo_geometry::PathCommand::Move {
            to: q.viewport.origin
        };
        262144
    ];
    q.draws.clear();
    q.clips.resize(5, q.clips[0].clone());
    assert!(matches!(
        compile(&q, &|| false),
        Err(RasterError::Limit("clip placement commands"))
    ));
}

#[test]
fn clips_share_geometry_range_precision_and_cancellation_checks() {
    let q = request();
    let calls = Cell::new(0usize);
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
    let mut q = q;
    q.clips[2].origin = Point {
        x: Fixed::from_raw(32768i128 << 32),
        y: Fixed::ZERO,
    };
    assert!(matches!(compile(&q, &|| false), Err(RasterError::Range)));
    q.clips[2].origin.x = Fixed::from_raw((16000i128 << 32) + 257);
    q.viewport.coordinate_tolerance = Fixed::from_raw(256);
    assert!(matches!(
        compile(&q, &|| false),
        Err(RasterError::Precision)
    ));
}
