use mo_common::Emu;
use mo_geometry::{Fixed, PathCommand as C, Point};
use mo_raster::{picking::*, *};
use mo_skia_sys::NativeRaster;
fn f(n: i64) -> Fixed {
    Fixed::emu(Emu::new(n))
}
fn p(x: i64, y: i64) -> Point {
    Point { x: f(x), y: f(y) }
}
fn square(a: i64, b: i64) -> Vec<C> {
    vec![
        C::Move { to: p(a, a) },
        C::Line { to: p(b, a) },
        C::Line { to: p(b, b) },
        C::Line { to: p(a, b) },
        C::Close,
    ]
}
fn scene(paths: Vec<FillPath>, draws: Vec<PathDraw>, clips: Vec<PathClip>) -> PathRasterRequest {
    PathRasterRequest {
        opacity_groups: vec![],
        viewport: RasterViewport {
            width: 100,
            height: 100,
            origin: p(0, 0),
            scale: PixelScale {
                numerator: 1,
                denominator: 1,
            },
            coordinate_tolerance: Fixed::from_raw(1 << 24),
            background: [0; 4],
        },
        paths,
        draws,
        clips,
    }
}
fn draw(path: u32, stroke: Option<StrokeStyle>, clip: Option<u32>) -> PathDraw {
    PathDraw {
        blend: Default::default(),
        clip,
        path,
        origin: p(0, 0),
        brush: Brush::Solid {
            rgba: [100, 50, 30, 255],
        },
        stroke,
    }
}
#[derive(Default)]
struct Backend {
    native: NativeRaster,
    frame: Vec<u32>,
    reply: Vec<u32>,
}
impl RasterBackend for Backend {
    fn raster(&mut self, frame: &[u32]) -> Result<BackendReply, RasterError> {
        self.native.raster(frame)
    }
    fn pick(&mut self, frame: &[u32]) -> Result<PickingReply, RasterError> {
        let r = self.native.pick(frame)?;
        self.frame = frame.to_vec();
        self.reply = r.words.clone();
        Ok(r)
    }
    fn invalidate(&mut self) {
        self.native.invalidate()
    }
}
fn run(label: &str, s: &PathRasterRequest, queries: &[DevicePickQuery]) -> Vec<Vec<DrawHit>> {
    let compiled = compile(s, &|| false).unwrap();
    let pick = compiled.picking(&|| false).unwrap();
    assert_eq!(pick.draw_count(), s.draws.len() as u32);
    let mut b = Backend::default();
    let result = pick.query(queries, &mut b, &|| false).unwrap();
    if let Some(out) = std::env::var_os("MO_PICK_FIXTURES") {
        let raster_frame = compiled.frame().to_vec();
        let pixels = render_compiled(compiled, &mut NativeRaster, &|| false)
            .unwrap()
            .pixels;
        let out = std::path::PathBuf::from(out);
        std::fs::create_dir_all(&out).unwrap();
        std::fs::write(
            out.join(format!("{label}.json")),
            serde_json::to_vec(&serde_json::json!({"frame":b.frame,"reply":b.reply,"rasterFrame":raster_frame,"pixels":pixels})).unwrap(),
        )
        .unwrap();
    }
    result.iter().map(|r| r.hits().collect()).collect()
}
fn q(x: i64, y: i64, r: i64) -> DevicePickQuery {
    DevicePickQuery {
        point: p(x, y),
        radius: f(r),
    }
}
fn exact(draw: u32) -> DrawHit {
    DrawHit {
        draw,
        kind: DrawHitKind::Exact,
    }
}
fn near(draw: u32) -> DrawHit {
    DrawHit {
        draw,
        kind: DrawHitKind::Nearby,
    }
}
#[test]
fn holes_paint_order_clips_and_pointer_tolerance_use_actual_paths() {
    let paths = vec![
        FillPath {
            fill_rule: FillRule::Nonzero,
            commands: square(10, 90),
        },
        FillPath {
            fill_rule: FillRule::Evenodd,
            commands: [square(20, 80), square(40, 60)].concat(),
        },
        FillPath {
            fill_rule: FillRule::Nonzero,
            commands: square(25, 75),
        },
    ];
    let s = scene(
        paths,
        vec![draw(0, None, None), draw(1, None, Some(0))],
        vec![PathClip {
            path: 2,
            parent: None,
            origin: p(0, 0),
        }],
    );
    assert_eq!(
        run(
            "fill-hole-clip",
            &s,
            &[
                q(30, 30, 0),
                q(50, 50, 0),
                q(21, 21, 8),
                q(41, 50, 2),
                q(8, 50, 3),
                q(100, 50, 32)
            ]
        ),
        vec![
            vec![exact(1), exact(0)],
            vec![exact(0)],
            vec![exact(0)],
            vec![near(1), exact(0)],
            vec![near(0)],
            vec![]
        ]
    );
    let mut reflected = s.clone();
    for d in &mut reflected.draws {
        d.origin = p(-5, 0);
    }
    assert_eq!(
        run(
            "independent-clip-origin",
            &reflected,
            &[q(25, 30, 0), q(24, 30, 4)]
        ),
        vec![vec![exact(1), exact(0)], vec![exact(0)]]
    );
}
#[test]
fn caps_joins_hairlines_and_cubic_outline_match_stroke_geometry() {
    for cap in [StrokeCap::Butt, StrokeCap::Round, StrokeCap::Square] {
        let s = scene(
            vec![FillPath {
                fill_rule: FillRule::Nonzero,
                commands: vec![C::Move { to: p(20, 50) }, C::Line { to: p(80, 50) }],
            }],
            vec![draw(
                0,
                Some(StrokeStyle {
                    width: f(10),
                    cap,
                    join: StrokeJoin::Round {},
                }),
                None,
            )],
            vec![],
        );
        let r = run(
            &format!("cap-{cap:?}"),
            &s,
            &[q(17, 50, 0), q(16, 46, 0), q(50, 54, 0), q(50, 57, 3)],
        );
        assert_eq!(
            r[0],
            if cap == StrokeCap::Butt {
                vec![]
            } else {
                vec![exact(0)]
            }
        );
        assert_eq!(
            r[1],
            if cap == StrokeCap::Square {
                vec![exact(0)]
            } else {
                vec![]
            }
        );
        assert_eq!(r[2], vec![exact(0)]);
        assert_eq!(r[3], vec![near(0)]);
    }
    for join in [
        StrokeJoin::Miter { limit: f(4) },
        StrokeJoin::MiterClip { limit: f(1) },
        StrokeJoin::Bevel {},
        StrokeJoin::Round {},
    ] {
        let s = scene(
            vec![FillPath {
                fill_rule: FillRule::Nonzero,
                commands: vec![
                    C::Move { to: p(20, 80) },
                    C::Line { to: p(50, 20) },
                    C::Line { to: p(80, 80) },
                ],
            }],
            vec![draw(
                0,
                Some(StrokeStyle {
                    width: f(10),
                    cap: StrokeCap::Butt,
                    join,
                }),
                None,
            )],
            vec![],
        );
        let r = run(
            &format!("join-{join:?}"),
            &s,
            &[q(50, 12, 0), q(50, 16, 0), q(50, 50, 0)],
        );
        assert_eq!(
            r[0],
            if matches!(join, StrokeJoin::Miter { .. }) {
                vec![exact(0)]
            } else {
                vec![]
            }
        );
        assert_eq!(r[2], vec![]);
        let image = render(&s, &mut NativeRaster, &|| false).unwrap();
        for (i, (x, y)) in [(50, 12), (50, 16), (50, 50)].into_iter().enumerate() {
            // These samples lie away from contour edges; their alpha establishes
            // agreement with the original renderer's custom/ordinary joins.
            assert_eq!(image.pixels[(y * 100 + x) * 4 + 3] > 127, !r[i].is_empty());
        }
    }
    let s = scene(
        vec![FillPath {
            fill_rule: FillRule::Nonzero,
            commands: vec![
                C::Move { to: p(20, 50) },
                C::Cubic {
                    control1: p(20, 10),
                    control2: p(80, 10),
                    to: p(80, 50),
                },
            ],
        }],
        vec![draw(
            0,
            Some(StrokeStyle {
                width: f(0),
                cap: StrokeCap::Butt,
                join: StrokeJoin::Round {},
            }),
            None,
        )],
        vec![],
    );
    assert_eq!(
        run(
            "cubic-hairline",
            &s,
            &[q(50, 20, 0), q(50, 18, 3), q(50, 50, 0)]
        ),
        vec![vec![exact(0)], vec![near(0)], vec![]]
    );
}
#[test]
fn invalid_abi_shapes_and_work_are_rejected_without_poisoning_component() {
    let s = scene(
        vec![FillPath {
            fill_rule: FillRule::Nonzero,
            commands: square(10, 90),
        }],
        vec![draw(0, None, None)],
        vec![],
    );
    let compiled = compile(&s, &|| false).unwrap().picking(&|| false).unwrap();
    let mut b = Backend::default();
    compiled.query(&[q(50, 50, 0)], &mut b, &|| false).unwrap();
    let frame = b.frame.clone();
    for end in 0..frame.len() {
        let r = b.native.pick(&frame[..end]).unwrap();
        assert_ne!(r.status, 0);
        assert!(r.words.is_empty());
    }
    for index in [0, 1, 4, 5, 6, 7, 8, 9, 10, 11] {
        let mut bad = frame.clone();
        bad[index] = u32::MAX;
        let r = b.native.pick(&bad).unwrap();
        assert_ne!(r.status, 0);
        assert!(r.words.is_empty());
    }
    let mut bad = frame.clone();
    bad.push(0);
    assert_ne!(b.native.pick(&bad).unwrap().status, 0);
    assert!(!b.native.is_invalid());
    assert_eq!(b.native.pick(&frame).unwrap().status, 0);
}

