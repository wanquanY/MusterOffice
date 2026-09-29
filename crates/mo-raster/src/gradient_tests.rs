use super::*;
use mo_geometry::{Fixed, PathCommand, Point};
use std::cell::Cell;
const U: i128 = 1 << 32;
fn p(x: i128, y: i128) -> Point {
    Point {
        x: Fixed::from_raw(x * U),
        y: Fixed::from_raw(y * U),
    }
}
pub(super) fn request() -> PathRasterRequest {
    PathRasterRequest {
        opacity_groups: vec![],
        clips: vec![],
        viewport: RasterViewport {
            width: 32,
            height: 32,
            origin: p(0, 0),
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
                PathCommand::Move { to: p(0, 0) },
                PathCommand::Line { to: p(32, 0) },
                PathCommand::Line { to: p(32, 32) },
                PathCommand::Close,
            ],
        }],
        draws: vec![PathDraw {
            blend: Default::default(),
            clip: None,
            path: 0,
            origin: p(0, 0),
            stroke: None,
            brush: Brush::Gradient {
                gradient: Gradient {
                    geometry: GradientGeometry::Linear {
                        start: p(0, 0),
                        end: p(32, 0),
                    },
                    stops: vec![
                        GradientStop {
                            position: 0.0,
                            srgb: [1.0, 0.0, 0.0, 1.0],
                        },
                        GradientStop {
                            position: 1.0,
                            srgb: [0.0, 0.0, 1.0, 1.0],
                        },
                    ]
                    .into(),
                    tile: GradientTile::Clamp,
                    interpolation: GradientInterpolation::Srgb,
                    alpha: GradientAlpha::Straight,
                },
            },
        }],
    }
}
pub(super) fn gradient(q: &mut PathRasterRequest) -> &mut Gradient {
    let Brush::Gradient { gradient } = &mut q.draws[0].brush else {
        unreachable!()
    };
    gradient
}
#[test]
fn gradient_interning_is_independent_of_path_placement() {
    let mut q = request();
    let mut draw = q.draws[0].clone();
    draw.origin = p(4, 0);
    q.draws.push(draw);
    q.draws.push(PathDraw {
        blend: Default::default(),
        clip: None,
        path: 0,
        origin: p(0, 0),
        brush: Brush::Solid { rgba: [20; 4] },
        stroke: None,
    });
    let c = compile(&q, &|| false).unwrap();
    assert_eq!(
        (
            c.work().gradients,
            c.work().gradient_stops,
            c.work().gradient_draws
        ),
        (1, 2, 2)
    );
    assert_eq!((c.frame()[1], c.frame()[9]), (4, 1));
    let draws = &c.frame()[c.frame().len() - 18..];
    assert_eq!([draws[5], draws[11], draws[17]], [1, 1, 0]);
    assert_eq!(draws[15], u32::from_le_bytes([20; 4]));
    assert_eq!(c.work().gradient_coordinate_error_bound, Fixed::ZERO);
    assert_eq!(c.work().gradient_value_error_bound, 0.0);
}
#[test]
fn brush_rebasing_keeps_large_world_origins_exact() {
    let mut q = request();
    let before = compile(&q, &|| false).unwrap();
    let huge = Fixed::from_raw(1 << 110);
    q.viewport.origin.x = huge;
    q.draws[0].origin.x = huge;
    let GradientGeometry::Linear { start, end } = &mut gradient(&mut q).geometry else {
        unreachable!()
    };
    start.x = start.x.checked_add(huge).unwrap();
    end.x = end.x.checked_add(huge).unwrap();
    assert_eq!(before.frame(), compile(&q, &|| false).unwrap().frame());
    let local = q.draws[0].brush.rebased(q.viewport.origin).unwrap();
    assert_eq!(local, request().draws[0].brush);
}
#[test]
fn hard_stops_keep_order_and_working_color_precision() {
    let mut q = request();
    let g = gradient(&mut q);
    g.stops = vec![
        GradientStop {
            position: 0.25,
            srgb: [0.25 / 255.0, -0.5, 1.5, 0.5],
        },
        GradientStop {
            position: 0.25,
            srgb: [1.0, 0.0, 0.0, 1.0],
        },
    ]
    .into();
    let c = compile(&q, &|| false).unwrap();
    let start = 10 + 2 + 4 * 7;
    let w = &c.frame()[start..];
    assert_eq!(w[9], w[14]);
    assert_eq!(f32::from_bits(w[10]), (0.25 / 255.0) as f32);
    assert_eq!((f32::from_bits(w[11]), f32::from_bits(w[12])), (-0.5, 1.5));
    assert!(c.work().gradient_value_error_bound > 0.0);
}
#[test]
fn invalid_or_collapsed_gradient_parameters_fail_before_backend() {
    type Mutation = Box<dyn Fn(&mut Gradient)>;
    let mutations: Vec<Mutation> = vec![
        Box::new(|g| g.stops = Default::default()),
        Box::new(|g| g.stops.make_mut()[0].position = -0.1),
        Box::new(|g| g.stops.make_mut()[1].position = f64::NAN),
        Box::new(|g| {
            g.stops.make_mut()[0].position = 0.75;
            g.stops.make_mut()[1].position = 0.5;
        }),
        Box::new(|g| g.stops.make_mut()[0].srgb[3] = 1.1),
        Box::new(|g| g.stops.make_mut()[0].srgb[0] = f64::INFINITY),
        Box::new(|g| {
            g.geometry = GradientGeometry::Radial {
                center: p(0, 0),
                radius: Fixed::ZERO,
            }
        }),
        Box::new(|g| {
            g.geometry = GradientGeometry::Linear {
                start: p(0, 0),
                end: p(0, 0),
            }
        }),
        Box::new(|g| {
            g.stops.make_mut()[0].position = 0.5;
            g.stops.make_mut()[1].position = 0.5 + 1e-10;
        }),
        Box::new(|g| g.stops.make_mut()[1].position = 1.0 - 1e-10),
    ];
    struct Never;
    impl RasterBackend for Never {
        fn raster(&mut self, _: &[u32]) -> Result<BackendReply, RasterError> {
            panic!("invalid brush reached backend")
        }
        fn invalidate(&mut self) {
            panic!("invalid input poisoned backend")
        }
    }
    for change in mutations {
        let mut q = request();
        change(gradient(&mut q));
        assert!(render(&q, &mut Never, &|| false).is_err());
    }
}
#[test]
fn gradient_resources_and_input_work_are_bounded() {
    let mut q = request();
    let repeated = q.draws[0].clone();
    q.draws = vec![repeated; 65536];
    let c = compile(&q, &|| false).unwrap();
    assert_eq!(c.work().gradients, 1);
    assert!(c.frame().len() <= MAX_FRAME_WORDS);
    for draw in &mut q.draws {
        let Brush::Gradient { gradient: g } = &mut draw.brush else {
            unreachable!()
        };
        g.stops = vec![g.stops[0].clone(); 5].into();
    }
    assert!(matches!(
        compile(&q, &|| false),
        Err(RasterError::Limit("gradient input stops"))
    ));
}
#[test]
fn cancellation_during_gradient_compilation_is_atomic() {
    let q = request();
    let ticks = Cell::new(0);
    compile(&q, &|| {
        ticks.set(ticks.get() + 1);
        false
    })
    .unwrap();
    for stop in 1..=ticks.get() {
        let at = Cell::new(0);
        assert!(matches!(
            compile(&q, &|| {
                at.set(at.get() + 1);
                at.get() == stop
            }),
            Err(RasterError::Cancelled)
        ));
    }
}
#[test]
fn every_interpolation_and_geometry_has_an_explicit_wire_value() {
    for (kind, geometry) in [
        GradientGeometry::Linear {
            start: p(0, 0),
            end: p(32, 0),
        },
        GradientGeometry::Radial {
            center: p(10, 10),
            radius: Fixed::from_raw(20 * U),
        },
    ]
    .into_iter()
    .enumerate()
    {
        for (tile, t) in [
            GradientTile::Clamp,
            GradientTile::Repeat,
            GradientTile::Mirror,
            GradientTile::Decal,
        ]
        .into_iter()
        .enumerate()
        {
            for (space, s) in [
                GradientInterpolation::Srgb,
                GradientInterpolation::LinearSrgb,
            ]
            .into_iter()
            .enumerate()
            {
                for (alpha, a) in [GradientAlpha::Straight, GradientAlpha::Premultiplied]
                    .into_iter()
                    .enumerate()
                {
                    let mut q = request();
                    let g = gradient(&mut q);
                    g.geometry = geometry.clone();
                    g.tile = t;
                    g.interpolation = s;
                    g.alpha = a;
                    let c = compile(&q, &|| false).unwrap();
                    let start = 10 + 2 + 4 * 7;
                    assert_eq!(
                        &c.frame()[start..start + 5],
                        &[kind as u32, tile as u32, space as u32, alpha as u32, 2]
                    );
                }
            }
        }
    }
}
