use super::gradient_tests::{gradient, request};
use super::*;
use mo_geometry::{Fixed, Point};
fn point(x: i128, y: i128) -> Point {
    Point {
        x: Fixed::from_raw(x << 32),
        y: Fixed::from_raw(y << 32),
    }
}
fn plane(q: &mut PathRasterRequest) -> (&mut GradientPlane, &mut GradientField) {
    let GradientGeometry::Plane { plane, field } = &mut gradient(q).geometry else {
        panic!()
    };
    (plane, field)
}
fn mapped() -> PathRasterRequest {
    let mut q = request();
    gradient(&mut q).geometry = GradientGeometry::Plane {
        plane: GradientPlane {
            origin: point(0, 0),
            x_step: point(32, 8),
            y_step: point(0, 16),
            tile_x: GradientAxisTile::Mirror,
            tile_y: GradientAxisTile::Repeat,
            uncertainty: None,
        },
        field: GradientField::Linear {
            coefficients: [
                Fixed::from_raw(1 << 31),
                Fixed::from_raw(1 << 31),
                Fixed::ZERO,
            ],
            uncertainty: None,
        },
    };
    q
}
#[test]
fn mapped_gradients_are_versioned_and_keep_independent_basis_and_tiles() {
    let mut q = mapped();
    q.draws.push(q.draws[0].clone());
    let c = compile(&q, &|| false).unwrap();
    assert_eq!(c.frame()[1], 9);
    assert_eq!(c.frame()[9], 1);
    assert_eq!(c.work().gradient_value_error_bound, 0.0);
    assert_eq!(c.work().gradient_coordinate_error_bound, Fixed::ZERO);
    assert!(c.work().compositing.is_none());
    let start = 14 + 2 + 7 * q.paths[0].commands.len();
    assert_eq!(&c.frame()[start..start + 5], &[2, 0, 0, 0, 2]);
    assert_eq!(&c.frame()[start + 11..start + 13], &[2, 1]);
    assert_eq!(c.frame().len(), start + 16 + 10 + 16);
    let serialized = serde_json::to_vec(&q).unwrap();
    let roundtrip: PathRasterRequest = serde_json::from_slice(&serialized).unwrap();
    assert_eq!(c.frame(), compile(&roundtrip, &|| false).unwrap().frame());
}
#[test]
fn mapped_gradient_uncertainty_rejects_singular_or_unbounded_fields() {
    for change in 0..6 {
        let mut q = mapped();
        let (p, f) = plane(&mut q);
        match change {
            0 => p.y_step = p.x_step,
            1 => {
                p.uncertainty = Some(Box::new(GradientPlaneUncertainty {
                    origin: Point {
                        x: Fixed::from_raw(-1),
                        y: Fixed::ZERO,
                    },
                    x_step: point(0, 0),
                    y_step: point(0, 0),
                }))
            }
            2 => p.x_step.x = Fixed::from_raw(1),
            3 => {
                let GradientField::Linear { uncertainty, .. } = f else {
                    panic!()
                };
                *uncertainty = Some([Fixed::from_raw(1 << 32); 3]);
            }
            4 => {
                let GradientField::Linear { uncertainty, .. } = f else {
                    panic!()
                };
                *uncertainty = Some([Fixed::from_raw(-1); 3]);
            }
            _ => p.origin.x = Fixed::from_raw(32769 << 32),
        }
        assert!(compile(&q, &|| false).is_err(), "case {change}");
    }
}
#[test]
fn mapped_gradient_rebase_retains_world_basis_and_uncertainty() {
    let q = mapped();
    let before = compile(&q, &|| false).unwrap();
    let huge = Point {
        x: Fixed::from_raw(1 << 110),
        y: Fixed::from_raw(-(1 << 110)),
    };
    let mut shifted = q.clone();
    shifted.viewport.origin = huge;
    shifted.draws[0].origin = huge;
    plane(&mut shifted).0.origin = huge;
    assert_eq!(
        before.frame(),
        compile(&shifted, &|| false).unwrap().frame()
    );
    assert_eq!(
        shifted.draws[0].brush.rebased(huge).unwrap(),
        q.draws[0].brush
    );
}
#[test]
fn mapped_gradient_errors_cover_accumulated_tile_phase() {
    let mut q = mapped();
    let (p, _) = plane(&mut q);
    p.x_step = Point {
        x: Fixed::from_raw(1 << 32),
        y: Fixed::ZERO,
    };
    p.y_step = Point {
        x: Fixed::ZERO,
        y: Fixed::from_raw(1 << 32),
    };
    p.uncertainty = Some(Box::new(GradientPlaneUncertainty {
        origin: point(0, 0),
        x_step: Point {
            x: Fixed::from_raw(1 << 12),
            y: Fixed::ZERO,
        },
        y_step: point(0, 0),
    }));
    let close = compile(&q, &|| false)
        .unwrap()
        .work()
        .gradient_coordinate_error_bound;
    assert!(close > Fixed::from_raw(1 << 12));
    q.viewport.width = 8192;
    assert!(matches!(
        compile(&q, &|| false),
        Err(RasterError::Precision)
    ));
    plane(&mut q).0.tile_x = GradientAxisTile::Clamp;
    assert!(compile(&q, &|| false).is_ok());
}