#[test]
fn fill_pointer_radius_includes_the_implicit_closing_edge() {
    let s = scene(
        vec![FillPath {
            fill_rule: FillRule::Nonzero,
            commands: vec![
                C::Move { to: p(20, 20) },
                C::Line { to: p(80, 20) },
                C::Line { to: p(80, 80) },
            ],
        }],
        vec![draw(0, None, None)],
        vec![],
    );
    assert_eq!(
        run(
            "implicit-fill-close",
            &s,
            &[q(60, 30, 0), q(49, 51, 3), q(40, 60, 3)]
        ),
        vec![vec![exact(0)], vec![near(0)], vec![]]
    );
}

#[test]
fn many_draws_transparent_paint_and_huge_origins_preserve_geometry_identity() {
    let mut s = scene(
        vec![FillPath {
            fill_rule: FillRule::Nonzero,
            commands: square(10, 50),
        }],
        vec![draw(0, None, None); 65],
        vec![],
    );
    s.viewport.origin = Point {
        x: Fixed::from_raw(1 << 112),
        y: Fixed::from_raw(-(1 << 112)),
    };
    s.viewport.scale = PixelScale {
        numerator: 3,
        denominator: 2,
    };
    for d in &mut s.draws {
        d.origin = s.viewport.origin;
        d.brush = Brush::Solid { rgba: [0; 4] };
    }
    let queries: Vec<_> = (0..64).map(|x| q(x + 5, 40, 0)).collect();
    let r = run("many-transparent-huge-origin", &s, &queries);
    for (i, hits) in r.into_iter().enumerate() {
        let x = i + 5;
        assert_eq!(
            hits,
            if x >= 15 {
                (0..65).rev().map(exact).collect()
            } else {
                vec![]
            }
        );
    }
}

