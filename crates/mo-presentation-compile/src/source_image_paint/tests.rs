use super::*;
use crate::source_placement::*;
use mo_common::Emu;
use mo_presentation_model::{Point as NativePoint, Size};
use mo_presentation_source::source::SourceObjectRef;
use mo_raster::ImageTile;
use std::cell::Cell;
fn f(v: i64) -> Fixed {
    Fixed::emu(Emu::new(v))
}
fn p(x: i64, y: i64) -> Point {
    Point { x: f(x), y: f(y) }
}
fn native(x: i64, y: i64) -> NativePoint {
    NativePoint {
        x: Emu::new(x),
        y: Emu::new(y),
    }
}
fn rect(l: i64, t: i64, r: i64, b: i64) -> ImageLayoutRectangle {
    ImageLayoutRectangle {
        left: f(l),
        top: f(t),
        right: f(r),
        bottom: f(b),
    }
}
fn value<T>(value: T) -> TransformValue<T> {
    TransformValue {
        value,
        source: TransformValueSource::Declaration {
            object: SourceObjectRef {
                part: "/ppt/slides/slide1.xml".into(),
                native_id: 2,
            },
        },
    }
}
fn plan() -> ImageSourceLayoutPlan {
    let size = Size {
        width: Emu::new(100),
        height: Emu::new(80),
    };
    ImageSourceLayoutPlan {
        target: FillTarget::Picture { native_id: 2 },
        resource: 0,
        placement: Some(NativePlacement {
            transform: ResolvedNativeTransform {
                origin: value(native(0, 0)),
                size: value(size),
                rotation: value(0),
                flip_horizontal: value(false),
                flip_vertical: value(false),
                child_origin: None,
                child_size: None,
                graphic_frame_orientation_ignored: false,
            },
            source_size: size,
            source_origin: native(-100, 200),
            anchor: p(-50, 240),
            affine: Affine {
                linear: [f(0), f(-2), f(3), f(0)],
                translation: p(500, 600),
            },
            uncertainty: crate::AffineUncertainty {
                linear: [Fixed::ZERO; 4],
                translation: ZERO,
            },
        }),
        layout: NativeImageLayout {
            profile: crate::source_image_layout::PROFILE.into(),
            source_rectangle: rect(0, 0, 2, 2),
            fill_rectangle: rect(10, 20, 30, 40),
            origin: p(10, 20),
            pixel_step: p(10, 10),
            tile_x: ImageTile::Clamp,
            tile_y: ImageTile::Clamp,
            clip_to_fill_rectangle: true,
            rotate_with_shape: true,
            density: ImageLayoutDensity::NotRequired,
            uncertainty: ImageLayoutUncertainty {
                source_rectangle: [Fixed::ZERO; 4],
                fill_rectangle: [Fixed::ZERO; 4],
                origin: ZERO,
                pixel_step: ZERO,
            },
        },
    }
}
#[test]
fn world_image_and_fill_clip_use_one_native_basis_and_group_source_origin() {
    let mut q = plan();
    let before = serde_json::to_value(&q).unwrap();
    let r = compile(&q, ImageSampling::Linear, &|| false).unwrap();
    assert_eq!(serde_json::to_value(&q).unwrap(), before);
    assert_eq!(
        (r.brush.origin, r.brush.x_step, r.brush.y_step),
        (p(540, 480), p(0, 30), p(-20, 0))
    );
    assert!(r.brush.uncertainty.is_none());
    let clip = r.fill_clip.unwrap();
    assert_eq!(
        clip.path.commands,
        vec![
            C::Move { to: p(-40, -20) },
            C::Line { to: p(-20, -20) },
            C::Line { to: p(-20, 0) },
            C::Line { to: p(-40, 0) },
            C::Close
        ]
    );
    assert_eq!(clip.affine, q.placement.as_ref().unwrap().affine);
    assert_eq!(clip.upstream_error, ZERO);
    // Changing chOff and anchor together leaves the effective viewport fixed.
    let placement = q.placement.as_mut().unwrap();
    placement.source_origin = native(0, 0);
    placement.anchor = p(50, 40);
    let b = compile(&q, ImageSampling::Linear, &|| false).unwrap();
    assert_eq!(r.brush, b.brush);
    let shift = Fixed::from_raw(1i128 << 112);
    q.placement.as_mut().unwrap().affine.translation.x = shift.checked_add(f(500)).unwrap();
    let b = compile(&q, ImageSampling::Linear, &|| false).unwrap();
    assert_eq!(
        b.brush.origin.x.checked_sub(shift).unwrap(),
        r.brush.origin.x
    );
}
#[test]
fn world_paint_and_clip_preserve_distinct_upstream_errors() {
    let mut q = plan();
    let placement = q.placement.as_mut().unwrap();
    placement.affine = Affine::IDENTITY;
    placement.uncertainty.translation = Point {
        x: Fixed::from_raw(1),
        y: Fixed::from_raw(2),
    };
    let e = &mut q.layout.uncertainty;
    e.origin = Point {
        x: Fixed::from_raw(4),
        y: Fixed::from_raw(8),
    };
    e.pixel_step = Point {
        x: Fixed::from_raw(9),
        y: Fixed::from_raw(3),
    };
    e.fill_rectangle = [1, 2, 3, 4].map(Fixed::from_raw);
    e.source_rectangle = [2, 4, 6, 8].map(Fixed::from_raw);
    let source_error = e.source_rectangle;
    let r = compile(&q, ImageSampling::Nearest, &|| false).unwrap();
    let error = r.brush.uncertainty.unwrap();
    assert_eq!(
        error.origin,
        Point {
            x: Fixed::from_raw(5),
            y: Fixed::from_raw(10)
        }
    );
    assert_eq!(
        error.x_step,
        Point {
            x: Fixed::from_raw(9),
            y: Fixed::ZERO
        }
    );
    assert_eq!(
        error.y_step,
        Point {
            x: Fixed::ZERO,
            y: Fixed::from_raw(3)
        }
    );
    assert_eq!(error.source_domain, source_error);
    assert_eq!(
        r.fill_clip.unwrap().upstream_error,
        Point {
            x: Fixed::from_raw(4),
            y: Fixed::from_raw(6)
        }
    );
}
#[test]
fn stationary_shape_images_require_policy_but_backgrounds_have_no_shape_rotation() {
    let mut q = plan();
    q.layout.rotate_with_shape = false;
    assert!(matches!(
        compile(&q, ImageSampling::Linear, &|| false),
        Err(ImagePaintError::OrientationRequired)
    ));
    q.target = FillTarget::Background {};
    q.placement = None;
    let b = compile(&q, ImageSampling::Linear, &|| false).unwrap();
    assert_eq!(b.brush.origin, p(10, 20));
    q.layout.tile_x = ImageTile::Mirror;
    q.layout.tile_y = ImageTile::Repeat;
    q.layout.clip_to_fill_rectangle = false;
    let b = compile(&q, ImageSampling::Linear, &|| false).unwrap();
    assert!(b.fill_clip.is_none());
    assert_eq!(
        (b.brush.tile_x, b.brush.tile_y),
        (ImageTile::Mirror, ImageTile::Repeat)
    );
    q.target = FillTarget::Picture { native_id: 2 };
    assert!(matches!(
        compile(&q, ImageSampling::Linear, &|| false),
        Err(ImagePaintError::Invalid(_))
    ));
}
#[test]
fn world_image_compilation_is_atomic_for_invalid_numbers_and_every_cancel_checkpoint() {
    let q = plan();
    let n = Cell::new(0);
    compile(&q, ImageSampling::Linear, &|| {
        n.set(n.get() + 1);
        false
    })
    .unwrap();
    for stop in 1..=n.get() {
        let n = Cell::new(0);
        assert!(matches!(
            compile(&q, ImageSampling::Linear, &|| {
                n.set(n.get() + 1);
                n.get() == stop
            }),
            Err(ImagePaintError::Cancelled)
        ));
    }
    let mut bad = q.clone();
    bad.layout.uncertainty.pixel_step.x = Fixed::from_raw(-1);
    assert!(matches!(
        compile(&bad, ImageSampling::Linear, &|| false),
        Err(ImagePaintError::Invalid(_))
    ));
    bad = q.clone();
    bad.layout.uncertainty.source_rectangle[0] = f(2);
    assert!(matches!(
        compile(&bad, ImageSampling::Linear, &|| false),
        Err(ImagePaintError::Precision)
    ));
    bad = q.clone();
    bad.placement.as_mut().unwrap().affine.translation.x = Fixed::from_raw(i128::MAX);
    assert!(matches!(
        compile(&bad, ImageSampling::Linear, &|| false),
        Err(ImagePaintError::Range)
    ));
}
