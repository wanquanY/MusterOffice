use super::*;
use crate::*;
use std::cell::Cell;

struct Backend<'a> {
    calls: &'a Cell<usize>,
    invalid: bool,
    reply: Option<Result<PickingReply, RasterError>>,
}
impl RasterBackend for Backend<'_> {
    fn pick(&mut self, _: &[u32]) -> Result<PickingReply, RasterError> {
        self.calls.set(self.calls.get() + 1);
        self.reply.take().unwrap()
    }
    fn raster(&mut self, _: &[u32]) -> Result<BackendReply, RasterError> {
        panic!("picking cannot rasterize")
    }
    fn invalidate(&mut self) {
        self.invalid = true;
    }
}
fn point(x: i128, y: i128) -> Point {
    Point {
        x: Fixed::from_raw(x << 32),
        y: Fixed::from_raw(y << 32),
    }
}
fn query(radius: i128) -> DevicePickQuery {
    DevicePickQuery {
        point: point(2, 3),
        radius: Fixed::from_raw(radius << 32),
    }
}
fn compiled(draws: usize) -> CompiledPicking {
    let mut q = crate::tests::request();
    q.draws.resize(draws, q.draws[0].clone());
    compile(&q, &|| false).unwrap().picking(&|| false).unwrap()
}
fn success(draws: u32, exact: &[u32], nearby: &[u32]) -> PickingReply {
    PickingReply {
        status: 0,
        words: [
            vec![PICK_MAGIC, 1, 1, draws, draws.div_ceil(32), 0, 1],
            exact.to_vec(),
            nearby.to_vec(),
            vec![1],
        ]
        .concat(),
    }
}
#[test]
fn reply_bits_preserve_reverse_paint_order_and_zero_draws() {
    for (draws, reply, expected) in [
        (0, success(0, &[], &[]), vec![]),
        (
            35,
            success(35, &[1, 4], &[1 << 31, 1]),
            vec![
                DrawHit {
                    draw: 34,
                    kind: DrawHitKind::Exact,
                },
                DrawHit {
                    draw: 32,
                    kind: DrawHitKind::Nearby,
                },
                DrawHit {
                    draw: 31,
                    kind: DrawHitKind::Nearby,
                },
                DrawHit {
                    draw: 0,
                    kind: DrawHitKind::Exact,
                },
            ],
        ),
    ] {
        let calls = Cell::new(0);
        let mut b = Backend {
            calls: &calls,
            invalid: false,
            reply: Some(Ok(reply)),
        };
        let result = compiled(draws)
            .query(&[query(3)], &mut b, &|| false)
            .unwrap();
        assert_eq!(result[0].hits().collect::<Vec<_>>(), expected);
        assert_eq!(calls.get(), 1);
        assert!(!b.invalid);
    }
}

