use super::*;
use mo_geometry::{Fixed, Point};
use serde_json::json;
use std::cell::Cell;
const U: i128 = 1 << 32;
fn request() -> PagePlacementRequest {
    let mut d: serde_json::Value = serde_json::from_str(include_str!(
        "../../../fixtures/presentations/basic-shape.json"
    ))
    .unwrap();
    d["objects"]["shape:1"]["content"]["text"] = serde_json::Value::Null;
    d["objects"]["shape:1"]["transform"]["size"] = json!({"width":"20","height":"20"});
    serde_json::from_value(json!({"document":d,"slide":"slide:1"})).unwrap()
}
fn grouped() -> PagePlacementRequest {
    let mut q = serde_json::to_value(request()).unwrap();
    let d = &mut q["document"];
    d["slides"]["slide:1"]["objects"] = json!(["group:1"]);
    let mut group = d["objects"]["shape:1"].clone();
    group["id"] = json!("group:1");
    group["transform"]["size"] = json!({"width":"200","height":"100"});
    group["content"] =
        json!({"kind":"group","children":["shape:1"],"viewport":{"width":"100","height":"100"}});
    d["objects"]["group:1"] = group;
    d["objects"]["shape:1"]["parent"] = json!({"kind":"group","id":"group:1"});
    d["objects"]["shape:1"]["transform"]["origin"] = json!({"x":"40","y":"40"});
    d["objects"]["shape:1"]["transform"]["rotation"] = json!(5400000);
    serde_json::from_value(q).unwrap()
}
#[test]
fn grouped_shape_orientation_does_not_inherit_parent_shear() {
    let q = grouped();
    let before = serde_json::to_string(&q).unwrap();
    let r = page_placements(&q, &|| false).unwrap();
    let p = &r.surfaces[0].objects[1];
    assert_eq!(p.affine.linear.map(Fixed::raw), [0, -2 * U, U, 0]);
    assert_eq!(
        p.affine.translation,
        Point {
            x: Fixed::from_raw(100 * U),
            y: Fixed::from_raw(50 * U)
        }
    );
    assert_eq!(p.anchor.x.raw(), 10 * U);
    assert_eq!(p.uncertainty.linear.map(Fixed::raw), [0; 4]);
    assert_eq!(serde_json::to_string(&q).unwrap(), before);
    assert_eq!(r.document_sha256, q.document.semantic_digest().unwrap());
}
#[test]
fn reflected_parent_reverses_child_rotation_without_swapping_flip_axes() {
    let mut q = grouped();
    q.document
        .objects
        .get_mut(&mo_common::ObjectId::new("group:1").unwrap())
        .unwrap()
        .transform
        .flip_horizontal = true;
    let r = page_placements(&q, &|| false).unwrap();
    assert_eq!(
        r.surfaces[0].objects[1].affine.linear.map(Fixed::raw),
        [0, 2 * U, U, 0]
    );
}
#[test]
fn nested_child_center_uses_the_resolved_group_space() {
    let mut v = serde_json::to_value(grouped()).unwrap();
    let objects = &mut v["document"]["objects"];
    let mut inner = objects["group:1"].clone();
    inner["id"] = json!("group:inner");
    inner["parent"] = json!({"kind":"group","id":"group:1"});
    inner["transform"] = json!({
        "origin":{"x":"10","y":"10"}, "size":{"width":"20","height":"40"},
        "rotation":5400000, "flipHorizontal":true, "flipVertical":false
    });
    inner["content"]["viewport"] = json!({"width":"20","height":"40"});
    objects["group:1"]["content"]["children"] = json!(["group:inner"]);
    objects["group:inner"] = inner;
    objects["shape:1"]["parent"] = json!({"kind":"group","id":"group:inner"});
    objects["shape:1"]["transform"] = json!({
        "origin":{"x":"5","y":"8"}, "size":{"width":"4","height":"8"},
        "rotation":0, "flipHorizontal":false, "flipVertical":false
    });
    let q = serde_json::from_value(v).unwrap();
    let r = page_placements(&q, &|| false).unwrap();
    let p = &r.surfaces[0].objects[2];
    // Inner center (20,30) -> (40,30); its resolved axes map the
    // child's center offset (-3,-8) to (16,3).
    assert_eq!(p.affine.linear.map(Fixed::raw), [0, -2 * U, -U, 0]);
    assert_eq!(p.affine.translation.x.raw(), 56 * U);
    assert_eq!(p.affine.translation.y.raw(), 33 * U);
    assert_eq!(
        p.uncertainty.translation,
        Point {
            x: Fixed::ZERO,
            y: Fixed::ZERO
        }
    );
}
#[test]
fn equivalent_signed_and_multi_turn_angles_have_identical_placement() {
    for base in [0, 2_699_999, 2_700_000, 8_100_000, 13_500_000, 18_900_000] {
        let mut q = grouped();
        let id = mo_common::ObjectId::new("shape:1").unwrap();
        let mut expected = None;
        for turns in [-90, -1, 0, 1, 90] {
            q.document.objects.get_mut(&id).unwrap().transform.rotation = base + turns * 21_600_000;
            let r = page_placements(&q, &|| false).unwrap();
            let p = &r.surfaces[0].objects[1];
            let placement = serde_json::to_value(p).unwrap();
            if let Some(e) = &expected {
                assert_eq!(&placement, e);
            } else {
                expected = Some(placement);
            }
        }
    }
}
#[test]
fn cardinal_flips_negative_rotation_and_half_emu_centers_are_exact() {
    let mut q = request();
    let id = mo_common::ObjectId::new("shape:1").unwrap();
    let o = q.document.objects.get_mut(&id).unwrap();
    o.transform.rotation = -5400000;
    o.transform.flip_horizontal = true;
    o.transform.size.width = mo_common::Emu::new(21);
    let r = page_placements(&q, &|| false).unwrap();
    let p = &r.surfaces[0].objects[0];
    assert_eq!(p.affine.linear.map(Fixed::raw), [0, U, U, 0]);
    assert_eq!(p.anchor.x.raw(), 21 * U / 2);
    assert_eq!(p.uncertainty.translation.x.raw(), 0);
}
#[test]
fn diagonal_angle_bounds_enclose_the_exact_algebraic_value() {
    for angle in [2700000, 8100000, 13500000, 18900000, -2700000] {
        let pair = trig::cos_sin(angle, &|| false).unwrap();
        for x in pair {
            let (v, e) = x.q32().unwrap();
            let c = v.raw().abs();
            let e = e.raw();
            assert!(2 * (c - e) * (c - e) <= U * U && U * U <= 2 * (c + e) * (c + e));
            assert!(e <= 1);
        }
    }
}
#[test]
fn interval_signed_division_and_q32_ties_are_outward() {
    use interval::Interval;
    use num_bigint::BigInt;
    let third = Interval::ratio(-1, 3);
    let (v, e) = third.q32().unwrap();
    assert!(3 * (v.raw() - e.raw()) <= -U && -U <= 3 * (v.raw() + e.raw()));
    for s in [-1, 1] {
        let n = BigInt::from(s) << 63usize;
        let i = Interval::raw(n.clone(), n);
        let (v, e) = i.q32().unwrap();
        assert_eq!(v.raw(), s);
        assert_eq!(e.raw(), 1);
    }
}
#[test]
fn all_selected_placement_cancellation_points_return_no_partial_result() {
    let mut q = grouped();
    q.document
        .objects
        .get_mut(&mo_common::ObjectId::new("shape:1").unwrap())
        .unwrap()
        .transform
        .rotation = 1234567;
    let calls = Cell::new(0);
    page_placements(&q, &|| {
        calls.set(calls.get() + 1);
        false
    })
    .unwrap();
    for stop in 0..calls.get() {
        let count = Cell::new(0);
        assert!(matches!(
            page_placements(&q, &|| {
                let n = count.get();
                count.set(n + 1);
                n == stop
            }),
            Err(CompileError::Cancelled)
        ));
    }
}
#[test]
fn invalid_ownership_and_missing_slide_fail_before_output() {
    let mut q = grouped();
    q.slide = mo_common::SlideId::new("slide:absent").unwrap();
    assert!(matches!(
        page_placements(&q, &|| false),
        Err(CompileError::Invalid(_))
    ));
    q.slide = mo_common::SlideId::new("slide:1").unwrap();
    q.document
        .slides
        .get_mut(&q.slide)
        .unwrap()
        .objects
        .push(mo_common::ObjectId::new("shape:1").unwrap());
    assert!(matches!(
        page_placements(&q, &|| false),
        Err(CompileError::Document(_))
    ));
}
