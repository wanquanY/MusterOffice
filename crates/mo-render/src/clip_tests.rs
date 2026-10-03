use super::*;
use mo_geometry::{Affine, Fixed};
use std::cell::Cell;

#[test]
fn semantic_clip_users_survive_pruning_with_explicit_source_identity() {
    let mut q = tests::request();
    q.scene.clips = vec![
        ClipNode {
            parent: None,
            path: 0,
            transform: Some(1),
        },
        ClipNode {
            parent: None,
            path: 0,
            transform: Some(2),
        },
    ];
    for draw in &mut q.scene.instances {
        draw.clip = Some(1);
    }
    let images = mo_raster::PreparedImages::new(&[], &[], &|| false).unwrap();
    let ordinary = compile_images(&q, &images, &|| false).unwrap();
    assert_eq!(ordinary.lowered_clip(0), None);
    assert_eq!(ordinary.lowered_clip(1), Some(0));
    let retained = compile_images_retaining_clips(&q, &images, &[0], &|| false).unwrap();
    assert_eq!(retained.lowered_clip(0), Some(0));
    assert_eq!(retained.lowered_clip(1), Some(1));
    assert_eq!(retained.work().clips.as_ref().unwrap().compiled_nodes, 2);
    assert_eq!(
        retained.raster().picking(&|| false).unwrap().draw_count(),
        q.scene.instances.len() as u32
    );
    assert!(matches!(
        compile_images_retaining_clips(&q, &images, &[2], &|| false),
        Err(RasterError::Invalid(_))
    ));
    q.scene.instances.clear();
    let retained = compile_images_retaining_clips(&q, &images, &[0], &|| false).unwrap();
    assert_eq!(retained.lowered_clip(0), Some(0));
    assert_eq!(retained.lowered_clip(1), None);
    assert_eq!(
        retained.raster().picking(&|| false).unwrap().draw_count(),
        0
    );
}

#[test]
fn clip_and_draw_geometry_share_resources_and_original_transform_chain() {
    let mut q = tests::request();
    q.scene.clips = vec![
        ClipNode {
            parent: None,
            path: 0,
            transform: Some(1),
        },
        ClipNode {
            parent: Some(0),
            path: 0,
            transform: Some(2),
        },
    ];
    for i in &mut q.scene.instances {
        i.clip = Some(1);
    }
    let c = compile(&q, &|| false).unwrap();
    assert_eq!(c.work().compiled_paths, 1);
    assert_eq!(c.raster().work().clips.as_ref().unwrap().applications, 2);
    assert_eq!(c.work().clips.as_ref().unwrap().compiled_nodes, 2);
    assert_eq!(c.work().combined_coordinate_error_bound, Fixed::ZERO);
    let mut q = tests::precision_request();
    q.scene.clips.push(ClipNode {
        parent: None,
        path: 0,
        transform: Some(1),
    });
    let mut small = q.scene.paths[0].clone();
    small.commands = vec![mo_geometry::PathCommand::Move {
        to: q.viewport.origin,
    }];
    q.scene.paths.push(small);
    q.scene.instances[0].path = 1;
    q.scene.instances[0].transform = None;
    q.scene.instances[0].clip = Some(0);
    let c = compile(&q, &|| false).unwrap();
    assert_eq!(c.work().lowering_attempts, 2);
    assert!(c.work().combined_coordinate_error_bound.raw() <= 256);
}

#[test]
fn unused_clips_are_pruned_but_their_references_are_validated_and_empty_clips_survive() {
    let mut q = tests::request();
    let baseline = compile(&q, &|| false).unwrap().raster().frame().to_vec();
    q.scene.transforms.push(TransformNode {
        parent: None,
        affine: Affine {
            translation: mo_geometry::Point {
                x: Fixed::from_raw(i128::MAX),
                y: Fixed::from_raw(i128::MIN),
            },
            ..Affine::IDENTITY
        },
    });
    q.scene.clips.push(ClipNode {
        parent: None,
        path: 0,
        transform: Some(3),
    });
    let c = compile(&q, &|| false).unwrap();
    assert_eq!(c.raster().frame(), baseline);
    assert_eq!(c.work().clips.as_ref().unwrap().compiled_nodes, 0);
    q.scene.clips[0].path = 1;
    assert!(compile(&q, &|| false).is_err());
    q.scene.paths.push(mo_raster::FillPath {
        fill_rule: mo_raster::FillRule::Nonzero,
        commands: vec![],
    });
    q.scene.instances[0].clip = Some(0);
    assert_eq!(
        compile(&q, &|| false)
            .unwrap()
            .raster()
            .work()
            .clips
            .as_ref()
            .unwrap()
            .nodes,
        1
    );
}

#[test]
fn clip_lowering_cancellation_is_atomic_at_every_checkpoint() {
    let mut q = tests::request();
    q.scene.clips.push(ClipNode {
        parent: None,
        path: 0,
        transform: Some(2),
    });
    q.scene.instances[0].clip = Some(0);
    let total = Cell::new(0);
    compile(&q, &|| {
        total.set(total.get() + 1);
        false
    })
    .unwrap();
    for stop in 1..=total.get() {
        let n = Cell::new(0);
        assert!(matches!(
            compile(&q, &|| {
                n.set(n.get() + 1);
                n.get() == stop
            }),
            Err(mo_raster::RasterError::Cancelled)
        ));
    }
}