#[test]
fn semantic_sample_uses_the_same_ties_even_device_point_as_component_geometry() {
    let calls = Cell::new(0);
    let mut b = Backend {
        calls: &calls,
        invalid: false,
        reply: Some(Ok(success(1, &[1], &[0]))),
    };
    let point = Point {
        x: Fixed::from_raw((1 << 32) + 256),
        y: Fixed::from_raw(-((1 << 32) + 768)),
    };
    let result = compiled(1)
        .query(
            &[DevicePickQuery {
                point,
                radius: Fixed::ZERO,
            }],
            &mut b,
            &|| false,
        )
        .unwrap();
    assert_eq!(
        result[0].sampled_point(),
        Point {
            x: Fixed::from_raw(1 << 32),
            y: Fixed::from_raw(-((1 << 32) + 1024))
        }
    );
}
#[test]
fn invalid_input_and_empty_queries_never_call_or_invalidate_backend() {
    let c = compiled(1);
    let calls = Cell::new(0);
    let mut b = Backend {
        calls: &calls,
        invalid: false,
        reply: None,
    };
    assert!(c.query(&[], &mut b, &|| false).unwrap().is_empty());
    assert!(matches!(
        c.query(&[query(0); 65], &mut b, &|| false),
        Err(RasterError::Limit(_))
    ));
    for q in [
        query(-1),
        query(33),
        DevicePickQuery {
            point: point(32769, 0),
            ..query(0)
        },
    ] {
        assert!(c.query(&[q], &mut b, &|| false).is_err());
    }
    assert!(matches!(
        c.query(&[query(0)], &mut b, &|| true),
        Err(RasterError::Cancelled)
    ));
    assert_eq!(calls.get(), 0);
    assert!(!b.invalid);
}
#[test]
fn malformed_status_shape_overlap_and_unused_bits_quarantine_backend() {
    let c = compiled(1);
    let good = success(1, &[1], &[0]);
    let mut failures = vec![
        success(1, &[1], &[1]),
        success(1, &[2], &[0]),
        success(1, &[0], &[1]),
        PickingReply {
            status: 5,
            words: vec![],
        },
        PickingReply {
            status: 1,
            words: vec![0],
        },
    ];
    for i in 0..good.words.len() {
        failures.push(PickingReply {
            status: 0,
            words: good.words[..i].to_vec(),
        });
    }
    for i in 0..7 {
        let mut words = good.words.clone();
        words[i] += 1;
        failures.push(PickingReply { status: 0, words });
    }
    for bits in [0, 2, u32::MAX] {
        let mut words = good.words.clone();
        *words.last_mut().unwrap() = bits;
        failures.push(PickingReply { status: 0, words });
    }
    let mut extra = good.words.clone();
    extra.push(0);
    failures.push(PickingReply {
        status: 0,
        words: extra,
    });
    for reply in failures {
        let calls = Cell::new(0);
        let mut b = Backend {
            calls: &calls,
            invalid: false,
            reply: Some(Ok(reply)),
        };
        assert!(matches!(
            c.query(&[query(0)], &mut b, &|| false),
            Err(RasterError::ComponentInvalid(_))
        ));
        assert!(b.invalid);
        assert_eq!(calls.get(), 1);
    }
}
#[test]
fn component_status_and_post_call_cancellation_have_explicit_lifetimes() {
    let c = compiled(1);
    for status in 1..=4 {
        let calls = Cell::new(0);
        let mut b = Backend {
            calls: &calls,
            invalid: false,
            reply: Some(Ok(PickingReply {
                status,
                words: vec![],
            })),
        };
        assert!(
            matches!(c.query(&[query(0)], &mut b, &|| false), Err(RasterError::Component(s)) if s == status)
        );
        assert_eq!(b.invalid, matches!(status, 2 | 4));
    }
    let calls = Cell::new(0);
    let mut b = Backend {
        calls: &calls,
        invalid: false,
        reply: Some(Ok(success(1, &[1], &[0]))),
    };
    assert!(matches!(
        c.query(&[query(0)], &mut b, &|| calls.get() > 0),
        Err(RasterError::Cancelled)
    ));
    assert!(b.invalid);
    let mut b = Backend {
        calls: &calls,
        invalid: false,
        reply: Some(Err(RasterError::Host("trap"))),
    };
    assert!(c.query(&[query(0)], &mut b, &|| false).is_err());
    assert!(b.invalid);
}
#[test]
fn projection_is_geometry_only_across_compositing_profiles_and_rebasing() {
    let mut q = crate::tests::request();
    let base = compile(&q, &|| false).unwrap().picking(&|| false).unwrap();
    q.draws[0].brush = Brush::Solid { rgba: [0; 4] };
    q.draws[0].blend = BlendMode::Source;
    let c = compile(&q, &|| false).unwrap();
    assert_eq!(c.frame()[1], 8);
    assert_eq!(c.picking(&|| false).unwrap().frame, base.frame);
    q.opacity_groups.push(OpacityGroup {
        first_draw: 0,
        end_draw: 1,
        opacity: 0,
    });
    let c = compile(&q, &|| false).unwrap();
    assert_eq!(c.frame()[1], 13);
    assert_eq!(c.picking(&|| false).unwrap().frame, base.frame);
    q.draws[0].brush = Brush::Snapshot {
        after_draws: 0,
        scope: SnapshotScope::Output,
    };
    let c = compile(&q, &|| false).unwrap();
    assert_eq!(c.frame()[1], 14);
    assert_eq!(c.picking(&|| false).unwrap().frame, base.frame);
    q.viewport.origin = point(1 << 80, -(1 << 80));
    q.draws[0].origin = q.viewport.origin;
    assert_eq!(
        compile(&q, &|| false)
            .unwrap()
            .picking(&|| false)
            .unwrap()
            .frame,
        base.frame
    );
    assert!(matches!(c.picking(&|| true), Err(RasterError::Cancelled)));
}
