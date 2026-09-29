use super::*;
use mo_geometry::Rect;
fn bounds() -> Rect {
    Rect {
        min: Point {
            x: raw(300),
            y: raw(-50),
        },
        max: Point {
            x: raw(1300),
            y: raw(750),
        },
    }
}
#[test]
fn native_receiver_uses_shared_crop_and_tile_math_without_integer_truncation() {
    let mut f = fill();
    f.dpi = value(96);
    f.source_rect = rect(["25%", "12500", "25000", "12500"]);
    for mode in [f.mode.clone(), tile(Alignment::Center, NativeTileFlip::Xy)] {
        f.mode = mode;
        let base = layout(&f, &image(), size(), &|| false).unwrap();
        let mut r = bounds();
        // A fractional receiver translation must survive; it is not an EMU Size.
        for x in [&mut r.min.x, &mut r.max.x] {
            *x = x.checked_add(Fixed::from_raw(1 << 30)).unwrap();
        }
        let moved = layout_region(&f, &image(), r, Fixed::ZERO, &|| false).unwrap();
        assert_eq!(moved.origin.x, base.origin.x.checked_add(r.min.x).unwrap());
        assert_eq!(moved.origin.y, base.origin.y.checked_add(r.min.y).unwrap());
        assert_eq!(moved.pixel_step, base.pixel_step);
        assert_eq!(
            moved.fill_rectangle.right,
            base.fill_rectangle.right.checked_add(r.min.x).unwrap()
        );
        assert_eq!(
            serde_json::to_value(&moved.uncertainty).unwrap(),
            serde_json::to_value(&base.uncertainty).unwrap()
        );
        assert_eq!(moved.tile_x, base.tile_x);
        assert_eq!(moved.tile_y, base.tile_y);
    }
}
#[test]
fn receiver_uncertainty_encloses_corner_extents_and_cancellation_never_returns_a_plan() {
    let f = fill();
    let e = Fixed::from_raw(1 << 20);
    let r = bounds();
    let outer = layout_region(&f, &image(), r, e, &|| false).unwrap();
    for signs in 0..16 {
        let mut perturbed = r;
        for (i, v) in [
            &mut perturbed.min.x,
            &mut perturbed.min.y,
            &mut perturbed.max.x,
            &mut perturbed.max.y,
        ]
        .into_iter()
        .enumerate()
        {
            *v = if signs & (1 << i) == 0 {
                v.checked_sub(e)
            } else {
                v.checked_add(e)
            }
            .unwrap();
        }
        // Compare to the exact rational rectangle, not another rounded plan's
        // symmetric enclosure (which can extend beyond the true corner value).
        for (center, error, numerator, denominator) in [
            (
                outer.origin.x,
                outer.uncertainty.origin.x,
                perturbed.min.x.raw(),
                1,
            ),
            (
                outer.origin.y,
                outer.uncertainty.origin.y,
                perturbed.min.y.raw(),
                1,
            ),
            (
                outer.pixel_step.x,
                outer.uncertainty.pixel_step.x,
                perturbed.max.x.raw() - perturbed.min.x.raw(),
                100,
            ),
            (
                outer.pixel_step.y,
                outer.uncertainty.pixel_step.y,
                perturbed.max.y.raw() - perturbed.min.y.raw(),
                80,
            ),
        ] {
            assert!((center.raw() * denominator - numerator).abs() <= error.raw() * denominator);
        }
    }
    assert!(layout_region(&f, &image(), r, Fixed::from_raw(-1), &|| false).is_err());
    assert!(layout_region(&f, &image(), r, raw(1000), &|| false).is_err());
    let calls = Cell::new(0);
    layout_region(&f, &image(), r, e, &|| {
        calls.set(calls.get() + 1);
        false
    })
    .unwrap();
    for stop in 1..=calls.get() {
        let n = Cell::new(0);
        assert!(matches!(
            layout_region(&f, &image(), r, e, &|| {
                n.set(n.get() + 1);
                n.get() == stop
            }),
            Err(ImageLayoutError::Cancelled)
        ));
    }
}
