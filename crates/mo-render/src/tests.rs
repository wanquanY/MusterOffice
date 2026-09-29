use super::*;
use mo_geometry::{Affine, Fixed, PathCommand, Point};
use mo_raster::{BackendReply, FillPath, FillRule, PixelScale, RasterViewport};
use std::cell::Cell;
const U: i128 = 1 << 32;
fn p(x: i128, y: i128) -> Point {
    Point {
        x: Fixed::from_raw(x),
        y: Fixed::from_raw(y),
    }
}
fn node(parent: Option<u32>, a: [i128; 4], t: Point) -> TransformNode {
    TransformNode {
        parent,
        affine: Affine {
            linear: a.map(Fixed::from_raw),
            translation: t,
        },
    }
}
pub(super) fn request() -> SceneRasterRequest {
    SceneRasterRequest {
        viewport: RasterViewport {
            width: 64,
            height: 64,
            origin: p(0, 0),
            scale: PixelScale {
                numerator: 1,
                denominator: 1,
            },
            coordinate_tolerance: Fixed::from_raw(1 << 24),
            background: [0; 4],
        },
        scene: DrawScene {
            opacity_groups: vec![],
            clips: vec![],
            paths: vec![FillPath {
                fill_rule: FillRule::Nonzero,
                commands: vec![
                    PathCommand::Move { to: p(0, 0) },
                    PathCommand::Line { to: p(8 * U, 0) },
                    PathCommand::Line { to: p(0, 8 * U) },
                    PathCommand::Close,
                ],
            }],
            transforms: vec![
                node(None, [0, -U, U, 0], p(20 * U, 10 * U)),
                node(Some(0), [U, 0, 0, U], p(U, 2 * U)),
                node(Some(0), [U, 0, 0, U], p(2 * U, 3 * U)),
            ],
            instances: vec![
                PathInstance {
                    blend: Default::default(),
                    clip: None,
                    path: 0,
                    transform: Some(1),
                    brush: mo_raster::Brush::Solid {
                        rgba: [255, 0, 0, 255],
                    },
                    stroke: None,
                },
                PathInstance {
                    blend: Default::default(),
                    clip: None,
                    path: 0,
                    transform: Some(2),
                    brush: mo_raster::Brush::Solid {
                        rgba: [0, 0, 255, 128],
                    },
                    stroke: None,
                },
            ],
        },
    }
}
#[derive(Default)]
struct Fake {
    calls: usize,
    invalid: bool,
}
impl RasterBackend for Fake {
    fn raster(&mut self, f: &[u32]) -> Result<BackendReply, RasterError> {
        self.calls += 1;
        Ok(BackendReply {
            status: 0,
            pixels: vec![0; f[2] as usize * f[3] as usize * 4],
        })
    }
    fn invalidate(&mut self) {
        self.invalid = true;
    }
}
#[test]
fn translated_glyphs_share_linear_resources_and_quarter_turns_are_exact() {
    let c = compile(&request(), &|| false).unwrap();
    assert_eq!(c.work.compiled_paths, 1);
    assert_eq!(c.work.compiled_commands, 4);
    assert_eq!(c.work.transform_error_bound, Fixed::ZERO);
    assert_eq!(c.raster.work().draws, 2);
    assert_eq!(c.work.maximum_depth, 2);
    let f = c.raster.frame();
    assert_eq!(f[5], 1);
    assert_eq!(f[6], 2);
    assert_eq!(f[1], 4);
    assert_eq!(f[8], 0);
    let start = f.len() - 12;
    assert_eq!(f32::from_bits(f[start + 1]), 18.0);
    assert_eq!(f32::from_bits(f[start + 2]), 11.0);
}
#[test]
fn unused_invalid_grammar_graph_and_budgets_do_not_call_backend() {
    for mode in 0..6 {
        let mut q = request();
        match mode {
            0 => q.scene.transforms[0].parent = Some(1),
            1 => q.scene.instances[0].transform = Some(100),
            2 => q.scene.instances[0].path = 3,
            3 => q.scene.paths.push(FillPath {
                fill_rule: FillRule::Nonzero,
                commands: vec![PathCommand::Close],
            }),
            4 => {
                q.scene.transforms = (0..65)
                    .map(|i| {
                        node(
                            if i == 0 { None } else { Some(i - 1) },
                            [U, 0, 0, U],
                            p(0, 0),
                        )
                    })
                    .collect()
            }
            _ => q.viewport.width = 8193,
        }
        let mut b = Fake::default();
        assert!(render(&q, &mut b, &|| false).is_err());
        assert_eq!(b.calls, 0);
        assert!(!b.invalid);
    }
}
#[test]
fn unused_and_empty_transform_branches_do_not_change_lowering_strategy() {
    let mut q = request();
    q.scene
        .transforms
        .push(node(None, [1 << 120, 0, 0, U], p(0, 0)));
    q.scene
        .transforms
        .push(node(Some(3), [1 << 120, 0, 0, U], p(0, 0)));
    let before = compile(&q, &|| false).unwrap();
    assert_eq!(before.work.lowering_attempts, 1);
    q.scene.paths.push(FillPath {
        fill_rule: FillRule::Nonzero,
        commands: vec![],
    });
    for _ in 0..32 {
        q.scene.instances.push(PathInstance {
            blend: Default::default(),
            clip: None,
            path: 1,
            transform: Some(4),
            brush: mo_raster::Brush::Solid { rgba: [0; 4] },
            stroke: None,
        });
    }
    let c = compile(&q, &|| false).unwrap();
    assert_eq!(c.work.lowering_attempts, 1);
    assert_eq!(c.work.compiled_paths, 2);
}
#[test]
fn composite_range_is_an_optimization_failure_not_a_scene_failure() {
    let mut q = request();
    q.scene.paths[0].commands = vec![
        PathCommand::Move { to: p(0, 0) },
        PathCommand::Line { to: p(0, U) },
    ];
    q.scene.transforms = vec![
        node(None, [1 << 120, 0, 0, U], p(0, 0)),
        node(Some(0), [1 << 120, 0, 0, U], p(0, 0)),
    ];
    q.scene.instances.truncate(1);
    let c = compile(&q, &|| false).unwrap();
    assert_eq!(c.work.lowering_attempts, 2);
    assert_eq!(c.work.combined_coordinate_error_bound, Fixed::ZERO);
}
pub(super) fn precision_request() -> SceneRasterRequest {
    let mut q = request();
    q.viewport.width = 1024;
    q.viewport.height = 4;
    q.viewport.coordinate_tolerance = Fixed::from_raw(256);
    let inner = Affine {
        linear: [U / 7, 0, 0, U].map(Fixed::from_raw),
        translation: p(0, 0),
    };
    let outer = Affine {
        linear: [U / 3, 0, 0, U].map(Fixed::from_raw),
        translation: p(0, 0),
    };
    let x = 21000 * U;
    let exact = outer
        .map(inner.map(p(x, 0)).unwrap().point)
        .unwrap()
        .point
        .x
        .raw();
    q.scene.transforms = vec![
        node(None, [U / 3, 0, 0, U], p(1000 * U - exact, 0)),
        TransformNode {
            parent: Some(0),
            affine: inner,
        },
    ];
    q.scene.instances.truncate(1);
    q.scene.paths[0].commands = vec![
        PathCommand::Move { to: p(0, 0) },
        PathCommand::Line { to: p(x, 0) },
        PathCommand::Line { to: p(x, U) },
        PathCommand::Line { to: p(0, U) },
        PathCommand::Close,
    ];
    q
}
#[test]
fn original_chain_restores_a_strict_precision_budget() {
    let q = precision_request();
    let c = compile(&q, &|| false).unwrap();
    assert_eq!(c.work.lowering_attempts, 2);
    assert!(c.work.combined_coordinate_error_bound.raw() <= 256);
}
#[test]
fn all_cancellation_points_are_atomic_and_post_render_cancellation_discards() {
    for q in [request(), precision_request()] {
        let count = Cell::new(0);
        render(&q, &mut Fake::default(), &|| {
            count.set(count.get() + 1);
            false
        })
        .unwrap();
        for stop in 0..count.get() {
            let n = Cell::new(0);
            let mut b = Fake::default();
            let r = render(&q, &mut b, &|| {
                let old = n.get();
                n.set(old + 1);
                old == stop
            });
            assert!(
                matches!(r, Err(RasterError::Cancelled)),
                "checkpoint {stop}"
            );
            assert_eq!(b.invalid, b.calls != 0);
        }
    }
}
