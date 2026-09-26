use super::*;
use crate::gradient_tests::{gradient, request};
use mo_geometry::{Fixed, Point};
fn fixed(v: i128) -> Fixed {
    Fixed::from_raw(v << 32)
}
fn elliptic() -> PathRasterRequest {
    let mut q = request();
    gradient(&mut q).geometry = GradientGeometry::Plane {
        plane: GradientPlane {
            origin: Point {
                x: fixed(0),
                y: fixed(0),
            },
            x_step: Point {
                x: fixed(32),
                y: fixed(0),
            },
            y_step: Point {
                x: fixed(0),
                y: fixed(16),
            },
            tile_x: GradientAxisTile::Mirror,
            tile_y: GradientAxisTile::Clamp,
            uncertainty: None,
        },
        field: GradientField::Elliptic {
            tile_scale: [fixed(1), Fixed::from_raw(1 << 31)],
            inner_center: [fixed(0); 2],
            inner_radii: [fixed(0); 2],
            uncertainty: None,
        },
    };
    q
}
fn field(q: &mut PathRasterRequest) -> &mut GradientField {
    let GradientGeometry::Plane { field, .. } = &mut gradient(q).geometry else {
        panic!()
    };
    field
}
#[test]
fn elliptic_frame_is_v12_deduplicated_and_preserves_ramps() {
    let mut q = elliptic();
    gradient(&mut q).interpolation = GradientInterpolation::OfficeGamma1875;
    q.draws.push(q.draws[0].clone());
    let c = compile(&q, &|| false).unwrap();
    assert_eq!(
        (c.frame()[1], c.work().gradients, c.work().gradient_draws),
        (12, 1, 2)
    );
    let at = 14 + 2 + q.paths[0].commands.len() * 7;
    assert_eq!(&c.frame()[at..at + 5], &[4, 0, 2, 0, 2]);
    assert_eq!(
        &c.frame()[at + 13..at + 19],
        &[1f32.to_bits(), 0.5f32.to_bits(), 0, 0, 0, 0]
    );
    let w = c.work().elliptic_gradients.as_ref().unwrap();
    assert_eq!(w.parameter_error_bounds, [Fixed::ZERO; 6]);
    assert_eq!(w.coordinate_error_bound, Fixed::ZERO);
    assert_eq!(w.encoded_root_interval_bound.raw(), 512);
    assert!(
        compile(&request(), &|| false)
            .unwrap()
            .work()
            .elliptic_gradients
            .is_none()
    );
}
#[test]
fn ellipse_uncertainty_is_geometry_not_a_claimed_scalar_error() {
    let mut q = elliptic();
    let GradientField::Elliptic {
        inner_center,
        uncertainty,
        ..
    } = field(&mut q)
    else {
        panic!()
    };
    inner_center[0] = Fixed::from_raw((1 << 32) / 3);
    *uncertainty = Some(Box::new([Fixed::from_raw(1); 6]));
    let c = compile(&q, &|| false).unwrap();
    assert_eq!(c.work().gradient_value_error_bound, 0.0);
    let w = c.work().elliptic_gradients.as_ref().unwrap();
    assert!(w.parameter_error_bounds[2].raw() > 1);
    assert!(w.coordinate_error_bound > Fixed::ZERO);
    assert!(w.coordinate_error_bound <= q.viewport.coordinate_tolerance);
    let GradientField::Elliptic { uncertainty, .. } = field(&mut q) else {
        panic!()
    };
    *uncertainty = Some(Box::new([Fixed::from_raw(1 << 20); 6]));
    assert!(matches!(
        compile(&q, &|| false),
        Err(RasterError::Precision)
    ));
}
#[test]
fn elliptic_domain_and_singular_uncertainty_fail_before_backend() {
    for (index, value) in [(0, 0), (1, (1 << 32) + 1), (4, -1), (5, -1)] {
        let mut q = elliptic();
        let GradientField::Elliptic {
            tile_scale,
            inner_radii,
            ..
        } = field(&mut q)
        else {
            panic!()
        };
        if index < 2 {
            tile_scale[index] = Fixed::from_raw(value);
        } else {
            inner_radii[index - 4] = Fixed::from_raw(value);
        }
        assert!(matches!(
            compile(&q, &|| false),
            Err(RasterError::Invalid(_))
        ));
    }
    let mut q = elliptic();
    let GradientField::Elliptic { uncertainty, .. } = field(&mut q) else {
        panic!()
    };
    *uncertainty = Some(Box::new([
        fixed(1),
        fixed(0),
        fixed(0),
        fixed(0),
        fixed(0),
        fixed(0),
    ]));
    assert!(matches!(
        compile(&q, &|| false),
        Err(RasterError::Precision)
    ));
}
