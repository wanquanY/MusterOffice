use crate::{Affine, Fixed, Point, PointEstimate};
const U: i128 = 1 << 32;
fn p(x: i128, y: i128) -> Point {
    Point {
        x: Fixed::from_raw(x),
        y: Fixed::from_raw(y),
    }
}
fn matrix(a: [i128; 4], t: Point) -> Affine {
    Affine {
        linear: a.map(Fixed::from_raw),
        translation: t,
    }
}
#[test]
fn exact_quarter_turn_flip_and_composition_order() {
    let rotate = matrix([0, -U, U, 0], p(10 * U, 20 * U));
    let scale = matrix([-2 * U, 0, 0, 3 * U], p(U, 2 * U));
    let a = rotate.compose(scale).unwrap();
    let input = p(4 * U, 5 * U);
    assert_eq!(a.map(input).unwrap().point, p(-7 * U, 13 * U));
    assert_eq!(
        a.map(input).unwrap(),
        rotate.map_estimate(scale.map(input).unwrap()).unwrap()
    );
    assert_ne!(a, scale.compose(rotate).unwrap());
}
#[test]
fn huge_products_cancel_and_only_final_coordinate_range_matters() {
    let huge = 1 << 110;
    let a = matrix([1 << 100, -(1 << 100) + U, 0, U], p(-huge, 0));
    assert_eq!(a.map(p(huge + U, huge + U)).unwrap().point, p(U, huge + U));
    let a = matrix([U, -U, 0, 0], p(0, 0));
    assert_eq!(
        a.map_vector_from(p(i128::MAX, i128::MAX), p(i128::MIN, i128::MIN))
            .unwrap(),
        p(0, 0)
    );
    assert!(
        matrix([2 * U, 0, 0, U], p(0, 0))
            .map(p(i128::MAX, 0))
            .is_err()
    );
    assert_eq!(
        Fixed::sum_deviation(
            Fixed::from_raw(i128::MAX),
            Fixed::from_raw(i128::MAX),
            Fixed::from_raw(i128::MAX)
        )
        .unwrap()
        .raw(),
        i128::MAX
    );
}
#[test]
fn rounding_is_after_translation_and_error_is_propagated_outward() {
    let a = matrix([U / 2, 0, 0, U / 2], p(1, -1));
    let r = a.map(p(-1, 1)).unwrap();
    assert_eq!(r.point, p(1, -1));
    assert_eq!(r.error, p(1, 1));
    let b = matrix([-3 * U, 2 * U, 0, U / 2], p(0, 0));
    let r = b
        .map_estimate(PointEstimate {
            point: p(1, 1),
            error: p(2, 3),
        })
        .unwrap();
    assert_eq!(r.point, p(-1, 1));
    assert_eq!(r.error, p(12, 3));
    assert!(
        b.map_estimate(PointEstimate {
            point: p(0, 0),
            error: p(-1, 0)
        })
        .is_err()
    );
    assert_eq!(
        Fixed::from_raw(i128::MAX)
            .ratio_up(u32::MAX, u32::MAX)
            .unwrap()
            .raw(),
        i128::MAX
    );
    assert_eq!(Fixed::from_raw(1).ratio_up(1, 3).unwrap().raw(), 1);
}
#[test]
fn root_viewport_cancellation_precedes_narrowing() {
    let a = matrix([U, 0, 0, U], p(i128::MAX, i128::MIN));
    let point = PointEstimate {
        point: p(8 * U, -8 * U),
        error: p(0, 0),
    };
    assert!(a.map(point.point).is_err());
    assert_eq!(
        a.map_in_view(point, p(i128::MAX, i128::MIN)).unwrap(),
        point
    );
    let a = matrix([U, 0, 0, U], p(i128::MAX, 0));
    let p0 = PointEstimate {
        point: p(i128::MIN, 0),
        error: p(0, 0),
    };
    assert_eq!(a.map_in_view(p0, p(-U, 0)).unwrap().point, p(U - 1, 0));
}
