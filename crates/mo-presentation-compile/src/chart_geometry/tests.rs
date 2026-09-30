use super::*;
use mo_charts::{
    DecimalNumber,
    sectors::{NegativeWeights, SectorDirection, SectorRequest, SectorWeight},
};
use std::cell::Cell;
type SampleCubic = ([f64; 2], [f64; 2], [f64; 2], [f64; 2]);

fn fixed(n: i64) -> Fixed {
    Fixed::from_raw(i128::from(n) << 32)
}
fn request(values: &[&str], inner: i64) -> ChartGeometryRequest {
    ChartGeometryRequest {
        sectors: SectorRequest {
            start_turn: Fixed::ZERO,
            direction: SectorDirection::Clockwise,
            negative_weights: NegativeWeights::Reject,
            weights: values
                .iter()
                .enumerate()
                .map(|(i, v)| SectorWeight {
                    point_index: (i * 3 + 7) as u32,
                    value: DecimalNumber::try_from((*v).to_owned()).unwrap(),
                })
                .collect(),
        },
        center: Point {
            x: fixed(100),
            y: fixed(-20),
        },
        outer_radius: fixed(1000),
        inner_radius: fixed(inner),
        coordinate_tolerance: Fixed::from_raw(1 << 22),
    }
}
fn run(r: &ChartGeometryRequest) -> ChartGeometry {
    compile(r, ChartGeometryLimits::default(), &|| false).unwrap()
}
fn value(v: Fixed) -> f64 {
    v.raw() as f64 / TURN as f64
}
fn xy(p: Point) -> [f64; 2] {
    [value(p.x), value(p.y)]
}
fn cubic(a: [f64; 2], b: [f64; 2], c: [f64; 2], d: [f64; 2], t: f64) -> [f64; 2] {
    let u = 1.0 - t;
    std::array::from_fn(|i| {
        u * u * u * a[i] + 3.0 * u * u * t * b[i] + 3.0 * u * t * t * c[i] + t * t * t * d[i]
    })
}

#[test]
fn pie_cardinal_boundaries_are_exact_and_shared() {
    let r = request(&["1", "1", "1", "1"], 0);
    let g = run(&r);
    let expected = [
        Point {
            x: fixed(100),
            y: fixed(-1020),
        },
        Point {
            x: fixed(1100),
            y: fixed(-20),
        },
        Point {
            x: fixed(100),
            y: fixed(980),
        },
        Point {
            x: fixed(-900),
            y: fixed(-20),
        },
    ];
    for (i, p) in g.paths.iter().enumerate() {
        assert_eq!(p.commands.first(), Some(&C::Move { to: expected[i] }));
        assert_eq!(
            p.commands[p.commands.len() - 3],
            match p.commands[p.commands.len() - 3].clone() {
                C::Cubic {
                    control1, control2, ..
                } => C::Cubic {
                    control1,
                    control2,
                    to: expected[(i + 1) % 4]
                },
                _ => panic!("outer arc ends in cubic"),
            }
        );
        assert_eq!(p.commands[p.commands.len() - 2], C::Line { to: r.center });
        assert_eq!(p.commands.last(), Some(&C::Close));
        assert!(p.coordinate_error_bound <= r.coordinate_tolerance);
    }
}

#[test]
fn complete_circles_have_no_radial_stroke_seam() {
    for inner in [0, 600] {
        let mut r = request(&["1"], inner);
        r.sectors.start_turn = Fixed::from_raw(123456789);
        for direction in [
            SectorDirection::Clockwise,
            SectorDirection::Counterclockwise,
        ] {
            r.sectors.direction = direction;
            let g = run(&r);
            let p = &g.paths[0];
            assert!(!p.commands.iter().any(|c| matches!(c, C::Line { .. })));
            assert_eq!(
                p.commands
                    .iter()
                    .filter(|c| matches!(c, C::Move { .. }))
                    .count(),
                if inner == 0 { 1 } else { 2 }
            );
            let mut start = None;
            let mut end = None;
            for c in &p.commands {
                match c {
                    C::Move { to } => start = Some(*to),
                    C::Cubic { to, .. } => end = Some(*to),
                    C::Close => assert_eq!(start, end),
                    _ => unreachable!(),
                }
            }
        }
    }
}

