use crate::*;
use mo_common::Emu;
fn p(x: i64, y: i64) -> Point {
    Point {
        x: Fixed::emu(Emu::new(x)),
        y: Fixed::emu(Emu::new(y)),
    }
}
fn bounds(path: &[PathCommand]) -> Rect {
    path_bounds(
        path,
        Fixed::from_raw(1 << 26),
        &mut BoundsBudget::new(100000),
        &|| false,
    )
    .unwrap()
    .unwrap()
}
#[test]
fn quadratic_extremum_is_inside_control_hull_with_certified_small_excess() {
    let b = bounds(&[
        PathCommand::Move { to: p(0, 0) },
        PathCommand::Quadratic {
            control: p(100, 200),
            to: p(200, 0),
        },
    ]);
    assert_eq!(b.min, p(0, 0));
    assert_eq!(b.max, p(200, 100));
}
#[test]
fn cubic_interior_extrema_and_translated_large_coordinates_are_tight() {
    let path = [
        PathCommand::Move { to: p(0, 0) },
        PathCommand::Cubic {
            control1: p(100, 300),
            control2: p(200, 300),
            to: p(300, 0),
        },
    ];
    let b = bounds(&path);
    assert_eq!(b.min, p(0, 0));
    assert_eq!(b.max, p(300, 225));
    let offset = p(9_000_000_000_000_000, -9_000_000_000_000_000);
    let shifted = [
        PathCommand::Move {
            to: p(0, 0).translate(offset).unwrap(),
        },
        PathCommand::Cubic {
            control1: p(100, 300).translate(offset).unwrap(),
            control2: p(200, 300).translate(offset).unwrap(),
            to: p(300, 0).translate(offset).unwrap(),
        },
    ];
    assert_eq!(bounds(&shifted), b.translate(offset).unwrap());
}
#[test]
fn negative_odd_midpoints_never_cut_a_curve_and_non_dyadic_extrema_refine() {
    // Cubic y=3t(1-t)(1-2t), extrema +/-sqrt(3)/6.
    let b = bounds(&[
        PathCommand::Move { to: p(0, 0) },
        PathCommand::Cubic {
            control1: p(1, 1),
            control2: p(2, -1),
            to: p(3, 0),
        },
    ]);
    // Rational brackets 0.288675 < sqrt(3)/6 < 0.288676.
    let lo = (288675i128 << 32) / 1_000_000;
    let hi = (288676i128 << 32) / 1_000_000;
    assert!(b.max.y.raw() >= lo && b.max.y.raw() <= hi + (1 << 26));
    assert!(b.min.y.raw() <= -lo && b.min.y.raw() >= -hi - (1 << 26));
    let small = Point {
        x: Fixed::from_raw(-1),
        y: Fixed::from_raw(-3),
    };
    let path = [
        PathCommand::Move { to: small },
        PathCommand::Quadratic {
            control: p(-10, 9),
            to: p(3, -1),
        },
    ];
    let b = bounds(&path);
    assert!(b.min.x <= small.x && b.max.y >= small.y);
}
#[test]
fn move_only_empty_closed_degenerate_and_multiple_open_contours_are_distinct() {
    assert_eq!(
        path_bounds(
            &[PathCommand::Move { to: p(99, 99) }],
            Fixed::from_raw(256),
            &mut BoundsBudget::new(10),
            &|| false
        )
        .unwrap(),
        None
    );
    let b = bounds(&[
        PathCommand::Move { to: p(99, 99) },
        PathCommand::Move { to: p(1, 2) },
        PathCommand::Close,
        PathCommand::Move { to: p(-3, -4) },
        PathCommand::Line { to: p(5, 6) },
    ]);
    assert_eq!(
        b,
        Rect {
            min: p(-3, -4),
            max: p(5, 6)
        }
    );
    assert!(
        path_bounds(
            &[PathCommand::Close],
            Fixed::from_raw(256),
            &mut BoundsBudget::new(10),
            &|| false
        )
        .is_err()
    );
}
#[test]
fn budgets_cancel_without_result_and_wire_coordinate_is_lossless() {
    let path = [
        PathCommand::Move { to: p(0, 0) },
        PathCommand::Quadratic {
            control: p(100, 200),
            to: p(200, 0),
        },
    ];
    assert!(matches!(
        path_bounds(
            &path,
            Fixed::from_raw(256),
            &mut BoundsBudget::new(2),
            &|| false
        ),
        Err(GeometryError::Limit(_))
    ));
    assert!(matches!(
        path_bounds(
            &path,
            Fixed::from_raw(256),
            &mut BoundsBudget::new(100),
            &|| true
        ),
        Err(GeometryError::Cancelled)
    ));
    for v in [i128::MIN, -1, 0, 1, i128::MAX] {
        let f = Fixed::from_raw(v);
        let json = serde_json::to_string(&f).unwrap();
        assert_eq!(serde_json::from_str::<Fixed>(&json).unwrap(), f);
    }
    for s in [
        "\"-0\"",
        "\"+1\"",
        "\"01\"",
        "\"1\\n\"",
        "1",
        "\"170141183460469231731687303715884105728\"",
    ] {
        assert!(serde_json::from_str::<Fixed>(s).is_err());
    }
}
#[test]
fn extreme_i128_coordinates_do_not_overflow_midpoint_intervals() {
    let min = Point {
        x: Fixed::from_raw(i128::MIN),
        y: Fixed::ZERO,
    };
    let max = Point {
        x: Fixed::from_raw(i128::MAX),
        y: Fixed::ZERO,
    };
    assert_eq!(
        bounds(&[
            PathCommand::Move { to: min },
            PathCommand::Cubic {
                control1: min,
                control2: max,
                to: max
            }
        ]),
        Rect { min, max }
    );
}
