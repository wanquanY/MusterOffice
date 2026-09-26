use super::*;
use crate::gradient_tests::{gradient, request};
use mo_geometry::{Fixed, Point};
fn point(x: i128, y: i128) -> Point {
    Point {
        x: Fixed::from_raw(x << 32),
        y: Fixed::from_raw(y << 32),
    }
}
fn rectangular() -> PathRasterRequest {
    let mut q = request();
    gradient(&mut q).geometry = GradientGeometry::Plane {
        plane: GradientPlane {
            origin: point(0, 0),
            x_step: point(32, 0),
            y_step: point(0, 32),
            tile_x: GradientAxisTile::Mirror,
            tile_y: GradientAxisTile::Repeat,
            uncertainty: None,
        },
        field: GradientField::Rectangular {
            edge_rates: [Fixed::from_raw(2 << 32); 4],
            uncertainty: None,
        },
    };
    q
}
fn field(q: &mut PathRasterRequest) -> (&mut [Fixed; 4], &mut Option<[Fixed; 4]>) {
    let GradientGeometry::Plane {
        field:
            GradientField::Rectangular {
                edge_rates,
                uncertainty,
            },
        ..
    } = &mut gradient(q).geometry
    else {
        panic!()
    };
    (edge_rates, uncertainty)
}
#[test]
fn rectangular_frames_share_geometry_and_ramps_and_keep_narrow_edges() {
    let mut q = rectangular();
    gradient(&mut q).interpolation = GradientInterpolation::OfficeGamma1875;
    q.draws.push(q.draws[0].clone());
    let c = compile(&q, &|| false).unwrap();
    assert_eq!(
        (c.frame()[1], c.work().gradients, c.work().gradient_draws),
        (11, 1, 2)
    );
    let start = 14 + 2 + q.paths[0].commands.len() * 7;
    assert_eq!(&c.frame()[start..start + 5], &[3, 0, 2, 0, 2]);
    assert_eq!(&c.frame()[start + 13..start + 17], &[2f32.to_bits(); 4]);
    let mut narrow = rectangular();
    field(&mut narrow).0[0] = Fixed::from_raw((1i128 << 100) + 7);
    let c = compile(&narrow, &|| false).unwrap();
    assert!(f32::from_bits(c.frame()[start + 13]) > 32768.0);
    assert!(c.work().gradient_value_error_bound < 1.0 / 1048576.0);
}
#[test]
fn maximum_field_uses_relative_error_and_rejects_uncertain_disabled_edges() {
    let mut q = rectangular();
    // Reciprocal of a 3% margin: absolute coefficient conversion error exceeds
    // the value budget, while the active field's relative error is small.
    let raw = Fixed::from_raw((100i128 << 32) / 3);
    *field(&mut q).0 = [raw; 4];
    let c = compile(&q, &|| false).unwrap();
    assert!(c.work().gradient_value_error_bound < 1.0 / 1048576.0);
    field(&mut q).0[0] = Fixed::ZERO;
    *field(&mut q).1 = Some([Fixed::from_raw(1); 4]);
    assert!(matches!(
        compile(&q, &|| false),
        Err(RasterError::Precision)
    ));
    *field(&mut q).1 = None;
    field(&mut q).0[0] = Fixed::from_raw(-1);
    assert!(matches!(
        compile(&q, &|| false),
        Err(RasterError::Invalid("negative rectangular gradient rate"))
    ));
    field(&mut q).0[0] = Fixed::from_raw(i128::MAX);
    assert!(matches!(compile(&q, &|| false), Err(RasterError::Range)));
}