#[test]
fn work_amplification_is_rejected_before_geometry_and_does_not_poison() {
    let command = |op, x: f32, y: f32| [op, x.to_bits(), y.to_bits(), 0, 0, 0, 0];
    let mut frame = vec![PICK_MAGIC, 1, 100, 100, 1, 5, 0, 0, 65536, 64, 0, 5];
    for c in [
        command(1, 10., 10.),
        command(2, 90., 10.),
        command(2, 90., 90.),
        command(2, 10., 90.),
        command(5, 0., 0.),
    ] {
        frame.extend(c);
    }
    for _ in 0..65536 {
        frame.extend([0; 5]);
    }
    for _ in 0..64 {
        frame.extend([50f32.to_bits(), 50f32.to_bits(), 0]);
    }
    let r = NativeRaster.pick(&frame).unwrap();
    assert_eq!(r.status, 3);
    assert!(r.words.is_empty());
    assert!(!NativeRaster.is_invalid());
    if let Some(out) = std::env::var_os("MO_PICK_FIXTURES") {
        let out = std::path::PathBuf::from(out);
        std::fs::create_dir_all(&out).unwrap();
        std::fs::write(
            out.join("work-limit.json"),
            serde_json::to_vec(&serde_json::json!({"frame":frame,"status":3,"reply":[]})).unwrap(),
        )
        .unwrap();
    }
}
