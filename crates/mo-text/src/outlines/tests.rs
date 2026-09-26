use super::*;
use sha2::{Digest as _, Sha256};
use std::cell::Cell;
const FONT: &[u8] = include_bytes!("../../../../fixtures/fonts/owned.ttf");
fn request() -> FontOutlinesRequest {
    FontOutlinesRequest {
        expected_sha256: mo_common::Digest::from_sha256(Sha256::digest(FONT).into()),
        face_index: 0,
        instances: vec![OutlineInstance {
            variations: vec![],
            glyph_ids: vec![1, 2],
            max_commands: 16,
            max_operations: 1024,
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
        panic!("wrong operation")
    }
    fn outline_batch(&mut self, _: &[u8], frame: &[u32]) -> Result<Vec<u32>, TextError> {
        self.calls += 1;
        if let Some(raw) = self.raw.take() {
            return Ok(raw);
        }
        let requests = decode_outline_requests(frame)?;
        let mut raw = vec![0, requests.len() as u32];
        for r in requests {
            let ids = &r[8 + r[3] as usize * 2..];
            raw.extend([
                6 + ids.len() as u32 * 3,
                OUTLINES_MAGIC,
                1,
                1000,
                64000,
                ids.len() as u32,
                0,
            ]);
            for (i, id) in ids.iter().enumerate() {
                raw.extend([*id, u32::from(i == 0), 0]);
            }
        }
        Ok(raw)
    }
    fn invalidate(&mut self) {
        self.invalid = true;
    }
}
#[test]
fn empty_outline_is_distinct_from_unavailable_and_batch_preserves_duplicates() {
    let mut q = request();
    q.instances.push(q.instances[0].clone());
    q.instances[1].glyph_ids = vec![2, 2];
    let mut b = Backend::default();
    let r = extract(&q, FONT, &mut b, &|| false).unwrap();
    assert_eq!(b.calls, 1);
    assert_eq!(r.instances[0].glyphs[0].path, Some(vec![]));
    assert_eq!(r.instances[0].glyphs[1].path, None);
    assert_eq!(r.instances[1].glyphs.len(), 2);
}
#[test]
fn late_invalid_glyph_axis_or_aggregate_budget_never_calls_component() {
    for variant in 0..5 {
        let mut q = request();
        q.instances.push(q.instances[0].clone());
        match variant {
            0 => q.instances[1].glyph_ids = vec![7],
            1 => q.instances[1].variations.push(ShapeVariation {
                tag: "xxxx".into(),
                value_16_16: 0,
            }),
            2 => q.instances[1].max_operations = 0,
            3 => q.instances[1].max_commands = 262144,
            _ => q.instances = vec![q.instances[0].clone(); 65],
        };
        let mut b = Backend::default();
        assert!(extract(&q, FONT, &mut b, &|| false).is_err());
        assert_eq!(b.calls, 0);
        assert!(!b.invalid);
    }
}
fn path_reply(records: &[[u32; 7]]) -> Vec<u32> {
    let mut r = vec![
        0,
        1,
        9 + records.len() as u32 * 7,
        OUTLINES_MAGIC,
        1,
        1000,
        64000,
        1,
        0,
        2,
        1,
        records.len() as u32,
    ];
    for v in records {
        r.extend(v);
    }
    r
}
#[test]
fn curves_are_typed_and_every_contour_is_closed() {
    let records = [
        [1, 0, 0, 0, 0, 0, 0],
        [3, 640, 1280, 1920, 0, 0, 0],
        [4, 1920, 640, 1280, 640, 0, 0],
        [5, 0, 0, 0, 0, 0, 0],
    ];
    let mut q = request();
    q.instances[0].glyph_ids = vec![2];
    let mut b = Backend {
        raw: Some(path_reply(&records)),
        ..Default::default()
    };
    let r = extract(&q, FONT, &mut b, &|| false).unwrap();
    assert!(matches!(
        r.instances[0].glyphs[0].path.as_ref().unwrap()[1],
        OutlineCommand::Quadratic { .. }
    ));
    assert!(!b.invalid);
}
#[test]
fn malformed_paths_and_frames_invalidate_the_whole_instance() {
    let mut q = request();
    q.instances[0].glyph_ids = vec![2];
    let mut bad = vec![vec![], vec![0], vec![0, 0], vec![7, 0], vec![2, 99]];
    for commands in [
        vec![[2, 0, 0, 0, 0, 0, 0]],
        vec![[1, 0, 0, 0, 0, 0, 0]],
        vec![[1, 0, 0, 1, 0, 0, 0], [5, 0, 0, 0, 0, 0, 0]],
        vec![
            [1, 0, 0, 0, 0, 0, 0],
            [1, 0, 0, 0, 0, 0, 0],
            [5, 0, 0, 0, 0, 0, 0],
        ],
        vec![[6, 0, 0, 0, 0, 0, 0]],
    ] {
        bad.push(path_reply(&commands));
    }
    let valid = path_reply(&[]);
    for (i, v) in [(3, 0), (5, 999), (6, 1), (7, 2), (8, 1), (9, 1), (10, 2)] {
        let mut r = valid.clone();
        r[i] = v;
        bad.push(r);
    }
    let mut trailing = valid.clone();
    trailing.push(0);
    bad.push(trailing);
    for raw in bad {
        let mut b = Backend {
            raw: Some(raw),
            ..Default::default()
        };
        assert!(matches!(
            extract(&q, FONT, &mut b, &|| false),
            Err(TextError::BackendInvalid(_))
        ));
        assert!(b.invalid);
    }
}
#[test]
fn fatal_errors_and_cancellation_discard_paths_but_budget_is_recoverable() {
    for status in [1, 2, 3, 4, 5, 6] {
        let mut b = Backend {
            raw: Some(vec![status, 0]),
            ..Default::default()
        };
        assert!(matches!(
            extract(&request(), FONT, &mut b, &|| false),
            Err(TextError::BackendFailure { .. })
        ));
        assert_eq!(b.invalid, status == 2 || status == 6);
    }
    let q = request();
    let font = VerifiedFont::load(&q.expected_sha256, 0, FONT, FontLimits::default(), &|| {
        false
    })
    .unwrap();
    let count = Cell::new(0);
    let mut b = Backend::default();
    let r = extract_verified(&q, &font, &mut b, &|| {
        count.set(count.get() + 1);
        count.get() >= 4
    });
    assert!(matches!(r, Err(TextError::Cancelled)));
    assert_eq!(b.calls, 1);
    assert!(b.invalid);
}
#[test]
fn no_instances_still_verifies_font_without_backend_call() {
    let mut q = request();
    q.instances.clear();
    let mut b = Backend::default();
    assert!(
        extract(&q, FONT, &mut b, &|| false)
            .unwrap()
            .instances
            .is_empty()
    );
    assert_eq!(b.calls, 0);
    q.face_index = 4;
    assert!(extract(&q, FONT, &mut b, &|| false).is_err());
}
