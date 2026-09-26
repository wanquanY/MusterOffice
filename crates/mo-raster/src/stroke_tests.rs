use super::*;
use mo_geometry::{Fixed, PathCommand, Point};

const U: i128 = 1 << 32;
#[test]
fn joins_reject_unknown_or_variant_specific_fields() {
    for input in [
        r#"{"kind":"round","limit":"0"}"#,
        r#"{"kind":"bevel","unused":false}"#,
        r#"{"kind":"miter","limit":"0","unused":false}"#,
        r#"{"kind":"miterClip","limit":"4294967296","unused":false}"#,
        r#"{"kind":"miterClip"}"#,
    ] {
        assert!(serde_json::from_str::<StrokeJoin>(input).is_err());
    }
}

#[test]
fn clipped_miter_is_distinct_and_limits_its_full_corner_extent() {
    let mut q = request();
    q.draws[0].stroke.as_mut().unwrap().join = StrokeJoin::MiterClip {
        limit: Fixed::from_raw(U),
    };
    let c = compile(&q, &|| false).unwrap();
    assert_eq!(c.frame()[10 + 2 + 2 * 7 + 2], 3);
    for limit in [-1, 0, U - 1, 1024 * U + 1] {
        q.draws[0].stroke.as_mut().unwrap().join = StrokeJoin::MiterClip {
            limit: Fixed::from_raw(limit),
        };
        assert!(matches!(
            compile(&q, &|| false),
            Err(RasterError::Invalid(_))
        ));
    }
    q.draws[0].stroke.as_mut().unwrap().join = StrokeJoin::MiterClip {
        limit: Fixed::from_raw(1024 * U),
    };
    // Even an explicit one-pixel hairline has a long clipped corner.
    q.draws[0].stroke.as_mut().unwrap().width = Fixed::from_raw(0);
    q.draws[0].origin = point(32500, 0);
    assert!(matches!(compile(&q, &|| false), Err(RasterError::Range)));
    q.draws[0].origin = point(0, 0);
    assert!(compile(&q, &|| false).is_ok());
}
fn point(x: i128, y: i128) -> Point {
    Point {
        x: Fixed::from_raw(x * U),
        y: Fixed::from_raw(y * U),
    }
}
fn request() -> PathRasterRequest {
    PathRasterRequest {
        clips: vec![],
        viewport: RasterViewport {
            width: 64,
            height: 64,
            origin: point(0, 0),
            scale: PixelScale {
                numerator: 1,
                denominator: 1,
            },
            coordinate_tolerance: Fixed::from_raw(1 << 24),
            background: [0; 4],
        },
        paths: vec![FillPath {
            fill_rule: FillRule::Nonzero,
            commands: vec![
                PathCommand::Move { to: point(10, 20) },
                PathCommand::Line { to: point(40, 20) },
            ],
        }],
        draws: vec![PathDraw {
            blend: Default::default(),
            clip: None,
            path: 0,
            origin: point(0, 0),
            brush: Brush::Solid {
                rgba: [255, 0, 0, 255],
            },
            stroke: Some(StrokeStyle {
                width: Fixed::from_raw(4 * U),
                cap: StrokeCap::Butt,
                join: StrokeJoin::Round {},
            }),
        }],
    }
}

#[test]
fn paint_interning_preserves_fill_order_and_open_contours() {
    let mut q = request();
    let mut fill = q.draws[0].clone();
    fill.stroke = None;
    q.draws.push(fill);
    let mut equivalent = q.draws[0].clone();
    equivalent.stroke.as_mut().unwrap().width = Fixed::from_raw(4 * U + 1);
    q.draws.push(equivalent);
    let c = compile(&q, &|| false).unwrap();
    assert_eq!(&c.frame()[..2], &[0x4d4f534b, 4]);
    assert_eq!(c.work().stroke_styles, 1);
    assert_eq!(c.work().stroke_draws, 2);
    assert_eq!(c.work().commands, 2); // No implicit close is introduced.
    assert_eq!(c.work().stroke_width_error_bound.raw(), 1);
    let start = 10 + 2 + 2 * 7;
    assert_eq!(&c.frame()[start..start + 4], &[4f32.to_bits(), 0, 1, 0]);
    assert_eq!(c.frame()[start + 8], 1);
    assert_eq!(c.frame()[start + 14], 0);
    assert_eq!(c.frame()[start + 20], 1);
}