#[test]
fn arbitrary_adjacent_edges_and_reverse_winding_agree() {
    let mut r = request(&["1.001", "31", "0.003", "4e1"], 600);
    r.sectors.start_turn = Fixed::from_raw(4000000000);
    for direction in [
        SectorDirection::Clockwise,
        SectorDirection::Counterclockwise,
    ] {
        r.sectors.direction = direction;
        let g = run(&r);
        for (i, p) in g.paths.iter().enumerate() {
            let boundary = p
                .commands
                .iter()
                .position(|c| matches!(c, C::Line { .. }))
                .unwrap();
            let C::Cubic { to: outer_end, .. } = p.commands[boundary - 1] else {
                panic!()
            };
            let C::Move { to: next_start } = g.paths[(i + 1) % g.paths.len()].commands[0] else {
                panic!()
            };
            assert_eq!(outer_end, next_start);
            let C::Line { to: inner_end } = p.commands[boundary] else {
                panic!()
            };
            let next = &g.paths[(i + 1) % g.paths.len()].commands;
            let C::Cubic {
                to: inner_start, ..
            } = next[next.len() - 2]
            else {
                panic!()
            };
            assert_eq!(inner_end, inner_start);
        }
    }
}

#[test]
fn every_sampled_cubic_stays_within_reported_coordinate_error() {
    for inner in [0, 1, 999] {
        let mut r = request(&["1", "2", "7"], inner);
        r.sectors.start_turn = Fixed::from_raw(2973182781);
        for direction in [
            SectorDirection::Clockwise,
            SectorDirection::Counterclockwise,
        ] {
            r.sectors.direction = direction;
            let g = run(&r);
            for (p, sector) in g.paths.iter().zip(&g.layout.sectors) {
                let mut pen = [0.0; 2];
                let mut arc = Vec::new();
                let mut inner_arc = false;
                let check_arc = |arc: &[SampleCubic], reverse: bool| {
                    let radius = if reverse { inner as f64 } else { 1000.0 };
                    let (a, b) = if reverse {
                        (sector.end_turn, sector.start_turn)
                    } else {
                        (sector.start_turn, sector.end_turn)
                    };
                    for (j, (p0, p1, p2, p3)) in arc.iter().enumerate() {
                        for sample in 0..=64 {
                            let t = sample as f64 / 64.0;
                            let fraction = (j as f64 + t) / arc.len() as f64;
                            let angle = (value(a) + (value(b) - value(a)) * fraction)
                                * std::f64::consts::TAU;
                            let exact =
                                [100.0 + radius * angle.sin(), -20.0 - radius * angle.cos()];
                            let got = cubic(*p0, *p1, *p2, *p3, t);
                            for i in 0..2 {
                                assert!(
                                    (exact[i] - got[i]).abs()
                                        <= value(p.numeric_error_bound)
                                            + value(p.curve_error_bound)
                                            + 1e-10
                                );
                            }
                        }
                    }
                };
                for c in &p.commands {
                    match *c {
                        C::Move { to } => pen = xy(to),
                        C::Cubic {
                            control1,
                            control2,
                            to,
                        } => {
                            arc.push((pen, xy(control1), xy(control2), xy(to)));
                            pen = xy(to);
                        }
                        C::Line { to } => {
                            check_arc(&arc, inner_arc);
                            arc.clear();
                            inner_arc = true;
                            pen = xy(to);
                        }
                        C::Close => {
                            if !arc.is_empty() {
                                check_arc(&arc, inner_arc);
                                arc.clear();
                            }
                        }
                        _ => unreachable!(),
                    }
                }
            }
        }
    }
}

