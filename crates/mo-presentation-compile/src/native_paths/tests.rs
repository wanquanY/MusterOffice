use super::*;
use mo_common::Emu;
use mo_geometry::{Fixed, PathCommand as C};
use mo_presentation_model::Size;
use mo_presentation_source::source::{SourceObjectRef, SourceResolvedValue, geometry::evaluate::*};

fn point(x: f64, y: f64) -> EvaluatedPoint {
    EvaluatedPoint {
        origin: GeometryOrigin::document(5),
        x,
        y,
    }
}
fn geometry(commands: Vec<EvaluatedCommand>) -> EvaluatedGeometry {
    EvaluatedGeometry {
        source_ordinal: 1,
        extent: SourceResolvedValue {
            value: Size {
                width: Emu::new(300),
                height: Emu::new(200),
            },
            declared_by: SourceObjectRef {
                part: "/ppt/slides/slide1.xml".into(),
                native_id: 2,
            },
        },
        adjustments: vec![],
        guides: vec![],
        handles: vec![],
        connections: vec![],
        text_rect: None,
        paths: vec![EvaluatedPath {
            origin: GeometryOrigin::document(2),
            width: None,
            height: None,
            fill: None,
            stroke: None,
            extrusion_ok: None,
            commands,
        }],
    }
}
fn options() -> NativePathOptions {
    NativePathOptions {
        profile: NativePathProfile::DrawingmlPolarArcsDraftV1,
        coordinate_tolerance: Fixed::from_raw(1 << 20),
    }
}
fn compile(g: &EvaluatedGeometry) -> Vec<CompiledNativePath> {
    NativePathCompiler::new(options(), NativePathLimits::default(), &|| false)
        .unwrap()
        .compile(g)
        .unwrap()
}
fn value(f: Fixed) -> f64 {
    f.raw() as f64 / 4294967296.0
}
fn arc(start: f64, sweep: f64) -> EvaluatedCommand {
    EvaluatedCommand::Arc {
        origin: GeometryOrigin::document(8),
        width_radius: 2.0,
        height_radius: 1.0,
        start_angle: start,
        sweep_angle: sweep,
    }
}
#[test]
fn native_path_scaling_is_per_axis_and_omission_keeps_emu() {
    let mut g = geometry(vec![
        EvaluatedCommand::Move {
            origin: GeometryOrigin::document(3),
            to: point(1.0, 1.0),
        },
        EvaluatedCommand::Line {
            origin: GeometryOrigin::document(4),
            to: point(3.0, 4.0),
        },
    ]);
    g.paths[0].width = Some(Emu::new(3));
    g.paths[0].height = Some(Emu::new(4));
    let p = compile(&g).remove(0);
    let C::Move { to } = p.commands[0] else {
        panic!()
    };
    assert_eq!((value(to.x), value(to.y)), (100.0, 50.0));
    let C::Line { to } = p.commands[1] else {
        panic!()
    };
    assert_eq!((value(to.x), value(to.y)), (300.0, 200.0));
    assert_eq!(p.coordinate_error_bound.x, Fixed::ZERO);
    g.paths[0].height = None;
    let p = compile(&g).remove(0);
    let C::Line { to } = p.commands[1] else {
        panic!()
    };
    assert_eq!(value(to.y), 4.0);
    g.paths[0].width = Some(Emu::new(0));
    assert!(matches!(
        NativePathCompiler::new(options(), NativePathLimits::default(), &|| false)
            .unwrap()
            .compile(&g),
        Err(NativePathError::Geometry {
            issue: NativePathIssue::ZeroPathExtent,
            ..
        })
    ));
}
#[test]
fn implicit_origin_and_close_continuation_keep_provenance() {
    let g = geometry(vec![
        EvaluatedCommand::Line {
            origin: GeometryOrigin::document(4),
            to: point(4.0, 5.0),
        },
        EvaluatedCommand::Close {
            origin: GeometryOrigin::document(6),
        },
        EvaluatedCommand::Line {
            origin: GeometryOrigin::document(7),
            to: point(7.0, 8.0),
        },
    ]);
    let p = compile(&g).remove(0);
    assert_eq!(p.commands.len(), 5);
    assert_eq!(p.source_map[0].command_count, 2);
    assert_eq!(p.source_map[2].first_command, 3);
    let C::Move { to } = p.commands[3] else {
        panic!()
    };
    assert_eq!(to.x, Fixed::ZERO);
    assert_eq!(to.y, Fixed::ZERO);
    let mut budget = mo_geometry::BoundsBudget::new(1000);
    assert!(
        mo_geometry::path_bounds(&p.commands, Fixed::from_raw(256), &mut budget, &|| false).is_ok()
    );
}
#[test]
fn polar_ellipse_endpoints_are_not_parameter_angles() {
    for direction in [-1.0, 1.0] {
        let g = geometry(vec![
            EvaluatedCommand::Move {
                origin: GeometryOrigin::document(3),
                to: point(10.0, 20.0),
            },
            arc(2700000.0, direction * 5400000.0),
        ]);
        let p = compile(&g).remove(0);
        let C::Cubic { to, .. } = p.commands.last().unwrap() else {
            panic!()
        };
        // 45-degree radial intersection has x=y=2/sqrt(5).
        let offset = 1.7888543819998318;
        let expected = if direction > 0.0 {
            (10.0 - offset, 20.0)
        } else {
            (10.0, 20.0 - offset)
        };
        assert!((value(to.x) - expected.0).abs() < 1e-8);
        assert!((value(to.y) - expected.1).abs() < 1e-8);
        assert!(p.arc_segments > 1);
        assert!(p.coordinate_error_bound.x <= options().coordinate_tolerance);
        assert_eq!(p.source_map[1].command_count, p.arc_segments);
    }
}
#[test]
fn signed_multiturn_arcs_preserve_winding_and_zero_sweep() {
    for sweep in [-43200000.0, 43200000.0] {
        let g = geometry(vec![
            EvaluatedCommand::Move {
                origin: GeometryOrigin::document(3),
                to: point(2.0, 0.0),
            },
            arc(0.0, sweep),
        ]);
        let p = compile(&g).remove(0);
        let C::Cubic { to, .. } = p.commands.last().unwrap() else {
            panic!()
        };
        assert!((value(to.x) - 2.0).abs() < 1e-8);
        assert!(value(to.y).abs() < 1e-8);
        assert!(p.arc_segments >= 16);
    }
    let p = compile(&geometry(vec![arc(1e100, 0.0)])).remove(0);
    assert_eq!(p.arc_segments, 0);
    assert_eq!(p.commands.len(), 1);
    let a = compile(&geometry(vec![arc(5400000.0, 5400000.0)]));
    let b = compile(&geometry(vec![arc(
        21600000.0 * 1000000.0 + 5400000.0,
        5400000.0,
    )]));
    assert_eq!(a[0].commands, b[0].commands);
}
#[test]
fn compilation_budget_is_shared_and_cancellation_is_not_partial() {
    let g = geometry(vec![
        EvaluatedCommand::Move {
            origin: GeometryOrigin::document(3),
            to: point(0.0, 0.0),
        },
        EvaluatedCommand::Line {
            origin: GeometryOrigin::document(4),
            to: point(1.0, 1.0),
        },
    ]);
    let mut c = NativePathCompiler::new(
        options(),
        NativePathLimits {
            max_commands: 3,
            ..NativePathLimits::default()
        },
        &|| false,
    )
    .unwrap();
    assert!(c.compile(&g).is_ok());
    assert!(matches!(
        c.compile(&g),
        Err(NativePathError::Limit("commands"))
    ));
    let mut c = NativePathCompiler::new(options(), NativePathLimits::default(), &|| true).unwrap();
    assert!(matches!(c.compile(&g), Err(NativePathError::Cancelled)));
    let g = geometry(vec![arc(0.0, 5400000.0)]);
    let mut c = NativePathCompiler::new(
        options(),
        NativePathLimits {
            max_arc_segments: 1,
            ..NativePathLimits::default()
        },
        &|| false,
    )
    .unwrap();
    assert!(matches!(
        c.compile(&g),
        Err(NativePathError::Limit("arc segments"))
    ));
}
#[test]
fn invalid_numeric_inputs_fail_without_zero_substitution() {
    let mut g = geometry(vec![arc(0.0, 5400000.0)]);
    for radius in [-1.0, 0.0, f64::NAN, f64::INFINITY] {
        let EvaluatedCommand::Arc { width_radius, .. } = &mut g.paths[0].commands[0] else {
            panic!()
        };
        *width_radius = radius;
        assert!(matches!(
            NativePathCompiler::new(options(), NativePathLimits::default(), &|| false)
                .unwrap()
                .compile(&g),
            Err(NativePathError::Geometry { .. })
        ));
    }
    let mut o = options();
    o.coordinate_tolerance = Fixed::ZERO;
    assert!(matches!(
        NativePathCompiler::new(o, NativePathLimits::default(), &|| false),
        Err(NativePathError::Options)
    ));
}

