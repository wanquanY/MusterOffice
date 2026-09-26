use super::*;
use mo_common::{ByteLength, Digest};
use sha2::{Digest as _, Sha256};
const FONT: &[u8] = include_bytes!("../../../../fixtures/fonts/owned.ttf");
struct Backend {
    replies: Vec<Result<u32, ()>>,
    calls: usize,
    invalid: bool,
}
impl TextBackend for Backend {
    fn shape_batch(&mut self, _: &[u8], frame: &[u32]) -> Result<Vec<u32>, TextError> {
        let reply = self.replies[self.calls];
        self.calls += 1;
        let glyph = match reply {
            Ok(glyph) => glyph,
            Err(()) => return Ok(vec![2, 0]),
        };
        let requests = backend::decode_requests(frame)?;
        let r = requests[0].words;
        Ok(vec![
            0,
            1,
            15,
            backend::COMPONENT_MAGIC,
            1,
            1000,
            64000,
            1,
            r[4],
            r[5],
            0,
            glyph,
            r[7],
            0,
            32000,
            0,
            0,
            0,
        ])
    }
    fn invalidate(&mut self) {
        self.invalid = true;
    }
}
fn backend(replies: Vec<Result<u32, ()>>) -> Backend {
    Backend {
        replies,
        calls: 0,
        invalid: false,
    }
}
fn request(text: &str) -> CascadeRequest {
    let source = CascadeFont {
        expected_sha256: Digest::from_sha256(Sha256::digest(FONT).into()),
        face_index: 0,
        offset: ByteLength::new(0),
        byte_length: ByteLength::new(FONT.len() as u64),
    };
    CascadeRequest {
        text: text.into(),
        fonts: vec![source.clone(), source],
        items: vec![CascadeItem {
            start: 0,
            end: text.chars().count() as u32,
            direction: Direction::LeftToRight,
            script: "Latn".into(),
            language: "en".into(),
            features: vec![],
            beginning_of_text: true,
            end_of_text: true,
            suppress_dotted_circle: false,
            max_glyphs: 32,
            candidates: vec![
                FontCandidate {
                    font: 0,
                    variations: vec![],
                },
                FontCandidate {
                    font: 1,
                    variations: vec![],
                },
            ],
        }],
    }
}
#[test]
fn ordered_fallback_reuses_a_verified_face_and_preserves_failed_evidence() {
    let r = shape_cascade(
        &request("A"),
        FONT,
        &mut backend(vec![Ok(0), Ok(2)]),
        CascadeLimits::default(),
        &|| false,
    )
    .unwrap();
    assert_eq!(r.verified_faces, 1);
    assert_eq!(r.shaping_calls, 2);
    assert_eq!(r.probed_glyphs, 2);
    let ItemSelection::Selected {
        font,
        candidate,
        attempts,
        shaped,
    } = &r.items[0]
    else {
        panic!("missing selection")
    };
    assert_eq!((*font, *candidate), (1, 1));
    assert_eq!(attempts[0].missing_glyph_clusters, vec![0]);
    assert_eq!(shaped.runs[0].glyphs[0].glyph_id, 2);
}
#[test]
fn component_failure_is_fatal_and_never_a_font_fallback() {
    let mut backend = backend(vec![Err(()), Ok(2)]);
    assert!(matches!(
        shape_cascade(
            &request("A"),
            FONT,
            &mut backend,
            CascadeLimits::default(),
            &|| false
        ),
        Err(TextError::BackendFailure { status: 2, .. })
    ));
    assert_eq!(backend.calls, 1);
    assert!(backend.invalid);
}
#[test]
fn selectors_cannot_disappear_behind_a_successful_glyph_array() {
    for (text, expected) in [
        ("A\u{fe00}", true),
        ("A\u{fe01}", true),
        ("A\u{fe02}", false),
        ("\u{fe00}", false),
    ] {
        let r = shape_cascade(
            &request(text),
            FONT,
            &mut backend(vec![Ok(2), Ok(2)]),
            CascadeLimits::default(),
            &|| false,
        )
        .unwrap();
        assert_eq!(
            matches!(r.items[0], ItemSelection::Selected { .. }),
            expected
        );
        if let ItemSelection::Unresolved { attempts, .. } = &r.items[0] {
            assert!(
                attempts
                    .iter()
                    .all(|v| v.missing_glyph_clusters.is_empty() && v.variation_issues.len() == 1)
            );
        }
    }
}
#[test]
fn unused_candidates_and_resources_are_not_exempt_from_validation() {
    let mut invalids = Vec::new();
    let mut r = request("A");
    r.fonts[1].expected_sha256 = Digest::from_sha256([0; 32]);
    invalids.push(r);
    let mut r = request("A");
    r.items[0].candidates[1].variations.push(ShapeVariation {
        tag: "xxxx".into(),
        value_16_16: 0,
    });
    invalids.push(r);
    let mut r = request("A");
    r.items[0].candidates[1].font = 2;
    invalids.push(r);
    let mut r = request("A\u{301}");
    r.items[0].end = 1;
    invalids.push(r);
    let mut r = request("A");
    r.fonts[1].offset = ByteLength::new(u64::MAX);
    invalids.push(r);
    let mut r = request("A");
    r.items.push(r.items[0].clone());
    invalids.push(r);
    for r in invalids {
        let mut backend = backend(vec![Ok(2)]);
        assert!(
            shape_cascade(&r, FONT, &mut backend, CascadeLimits::default(), &|| false).is_err()
        );
        assert_eq!(backend.calls, 0);
    }
}
#[test]
fn all_cascade_budgets_and_cancellation_fail_without_partial_results() {
    let d = CascadeLimits::default();
    for limits in [
        CascadeLimits {
            max_bundle_bytes: 0,
            ..d
        },
        CascadeLimits {
            max_font_bindings: 1,
            ..d
        },
        CascadeLimits { max_items: 0, ..d },
        CascadeLimits {
            max_candidates_per_item: 1,
            ..d
        },
        CascadeLimits {
            max_attempts: 0,
            ..d
        },
        CascadeLimits {
            max_context_scalars: 0,
            ..d
        },
        CascadeLimits {
            max_probed_glyphs: 0,
            ..d
        },
        CascadeLimits {
            max_selected_glyphs: 0,
            ..d
        },
    ] {
        assert!(matches!(
            shape_cascade(
                &request("A"),
                FONT,
                &mut backend(vec![Ok(2)]),
                limits,
                &|| false
            ),
            Err(TextError::Limit(_))
        ));
    }
    let mut backend = backend(vec![Ok(2)]);
    assert!(matches!(
        shape_cascade(&request("A"), FONT, &mut backend, d, &|| true),
        Err(TextError::Cancelled)
    ));
    assert_eq!(backend.calls, 0);
    let limits = CascadeLimits {
        max_attempts: 1,
        ..d
    };
    let mut backend = super::tests::backend(vec![Ok(0), Ok(2)]);
    assert!(matches!(
        shape_cascade(&request("A"), FONT, &mut backend, limits, &|| false),
        Err(TextError::Limit(_))
    ));
    assert_eq!(backend.calls, 1);
}
