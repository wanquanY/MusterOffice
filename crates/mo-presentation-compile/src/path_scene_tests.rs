use super::path_scene::*;
use mo_geometry::{Affine, Fixed, PathCommand as C, Point};
use mo_raster::*;
fn brush() -> Brush {
    Brush::Gradient {
        gradient: Gradient {
            geometry: GradientGeometry::Linear {
                start: Point {
                    x: Fixed::ZERO,
                    y: Fixed::ZERO,
                },
                end: Point {
                    x: Fixed::from_raw(32 << 32),
                    y: Fixed::ZERO,
                },
            },
            stops: vec![
                GradientStop {
                    position: 0.0,
                    srgb: [1.0, 0.0, 0.0, 1.0]
                };
                4096
            ]
            .into(),
            tile: GradientTile::Clamp,
            interpolation: GradientInterpolation::Srgb,
            alpha: GradientAlpha::Straight,
        },
    }
}
#[test]
fn construction_admits_paint_before_geometry_and_preserves_shared_ramps() {
    let b = brush();
    let mut scene = SceneBuilder::new();
    let path = [
        C::Move {
            to: Point {
                x: Fixed::ZERO,
                y: Fixed::ZERO,
            },
        },
        C::Close,
    ];
    for _ in 0..64 {
        scene
            .add(&path, Affine::IDENTITY, b.clone(), None, |i| i)
            .unwrap();
    }
    let before = (
        scene.scene.paths.len(),
        scene.scene.transforms.len(),
        scene.generated_commands,
        scene.sources.len(),
    );
    let rejected = [C::Move {
        to: Point {
            x: Fixed::from_raw(1),
            y: Fixed::ZERO,
        },
    }];
    let error = scene.add(&rejected, Affine::IDENTITY, b.clone(), None, |_| {
        panic!("rejected source materialized")
    });
    assert!(matches!(
        error,
        Err(RasterError::Limit("gradient input stops"))
    ));
    assert_eq!(
        before,
        (
            scene.scene.paths.len(),
            scene.scene.transforms.len(),
            scene.generated_commands,
            scene.sources.len()
        )
    );
    let Brush::Gradient { gradient: base } = &b else {
        panic!()
    };
    for i in &scene.scene.instances {
        let Brush::Gradient { gradient } = &i.brush else {
            panic!()
        };
        assert_eq!(base.stops.as_ptr(), gradient.stops.as_ptr());
    }
    scene
        .add(
            &path,
            Affine::IDENTITY,
            Brush::Solid { rgba: [255; 4] },
            None,
            |i| i,
        )
        .unwrap();
    assert_eq!(scene.scene.instances.len(), 65);
}
#[test]
fn empty_draws_still_consume_construction_budget_and_clips_do_not() {
    let mut scene = SceneBuilder::new();
    for _ in 0..MAX_DRAWS {
        scene
            .add(
                &[],
                Affine::IDENTITY,
                Brush::Solid { rgba: [255; 4] },
                None,
                |_| (),
            )
            .unwrap();
    }
    assert!(matches!(
        scene.add(
            &[],
            Affine::IDENTITY,
            Brush::Solid { rgba: [255; 4] },
            None,
            |_| panic!()
        ),
        Err(RasterError::Limit("draws"))
    ));
    assert_eq!(scene.scene.instances.len(), MAX_DRAWS);
    scene
        .clip(
            &FillPath {
                fill_rule: FillRule::Nonzero,
                commands: vec![],
            },
            Affine::IDENTITY,
        )
        .unwrap();
    assert_eq!(scene.scene.clips.len(), 1);
}