#[test]
fn point_ellipse_is_an_empty_arc_at_the_existing_pen() {
    let mut a = arc(2700000.0, 5400000.0);
    let EvaluatedCommand::Arc {
        width_radius,
        height_radius,
        ..
    } = &mut a
    else {
        panic!()
    };
    *width_radius = 0.0;
    *height_radius = 0.0;
    let p = compile(&geometry(vec![
        EvaluatedCommand::Move {
            origin: GeometryOrigin::document(3),
            to: point(10.0, 20.0),
        },
        a,
        EvaluatedCommand::Line {
            origin: GeometryOrigin::document(9),
            to: point(30.0, 40.0),
        },
    ]))
    .remove(0);
    assert_eq!(p.commands.len(), 2);
    assert_eq!(p.source_map[1].command_count, 0);
    assert_eq!(p.arc_segments, 0);
}

#[test]
fn dependent_precision_keeps_the_shared_compilation_budget() {
    let g = geometry(vec![
        EvaluatedCommand::Move {
            origin: GeometryOrigin::document(3),
            to: point(0.0, 0.0),
        },
        EvaluatedCommand::Line {
            origin: GeometryOrigin::document(4),
            to: point(1.0, 1.0),
        },
    ]);
    let mut c = NativePathCompiler::new(
        options(),
        NativePathLimits {
            max_commands: 3,
            ..NativePathLimits::default()
        },
        &|| false,
    )
    .unwrap();
    assert!(matches!(
        c.compile_with_tolerance(&g, Fixed::from_raw(1 << 21)),
        Err(NativePathError::Options)
    ));
    assert!(
        c.compile_with_tolerance(&g, Fixed::from_raw(1 << 16))
            .is_ok()
    );
    assert!(matches!(
        c.compile(&g),
        Err(NativePathError::Limit("commands"))
    ));
}