#[test]
fn zero_retained_and_positive_collapsed_sector_rejected() {
    let r = request(&["0", "1", "-0.00"], 0);
    let g = run(&r);
    assert!(g.paths[0].commands.is_empty());
    assert!(g.paths[2].commands.is_empty());
    assert_eq!(g.paths[0].point_index, 7);
    assert_eq!(run(&request(&["0", "0"], 500)).work.commands, 0);
    assert!(matches!(
        compile(
            &request(&["1e-4000", "1e4000"], 0),
            Default::default(),
            &|| false
        ),
        Err(ChartGeometryError::Precision)
    ));
}

#[test]
fn limits_are_global_across_sectors_and_both_arcs() {
    let r = request(&["1", "2", "3"], 500);
    let g = run(&r);
    let base = ChartGeometryLimits::default();
    for limits in [
        ChartGeometryLimits {
            max_paths: 2,
            ..base
        },
        ChartGeometryLimits {
            max_commands: g.work.commands - 1,
            ..base
        },
        ChartGeometryLimits {
            max_arc_segments: g.work.arc_segments - 1,
            ..base
        },
        ChartGeometryLimits {
            max_steps: g.work.steps - 1,
            ..base
        },
    ] {
        assert!(matches!(
            compile(&r, limits, &|| false),
            Err(ChartGeometryError::Limit(_))
        ));
    }
    let exact = ChartGeometryLimits {
        max_paths: g.work.paths,
        max_commands: g.work.commands,
        max_arc_segments: g.work.arc_segments,
        max_steps: g.work.steps,
        ..base
    };
    assert!(compile(&r, exact, &|| false).is_ok());
}

#[test]
fn interpolation_refines_when_angular_error_uses_most_of_the_budget() {
    let mut r = request(&["1"], 0);
    let coarse = run(&r);
    r.coordinate_tolerance = Fixed::from_raw(coarse.paths[0].angular_error_bound.raw() + 8);
    let fine = run(&r);
    assert!(fine.paths[0].arc_segments > coarse.paths[0].arc_segments);
    assert!(fine.paths[0].coordinate_error_bound <= r.coordinate_tolerance);
}

#[test]
fn invalid_geometry_and_unattainable_tolerance_fail_explicitly() {
    let r = request(&["1", "1"], 0);
    for bad in [
        ChartGeometryRequest {
            inner_radius: fixed(-1),
            ..r.clone()
        },
        ChartGeometryRequest {
            inner_radius: r.outer_radius,
            ..r.clone()
        },
        ChartGeometryRequest {
            outer_radius: Fixed::ZERO,
            ..r.clone()
        },
        ChartGeometryRequest {
            center: Point {
                x: Fixed::from_raw(i128::MAX),
                y: Fixed::ZERO,
            },
            ..r.clone()
        },
        ChartGeometryRequest {
            coordinate_tolerance: Fixed::ZERO,
            ..r.clone()
        },
    ] {
        assert!(matches!(
            compile(&bad, Default::default(), &|| false),
            Err(ChartGeometryError::Invalid(_))
        ));
    }
    let tiny = ChartGeometryRequest {
        coordinate_tolerance: Fixed::from_raw(1),
        ..r
    };
    assert!(matches!(
        compile(&tiny, Default::default(), &|| false),
        Err(ChartGeometryError::Precision)
    ));
}

#[test]
fn cancellation_returns_no_partial_path_at_every_stage() {
    let mut r = request(&["1"], 500);
    // Small subdivision still reaches every geometry/trig/cancellation stage;
    // exhaustive cancellation over a very fine mesh needlessly repeats them.
    r.coordinate_tolerance = fixed(10);
    let calls = Cell::new(0usize);
    compile(&r, Default::default(), &|| {
        calls.set(calls.get() + 1);
        false
    })
    .unwrap();
    for stop in 0..calls.get() {
        let seen = Cell::new(0usize);
        assert!(matches!(
            compile(&r, Default::default(), &|| {
                let n = seen.get();
                seen.set(n + 1);
                n >= stop
            }),
            Err(ChartGeometryError::Cancelled
                | ChartGeometryError::Sectors(sectors::SectorError::Cancelled))
        ));
    }
}