#[test]
fn width_uses_exact_viewport_scale_and_miter_is_dimensionless() {
    let mut q = request();
    q.viewport.scale = PixelScale {
        numerator: 1,
        denominator: u32::MAX,
    };
    let style = q.draws[0].stroke.as_mut().unwrap();
    style.width = Fixed::from_raw((U + 256) * i128::from(u32::MAX) + 1);
    style.join = StrokeJoin::Miter {
        limit: Fixed::from_raw(4 * U + 1),
    };
    let c = compile(&q, &|| false).unwrap();
    let start = 10 + 2 + 2 * 7;
    assert_eq!(c.frame()[start], 0x3f800001); // f64-mediated conversion rounds down incorrectly.
    assert_eq!(c.frame()[start + 3], 4f32.to_bits());
    assert_eq!(c.work().miter_limit_error_bound.raw(), 1);
    assert!(c.work().stroke_width_error_bound.raw() > 0);
    q.viewport.coordinate_tolerance = Fixed::from_raw(256);
    q.viewport.scale = PixelScale {
        numerator: 1,
        denominator: 3,
    };
    q.draws[0].stroke.as_mut().unwrap().width = Fixed::from_raw(1000 * U);
    assert!(matches!(
        compile(&q, &|| false),
        Err(RasterError::Precision)
    ));
}

#[test]
fn invalid_parameters_and_stroke_expansion_are_rejected() {
    for width in [-1, 32768 * U + 1, i128::MAX] {
        let mut q = request();
        q.draws[0].stroke.as_mut().unwrap().width = Fixed::from_raw(width);
        assert!(compile(&q, &|| false).is_err());
    }
    for limit in [-1, 1024 * U + 1, i128::MAX] {
        let mut q = request();
        q.draws[0].stroke.as_mut().unwrap().join = StrokeJoin::Miter {
            limit: Fixed::from_raw(limit),
        };
        assert!(matches!(
            compile(&q, &|| false),
            Err(RasterError::Invalid(_))
        ));
    }
    for width in [0, 2 * U] {
        let mut q = request();
        q.draws[0].stroke.as_mut().unwrap().width = Fixed::from_raw(width);
        q.draws[0].origin = point(32728, 0);
        assert!(matches!(compile(&q, &|| false), Err(RasterError::Range)));
    }
    let mut q = request();
    q.draws[0].stroke.as_mut().unwrap().join = StrokeJoin::Miter {
        limit: Fixed::from_raw(1024 * U),
    };
    q.draws[0].origin = point(30700, 0);
    assert!(matches!(compile(&q, &|| false), Err(RasterError::Range)));
}

#[test]
fn style_budget_counts_distinct_evaluated_parameters() {
    let mut q = request();
    q.paths[0].commands.clear(); // Style validation is independent of visible geometry.
    q.draws = (0..4096)
        .map(|i| {
            let mut draw = q.draws[0].clone();
            draw.stroke.as_mut().unwrap().width = Fixed::from_raw(i * U);
            draw
        })
        .collect();
    assert_eq!(compile(&q, &|| false).unwrap().work().stroke_styles, 4096);
    let mut extra = q.draws[0].clone();
    extra.stroke.as_mut().unwrap().cap = StrokeCap::Square;
    q.draws.push(extra);
    assert!(matches!(
        compile(&q, &|| false),
        Err(RasterError::Limit("stroke styles"))
    ));
}
