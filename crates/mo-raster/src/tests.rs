use super::*;
use mo_geometry::{Fixed, PathCommand, Point};
use std::cell::Cell;
fn p(x: i128, y: i128) -> Point {
    Point {
        x: Fixed::from_raw(x << 32),
        y: Fixed::from_raw(y << 32),
    }
}
pub(super) fn request() -> PathRasterRequest {
    PathRasterRequest {
        clips: vec![],
        viewport: RasterViewport {
            width: 16,
            height: 16,
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
                PathCommand::Move { to: p(1, 2) },
                PathCommand::Line { to: p(5, 2) },
                PathCommand::Line { to: p(5, 6) },
                PathCommand::Close,
            ],
        }],
        draws: vec![PathDraw {
            blend: Default::default(),
            clip: None,
            path: 0,
            origin: p(0, 0),
            brush: Brush::Solid {
                rgba: [255, 0, 0, 255],
            },
            stroke: None,
        }],
    }
}
#[test]
fn exact_conversion_avoids_double_rounding_and_uses_ties_even() {
    let s = number::Scale::new(PixelScale {
        numerator: 1,
        denominator: 1,
    })
    .unwrap();
    for sign in [-1i128, 1] {
        let sign_bit = if sign < 0 { 1 << 31 } else { 0 };
        assert_eq!(
            s.value(sign * ((1 << 32) + 256)).unwrap().to_bits(),
            sign_bit | 0x3f800000
        );
        assert_eq!(
            s.value(sign * ((1 << 32) + 768)).unwrap().to_bits(),
            sign_bit | 0x3f800002
        );
    }
    let d = u32::MAX;
    let raw = ((1i128 << 32) + 256) * i128::from(d) + 1;
    let s = number::Scale::new(PixelScale {
        numerator: 1,
        denominator: d,
    })
    .unwrap();
    assert_eq!(s.value(raw).unwrap().to_bits(), 0x3f800001);
    assert_eq!(
        (raw as f64 / (f64::from(d) * 4294967296.0)) as f32,
        f32::from_bits(0x3f800000)
    );
    assert!(s.value(i128::MAX).is_err());
    assert!(s.value(i128::MIN).is_err());
    assert_eq!(s.value(1).unwrap().to_bits(), 0x1f800000);
}
#[test]
fn integer_interval_contains_small_and_signed_float_errors() {
    let s = number::Scale::new(PixelScale {
        numerator: 1,
        denominator: 3,
    })
    .unwrap();
    for raw in [1, -1, 1 << 32, -(1 << 32), 999999 << 24] {
        let actual = s.value(raw).unwrap();
        let error = s.error(raw, actual).unwrap();
        let observed = ((f64::from(actual) * 4294967296.0) - raw as f64 / 3.0).abs();
        assert!(observed <= error as f64);
        assert!(error as f64 - observed <= 2.0);
    }
    assert_eq!(
        number::add_sub(i128::MAX, i128::MAX, i128::MAX).unwrap(),
        i128::MAX
    );
    assert_eq!(
        number::add_sub(i128::MIN, i128::MIN, i128::MIN).unwrap(),
        i128::MIN
    );
}
#[test]
fn rebasing_preserves_shared_paths_and_huge_origin_invariance() {
    let mut q = request();
    q.draws.push(PathDraw {
        blend: Default::default(),
        clip: None,
        path: 0,
        origin: p(3, 4),
        brush: Brush::Solid {
            rgba: [0, 0, 255, 255],
        },
        stroke: None,
    });
    let before = compile(&q, &|| false).unwrap();
    assert_eq!(before.work.paths, 1);
    assert_eq!(before.work.commands, 4);
    assert_eq!(before.work.drawn_commands, 8);
    assert_eq!(before.work.coordinate_error_bound, Fixed::ZERO);
    let huge = Fixed::from_raw(1 << 110);
    q.viewport.origin.x = huge;
    for d in &mut q.draws {
        d.origin.x = d.origin.x.checked_add(huge).unwrap();
    }
    assert_eq!(before.frame(), compile(&q, &|| false).unwrap().frame());
    let mut q = request();
    q.viewport.origin.x = huge;
    for c in &mut q.paths[0].commands {
        if let PathCommand::Move { to } | PathCommand::Line { to } = c {
            to.x = to.x.checked_add(huge).unwrap();
        }
    }
    assert_eq!(
        compile(&request(), &|| false).unwrap().frame(),
        compile(&q, &|| false).unwrap().frame()
    );
    q.viewport.scale = PixelScale {
        numerator: u32::MAX,
        denominator: u32::MAX,
    };
    assert_eq!(
        compile(&request(), &|| false).unwrap().frame(),
        compile(&q, &|| false).unwrap().frame()
    );
}
#[test]
fn precision_and_range_fail_explicitly() {
    let mut q = request();
    q.viewport.scale.denominator = 3;
    q.draws[0].origin = p(20000, 0);
    q.viewport.coordinate_tolerance = Fixed::from_raw(256);
    assert!(matches!(
        compile(&q, &|| false),
        Err(RasterError::Precision)
    ));
    q.viewport.coordinate_tolerance = Fixed::from_raw(1 << 24);
    assert!(compile(&q, &|| false).is_ok());
    q.draws[0].origin = p(100000, 0);
    assert!(matches!(compile(&q, &|| false), Err(RasterError::Range)));
    q = request();
    q.viewport.scale.numerator = 0;
    assert!(matches!(
        compile(&q, &|| false),
        Err(RasterError::Invalid(_))
    ));
}
#[test]
fn invalid_geometry_and_references_never_reach_component() {
    for mode in 0..5 {
        let mut q = request();
        match mode {
            0 => q.draws[0].path = 1,
            1 => q.paths[0].commands[0] = PathCommand::Close,
            2 => q.paths[0].commands.push(PathCommand::Close),
            3 => q.viewport.width = 8193,
            _ => q.viewport.coordinate_tolerance = Fixed::ZERO,
        }
        let mut backend = Fake::default();
        assert!(render(&q, &mut backend, &|| false).is_err());
        assert_eq!(backend.calls, 0);
        assert!(!backend.invalid);
    }
}
#[derive(Default)]
struct Fake {
    calls: usize,
    invalid: bool,
    mode: u8,
}
impl RasterBackend for Fake {
    fn raster(&mut self, frame: &[u32]) -> Result<BackendReply, RasterError> {
        self.calls += 1;
        if self.mode == 6 {
            return Err(RasterError::Host("injected"));
        }
        let mut pixels = vec![0; frame[2] as usize * frame[3] as usize * 4];
        let status = match self.mode {
            3 => 1,
            4 => 2,
            5 => 3,
            _ => 0,
        };
        match self.mode {
            1 => {
                pixels.pop();
            }
            2 => pixels[0] = 1,
            4 | 5 => pixels.clear(),
            _ => {}
        }
        Ok(BackendReply { status, pixels })
    }
    fn invalidate(&mut self) {
        self.invalid = true;
    }
}
#[test]
fn malformed_results_and_fatal_failures_invalidate() {
    for mode in 1..=6 {
        let mut backend = Fake {
            mode,
            ..Fake::default()
        };
        assert!(render(&request(), &mut backend, &|| false).is_err());
        assert_eq!(backend.invalid, mode != 5);
    }
}
#[test]
fn cancellation_never_publishes_and_post_call_cancel_discards_instance() {
    let count = Cell::new(0);
    render(&request(), &mut Fake::default(), &|| {
        count.set(count.get() + 1);
        false
    })
    .unwrap();
    for stop in 0..count.get() {
        let current = Cell::new(0);
        let mut backend = Fake::default();
        let result = render(&request(), &mut backend, &|| {
            let n = current.get();
            current.set(n + 1);
            n == stop
        });
        assert!(
            matches!(result, Err(RasterError::Cancelled)),
            "checkpoint {stop}"
        );
        assert_eq!(backend.invalid, backend.calls != 0);
    }
}
