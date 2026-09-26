use super::*;
use mo_common::Digest;
use mo_font::FontRequest;
use sha2::{Digest as _, Sha256};
const FONT: &[u8] = include_bytes!("../../../fixtures/fonts/owned.ttf");
struct Reply {
    words: Vec<u32>,
    calls: usize,
    invalid: bool,
}
impl TextBackend for Reply {
    fn shape_batch(&mut self, _: &[u8], frame: &[u32]) -> Result<Vec<u32>, TextError> {
        assert_eq!(backend::decode_requests(frame)?.len(), 1);
        self.calls += 1;
        Ok(self.words.clone())
    }
    fn invalidate(&mut self) {
        self.invalid = true;
    }
}
fn request() -> ShapeRequest {
    ShapeRequest {
        expected_sha256: Digest::from_sha256(Sha256::digest(FONT).into()),
        face_index: 0,
        text: "A😀".into(),
        runs: vec![ShapeRun {
            start: 0,
            end: 2,
            direction: Direction::LeftToRight,
            script: "Latn".into(),
            language: "en".into(),
            cluster_level: ClusterLevel::MonotoneGraphemes,
            flags: ShapeFlags {
                beginning_of_text: true,
                end_of_text: true,
                ignorables: Ignorables::Default,
                suppress_dotted_circle: false,
                unsafe_to_concat: true,
                safe_to_insert_tatweel: false,
            },
            features: vec![],
            variations: vec![],
            max_glyphs: 64,
        }],
    }
}
fn reply() -> Reply {
    Reply {
        words: vec![
            0,
            1,
            15,
            backend::COMPONENT_MAGIC,
            1,
            1000,
            64000,
            1,
            67,
            0,
            0,
            1,
            1,
            3,
            32000,
            0,
            (-20i32) as u32,
            0,
        ],
        calls: 0,
        invalid: false,
    }
}
fn call(request: &ShapeRequest, backend: &mut Reply) -> Result<ShapedText, TextError> {
    shape(
        request,
        FONT,
        backend,
        TextLimits::default(),
        FontLimits::default(),
        &|| false,
    )
}
#[test]
fn verified_shaping_requires_matching_identity_and_face_without_reloading() {
    let mut request = request();
    let font = VerifiedFont::load(
        &request.expected_sha256,
        0,
        FONT,
        FontLimits::default(),
        &|| false,
    )
    .unwrap();
    let mut backend = reply();
    for _ in 0..3 {
        let output = shape_verified(
            &request,
            &font,
            &mut backend,
            TextLimits::default(),
            &|| false,
        )
        .unwrap();
        assert_eq!(output.font_sha256, request.expected_sha256);
    }
    assert_eq!(backend.calls, 3);
    request.face_index = 1;
    assert!(matches!(
        shape_verified(
            &request,
            &font,
            &mut backend,
            TextLimits::default(),
            &|| false
        ),
        Err(TextError::Font(FontError::ResourceConflict))
    ));
    request.face_index = 0;
    request.expected_sha256 = Digest::from_sha256([0; 32]);
    assert!(matches!(
        shape_verified(
            &request,
            &font,
            &mut backend,
            TextLimits::default(),
            &|| false
        ),
        Err(TextError::Font(FontError::ResourceConflict))
    ));
    assert_eq!(backend.calls, 3);
}
#[test]
fn explicit_scalar_coordinates_and_signed_positions() {
    let result = call(&request(), &mut reply()).unwrap();
    let g = &result.runs[0].glyphs[0];
    assert_eq!(g.cluster, 1);
    assert_eq!(g.x_offset, -20);
    assert!(g.unsafe_to_break && g.unsafe_to_concat);
    assert_eq!(result.position_units_per_em, 64000);
}
#[test]
fn every_truncated_reply_and_trailing_word_invalidates() {
    let valid = reply().words;
    for n in 0..valid.len() {
        let mut r = reply();
        r.words.truncate(n);
        assert!(matches!(
            call(&request(), &mut r),
            Err(TextError::BackendInvalid(_))
        ));
        assert!(r.invalid);
    }
    let mut r = reply();
    r.words.push(0);
    assert!(call(&request(), &mut r).is_err());
    assert!(r.invalid);
}
#[test]
fn output_headers_and_glyph_references_are_checked() {
    for (index, value) in [
        (0, 77),
        (1, 2),
        (3, 0),
        (4, 2),
        (5, 2048),
        (6, 1),
        (7, 99),
        (8, 0),
        (9, 3),
        (10, 1),
        (11, 9999),
        (12, 2),
        (13, 8),
    ] {
        let mut r = reply();
        r.words[index] = value;
        assert!(
            matches!(call(&request(), &mut r), Err(TextError::BackendInvalid(_))),
            "index {index}"
        );
        assert!(r.invalid);
    }
}
#[test]
fn missing_glyphs_remain_explicit() {
    let mut r = reply();
    r.words[11] = 0;
    let result = call(&request(), &mut r).unwrap();
    assert_eq!(result.runs[0].missing_glyph_clusters, vec![1]);
}
#[test]
fn failed_batch_has_no_partial_result_and_memory_failure_invalidates() {
    for status in 1..=6 {
        let mut r = reply();
        r.words = vec![status, 0];
        assert!(matches!(
            call(&request(), &mut r),
            Err(TextError::BackendFailure { .. })
        ));
        assert_eq!(r.invalid, status == 2 || status == 6);
    }
}
#[test]
fn invalid_request_does_not_invoke_component() {
    let mut variants = vec![];
    let mut r = request();
    r.runs[0].end = 3;
    variants.push(r);
    let mut r = request();
    r.runs[0].features.push(ShapeFeature {
        tag: "liga".into(),
        value: 0,
        start: 2,
        end: Some(1),
    });
    variants.push(r);
    let mut r = request();
    r.runs[0].script = "a  b".into();
    variants.push(r);
    let mut r = request();
    r.runs[0].language = "en/US".into();
    variants.push(r);
    let mut r = request();
    r.runs[0].variations.push(ShapeVariation {
        tag: "zzzz".into(),
        value_16_16: 100 << 16,
    });
    variants.push(r);
    for r in variants {
        let mut backend = reply();
        assert!(call(&r, &mut backend).is_err());
        assert_eq!(backend.calls, 0);
    }
}
#[test]
fn identity_limits_and_cancellation_precede_backend() {
    let mut r = request();
    r.expected_sha256 = Digest::from_sha256([0; 32]);
    let mut backend = reply();
    assert!(matches!(
        call(&r, &mut backend),
        Err(TextError::Font(FontError::ResourceConflict))
    ));
    assert_eq!(backend.calls, 0);
    assert!(matches!(
        shape(
            &request(),
            FONT,
            &mut backend,
            TextLimits {
                max_context_scalars: 1,
                ..TextLimits::default()
            },
            FontLimits::default(),
            &|| false
        ),
        Err(TextError::Limit(_))
    ));
    assert!(matches!(
        shape(
            &request(),
            FONT,
            &mut backend,
            TextLimits::default(),
            FontLimits::default(),
            &|| true
        ),
        Err(TextError::Cancelled)
    ));
    assert_eq!(backend.calls, 0);
}
#[test]
fn internal_frame_cannot_smuggle_lengths_or_trailing_words() {
    let font = mo_font::inspect(
        &FontRequest {
            expected_sha256: request().expected_sha256,
            face_index: 0,
            characters: vec![],
        },
        FONT,
        FontLimits::default(),
        &|| false,
    )
    .unwrap();
    let frame = prepare::encode(&request(), &font, TextLimits::default(), &|| false)
        .unwrap()
        .frame;
    for n in 0..frame.len() {
        assert!(backend::decode_requests(&frame[..n]).is_err());
    }
    let mut extra = frame;
    extra.push(0);
    assert!(backend::decode_requests(&extra).is_err());
}
