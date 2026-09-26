use super::*;
use sha2::{Digest as _, Sha256};
use std::cell::Cell;
const FONT: &[u8] = include_bytes!("../../../../fixtures/fonts/owned.ttf");
fn request() -> FontMetricsRequest {
    FontMetricsRequest {
        expected_sha256: mo_common::Digest::from_sha256(Sha256::digest(FONT).into()),
        face_index: 0,
        instances: vec![FontMetricsInstance {
            variations: vec![],
            metrics: vec![FontMetric::HorizontalAscender, FontMetric::UnderlineOffset],
        }],
    }
}
#[derive(Default)]
struct Backend {
    raw: Option<Vec<u32>>,
    invalid: bool,
    calls: usize,
}
impl TextBackend for Backend {
    fn shape_batch(&mut self, _: &[u8], _: &[u32]) -> Result<Vec<u32>, TextError> {
        panic!("measurement must use its own operation")
    }
    fn measure_batch(&mut self, font: &[u8], frame: &[u32]) -> Result<Vec<u32>, TextError> {
        assert_eq!(font, FONT);
        self.calls += 1;
        let decoded = decode_metric_requests(frame)?;
        assert_eq!(decoded.len(), frame[3] as usize);
        if let Some(raw) = self.raw.take() {
            return Ok(raw);
        }
        let mut raw = vec![0, decoded.len() as u32];
        for r in decoded {
            assert_eq!(r[..3], [METRICS_MAGIC, 1, 0]);
            let tags = &r[6 + r[3] as usize * 2..];
            assert_eq!(tags.len(), r[4] as usize);
            raw.extend([
                6 + tags.len() as u32 * 3,
                METRICS_MAGIC,
                1,
                1000,
                64000,
                tags.len() as u32,
                0,
            ]);
            for (i, &tag) in tags.iter().enumerate() {
                raw.extend([tag, if i == 0 { 1 } else { 0 }, 0]);
            }
        }
        Ok(raw)
    }
    fn invalidate(&mut self) {
        self.invalid = true;
    }
}
#[test]
fn missing_values_and_real_zero_are_distinct_and_many_instances_batch_once() {
    let mut q = request();
    q.instances.push(q.instances[0].clone());
    q.instances[1].variations.push(ShapeVariation {
        tag: "wght".into(),
        value_16_16: 700 * 65536 + 1,
    });
    let mut b = Backend::default();
    let r = measure(&q, FONT, &mut b, &|| false).unwrap();
    assert_eq!(b.calls, 1);
    assert_eq!(r.instances[0].values[0].position, Some(0));
    assert_eq!(r.instances[0].values[1].position, None);
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
fn all_instance_parameters_are_checked_before_component_call() {
    let mut variants = Vec::new();
    let mut q = request();
    q.instances[0].metrics.push(FontMetric::HorizontalAscender);
    variants.push(q);
    let mut q = request();
    q.instances.push(FontMetricsInstance {
        metrics: vec![],
        variations: vec![ShapeVariation {
            tag: "xxxx".into(),
            value_16_16: 0,
        }],
    });
    variants.push(q);
    let mut q = request();
    q.instances[0].variations = vec![ShapeVariation {
        tag: "wght".into(),
        value_16_16: 999 * 65536,
    }];
    variants.push(q);
    let mut q = request();
    q.instances = vec![q.instances[0].clone(); 257];
    variants.push(q);
    let mut q = request();
    q.instances[0].metrics = vec![FontMetric::CapHeight; 29];
    variants.push(q);
    let mut q = request();
    q.expected_sha256 = mo_common::Digest::from_sha256([0; 32]);
    variants.push(q);
    for q in variants {
        let mut b = Backend::default();
        assert!(measure(&q, FONT, &mut b, &|| false).is_err());
        assert_eq!(b.calls, 0);
        assert!(!b.invalid);
    }
}
#[test]
fn empty_measurement_verifies_font_without_component_work() {
    let mut q = request();
    q.instances.clear();
    let mut b = Backend::default();
    let r = measure(&q, FONT, &mut b, &|| false).unwrap();
    assert!(r.instances.is_empty());
    assert_eq!(b.calls, 0);
}
#[test]
fn cancelling_empty_measurement_does_not_invalidate_an_unused_component() {
    let mut q = request();
    q.instances.clear();
    let total = Cell::new(0);
    measure(&q, FONT, &mut Backend::default(), &|| {
        total.set(total.get() + 1);
        false
    })
    .unwrap();
    for stop in 1..=total.get() {
        let at = Cell::new(0);
        let mut backend = Backend::default();
        assert!(matches!(
            measure(&q, FONT, &mut backend, &|| {
                at.set(at.get() + 1);
                at.get() == stop
            }),
            Err(TextError::Cancelled | TextError::Font(mo_font::FontError::Cancelled))
        ));
        assert_eq!(backend.calls, 0);
        assert!(!backend.invalid);
    }
}
#[test]
fn malformed_reply_or_failure_never_publishes_partial_metrics() {
    let q = request();
    let valid = vec![
        0,
        1,
        12,
        METRICS_MAGIC,
        1,
        1000,
        64000,
        2,
        0,
        FontMetric::HorizontalAscender.word(),
        1,
        123,
        FontMetric::UnderlineOffset.word(),
        0,
        0,
    ];
    let mut variants = vec![
        vec![],
        vec![0],
        vec![0, 2],
        vec![7, 0],
        vec![2, 0, 0],
        vec![2, 1],
    ];
    for (index, value) in [
        (2, 99999),
        (3, 0),
        (4, 2),
        (5, 999),
        (6, 63000),
        (7, 1),
        (8, 1),
        (9, 0),
        (10, 2),
        (14, 9),
    ] {
        let mut raw = valid.clone();
        raw[index] = value;
        variants.push(raw);
    }
    let mut extra = valid.clone();
    extra.push(0);
    variants.push(extra);
    for raw in variants {
        let mut b = Backend {
            raw: Some(raw),
            ..Default::default()
        };
        assert!(matches!(
            measure(&q, FONT, &mut b, &|| false),
            Err(TextError::BackendInvalid(_))
        ));
        assert!(b.invalid);
    }
    for code in 1..=6 {
        let mut b = Backend {
            raw: Some(vec![code, 0]),
            ..Default::default()
        };
        assert!(matches!(
            measure(&q, FONT, &mut b, &|| false),
            Err(TextError::BackendFailure { .. })
        ));
        assert_eq!(b.invalid, matches!(code, 2 | 6));
    }
}
#[test]
fn every_cancellation_checkpoint_discards_the_batch_and_postcall_invalidates() {
    let q = request();
    let n = Cell::new(0);
    measure(&q, FONT, &mut Backend::default(), &|| {
        n.set(n.get() + 1);
        false
    })
    .unwrap();
    for stop in 1..=n.get() {
        let mut b = Backend::default();
        let at = Cell::new(0);
        assert!(matches!(
            measure(&q, FONT, &mut b, &|| {
                at.set(at.get() + 1);
                at.get() == stop
            }),
            Err(TextError::Cancelled | TextError::Font(mo_font::FontError::Cancelled))
        ));
        assert_eq!(b.invalid, b.calls > 0);
    }
}
