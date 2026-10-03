use super::*;
use sha2::{Digest as _, Sha256};
use std::cell::Cell;
const FONT: &[u8] = include_bytes!("../../../../fixtures/fonts/owned-carets.ttf");
fn request() -> FontCaretsRequest {
    FontCaretsRequest {
        expected_sha256: mo_common::Digest::from_sha256(Sha256::digest(FONT).into()),
        face_index: 0,
        instances: vec![FontCaretsInstance {
            variations: vec![],
            direction: Direction::LeftToRight,
            glyph_ids: vec![1, 2],
        }],
    }
}
#[derive(Default)]
struct Backend {
    raw: Option<Vec<u32>>,
    calls: usize,
    invalid: bool,
    fail: bool,
}
impl TextBackend for Backend {
    fn shape_batch(&mut self, _: &[u8], _: &[u32]) -> Result<Vec<u32>, TextError> {
        panic!("wrong operation")
    }
    fn caret_batch(&mut self, font: &[u8], frame: &[u32]) -> Result<Vec<u32>, TextError> {
        assert_eq!(font, FONT);
        self.calls += 1;
        if self.fail {
            return Err(TextError::Host("injected"));
        }
        if let Some(raw) = self.raw.take() {
            return Ok(raw);
        }
        let requests = decode_caret_requests(frame)?;
        let mut raw = vec![0, requests.len() as u32];
        for r in requests {
            assert_eq!(r[..3], [CARETS_MAGIC, 1, 0]);
            let ids = &r[8 + 2 * r[3] as usize..];
            assert_eq!(ids, [1, 2]);
            raw.extend([
                13,
                CARETS_MAGIC,
                1,
                1000,
                64000,
                r[4],
                2,
                1,
                0,
                2,
                3,
                (-3200i32) as u32,
                0,
                11072,
            ]);
        }
        Ok(raw)
    }
    fn invalidate(&mut self) {
        self.invalid = true;
    }
}
#[test]
fn absent_zero_signed_positions_axes_and_direction_survive_one_batch() {
    let mut q = request();
    q.instances.push(q.instances[0].clone());
    q.instances[1].direction = Direction::BottomToTop;
    q.instances[1].variations.push(ShapeVariation {
        tag: "wght".into(),
        value_16_16: 700 * 65536 + 1,
    });
    let mut b = Backend::default();
    let r = query(&q, FONT, &mut b, &|| false).unwrap();
    assert_eq!(b.calls, 1);
    assert_eq!(r.position_units_per_em, 64000);
    assert!(r.instances[0].glyphs[0].positions.is_empty());
    assert_eq!(r.instances[0].glyphs[1].positions, [-3200, 0, 11072]);
    assert_eq!(r.instances[1].direction, Direction::BottomToTop);
    assert_eq!(
        r.instances[1].effective_variations[0].requested_16_16,
        700 * 65536 + 1
    );
    assert_eq!(
        r.instances[1].effective_variations[0].effective_f32_bits,
        700f32.to_bits()
    );
}
#[test]
fn late_invalid_instance_or_binding_never_calls_or_invalidates_component() {
    for variant in 0..10 {
        let mut q = request();
        q.instances.push(q.instances[0].clone());
        match variant {
            0 => q.instances[1].glyph_ids.push(1),
            1 => q.instances[1].glyph_ids = vec![7],
            2 => q.instances[1].variations.push(ShapeVariation {
                tag: "xxxx".into(),
                value_16_16: 0,
            }),
            3 => q.instances[1].variations.push(ShapeVariation {
                tag: "wght".into(),
                value_16_16: 901 * 65536,
            }),
            4 => q.instances = vec![q.instances[0].clone(); 65],
            5 => q.instances[1].glyph_ids = vec![1; 257],
            6 => {
                q.instances[0].glyph_ids = vec![1; 256];
                q.instances = vec![q.instances[0].clone(); 17];
            }
            7 => {
                q.instances[1].variations = vec![
                    ShapeVariation {
                        tag: "wght".into(),
                        value_16_16: 400 * 65536
                    };
                    65
                ]
            }
            8 => q.expected_sha256 = mo_common::Digest::from_sha256([0; 32]),
            _ => q.face_index = 999,
        }
        let mut b = Backend::default();
        assert!(
            query(&q, FONT, &mut b, &|| false).is_err(),
            "variant {variant}"
        );
        assert_eq!(b.calls, 0);
        assert!(!b.invalid);
    }
    let mut q = request();
    let font = VerifiedFont::load(&q.expected_sha256, 0, FONT, FontLimits::default(), &|| {
        false
    })
    .unwrap();
    q.face_index = 1;
    let mut b = Backend::default();
    assert!(matches!(
        query_verified(&q, &font, &mut b, &|| false),
        Err(TextError::Font(mo_font::FontError::ResourceConflict))
    ));
    assert_eq!(b.calls, 0);
}
#[test]
fn malformed_reply_failure_or_host_error_cannot_publish_partial_carets() {
    let q = request();
    let good = vec![
        0,
        1,
        13,
        CARETS_MAGIC,
        1,
        1000,
        64000,
        4,
        2,
        1,
        0,
        2,
        3,
        (-3200i32) as u32,
        0,
        11072,
    ];
    let mut variants = vec![
        vec![],
        vec![0],
        vec![0, 2],
        vec![7, 0],
        vec![2, 1],
        vec![2, 0, 0],
    ];
    for (i, v) in [
        (2, 5),
        (2, 99999),
        (3, 0),
        (4, 2),
        (5, 999),
        (6, 63000),
        (7, 5),
        (8, 1),
        (9, 2),
        (10, 65),
        (11, 3),
        (12, 65),
    ] {
        let mut raw = good.clone();
        raw[i] = v;
        variants.push(raw);
    }
    for end in 2..good.len() {
        variants.push(good[..end].to_vec());
    }
    let mut extra = good.clone();
    extra.push(0);
    variants.push(extra);
    for raw in variants {
        let mut b = Backend {
            raw: Some(raw),
            ..Default::default()
        };
        assert!(matches!(
            query(&q, FONT, &mut b, &|| false),
            Err(TextError::BackendInvalid(_))
        ));
        assert!(b.invalid);
    }
    for status in 1..=6 {
        let mut b = Backend {
            raw: Some(vec![status, 0]),
            ..Default::default()
        };
        assert!(matches!(
            query(&q, FONT, &mut b, &|| false),
            Err(TextError::BackendFailure { .. })
        ));
        assert_eq!(b.invalid, matches!(status, 2 | 6));
    }
    let mut b = Backend {
        fail: true,
        ..Default::default()
    };
    assert!(matches!(
        query(&q, FONT, &mut b, &|| false),
        Err(TextError::Host(_))
    ));
    assert!(b.invalid);
}
#[test]
fn every_cancellation_checkpoint_discards_results_and_only_postcall_invalidates() {
    for empty in [false, true] {
        let mut q = request();
        if empty {
            q.instances.clear();
        }
        let n = Cell::new(0);
        let mut b = Backend::default();
        let r = query(&q, FONT, &mut b, &|| {
            n.set(n.get() + 1);
            false
        })
        .unwrap();
        assert_eq!(r.instances.len(), usize::from(!empty));
        assert_eq!(b.calls, usize::from(!empty));
        for stop in 1..=n.get() {
            let at = Cell::new(0);
            let mut b = Backend::default();
            assert!(matches!(
                query(&q, FONT, &mut b, &|| {
                    at.set(at.get() + 1);
                    at.get() == stop
                }),
                Err(TextError::Cancelled | TextError::Font(mo_font::FontError::Cancelled))
            ));
            assert_eq!(b.invalid, b.calls > 0);
        }
    }
}
#[test]
fn transport_rejects_truncation_trailing_and_out_of_bounds_instances() {
    let good = [
        CARETS_BATCH_MAGIC,
        1,
        HARFBUZZ_VERSION,
        1,
        8,
        CARETS_MAGIC,
        1,
        0,
        0,
        4,
        0,
        64,
        0,
    ];
    assert_eq!(decode_caret_requests(&good).unwrap(), vec![&good[5..]]);
    for end in 0..good.len() {
        assert!(decode_caret_requests(&good[..end]).is_err());
    }
    for (i, v) in [
        (0, 0),
        (1, 2),
        (2, 0),
        (3, 65),
        (4, 7),
        (4, 393),
        (4, u32::MAX),
    ] {
        let mut bad = good;
        bad[i] = v;
        assert!(decode_caret_requests(&bad).is_err());
    }
    let mut bad = good.to_vec();
    bad.push(0);
    assert!(decode_caret_requests(&bad).is_err());
}
