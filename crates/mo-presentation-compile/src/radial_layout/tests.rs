use super::*;
use mo_geometry::{PathCommand as C, Point};
use mo_presentation_source::source::geometry::evaluate::GeometryOrigin;
fn pt(x: i128, y: i128) -> Point {
    Point {
        x: Fixed::from_raw(x << 32),
        y: Fixed::from_raw(y << 32),
    }
}
fn path() -> CompiledNativePath {
    CompiledNativePath {
        origin: GeometryOrigin::document(0),
        commands: vec![C::Move { to: pt(-100, 200) }, C::Line { to: pt(300, 500) }],
        source_map: vec![],
        fill: None,
        stroke: None,
        extrusion_ok: None,
        numeric_error_bound: pt(0, 0),
        curve_error_bound: pt(0, 0),
        coordinate_error_bound: pt(10, 20),
        arc_segments: 0,
    }
}
#[test]
fn radial_bounds_propagate_each_input_axis_and_do_not_bound_control_hulls() {
    let p = path();
    let (b, w) = bounds::collect(
        &[p],
        Fixed::from_raw(256),
        &mut BoundsBudget::new(100),
        &|| false,
    )
    .unwrap();
    for (i, (lo, hi)) in [(-110, -90), (180, 220), (290, 310), (480, 520)]
        .into_iter()
        .enumerate()
    {
        assert_eq!(b[i].lo, I::integer(lo).lo);
        assert_eq!(b[i].hi, I::integer(hi).hi);
    }
    assert_eq!(w.paths, 1);
    let mut p = path();
    p.coordinate_error_bound = pt(0, 0);
    p.commands = vec![
        C::Move { to: pt(0, 0) },
        C::Quadratic {
            control: pt(100, 200),
            to: pt(200, 0),
        },
        C::Close,
    ];
    let (b, _) = bounds::collect(
        &[p],
        Fixed::from_raw(256),
        &mut BoundsBudget::new(10000),
        &|| false,
    )
    .unwrap();
    assert!(b[3].lo <= I::integer(100).lo && b[3].hi >= I::integer(100).hi);
    assert!(b[3].hi < I::integer(101).hi);
}
#[test]
fn radial_bounds_reject_invalid_uncertainty_and_consume_one_shared_budget() {
    let mut p = path();
    p.coordinate_error_bound.x = Fixed::from_raw(-1);
    assert!(matches!(
        bounds::collect(
            &[p],
            Fixed::from_raw(256),
            &mut BoundsBudget::new(100),
            &|| false
        ),
        Err(RadialLayoutError::Invalid(_))
    ));
    let p = path();
    let mut budget = BoundsBudget::new(3);
    bounds::collect(
        std::slice::from_ref(&p),
        Fixed::from_raw(256),
        &mut budget,
        &|| false,
    )
    .unwrap();
    assert!(bounds::collect(&[p], Fixed::from_raw(256), &mut budget, &|| false).is_err());
}
