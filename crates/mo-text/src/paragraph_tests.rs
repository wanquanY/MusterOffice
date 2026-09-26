use super::*;
use mo_common::{ByteLength, Digest};
use sha2::{Digest as _, Sha256};
const FONT: &[u8] = include_bytes!("../../../fixtures/fonts/owned.ttf");
struct Never;
impl backend::TextBackend for Never {
    fn shape_batch(&mut self, _: &[u8], _: &[u32]) -> Result<Vec<u32>, TextError> {
        panic!("invalid or control-only request must not call component")
    }
    fn invalidate(&mut self) {
        panic!("unused component must not be invalidated")
    }
}
fn request(text: &str) -> ParagraphShapeRequest {
    ParagraphShapeRequest {
        text: text.into(),
        direction: ParagraphDirection::LeftToRight,
        spans: if text.is_empty() {
            vec![]
        } else {
            vec![StyleSpan {
                end: text.chars().count() as u32,
                style: 0,
            }]
        },
        styles: vec![ParagraphTextStyle {
            language: "en".into(),
            features: vec![],
            candidates: vec![FontCandidate {
                font: 0,
                variations: vec![],
            }],
            suppress_dotted_circle: false,
            max_glyphs: 32,
        }],
        fonts: vec![CascadeFont {
            expected_sha256: Digest::from_sha256(Sha256::digest(FONT).into()),
            face_index: 0,
            offset: ByteLength::new(0),
            byte_length: ByteLength::new(FONT.len() as u64),
        }],
    }
}
#[test]
fn unused_and_control_only_styles_are_validated_before_any_shaping() {
    for text in ["A", "\t", ""] {
        let mut q = request(text);
        q.styles.push(ParagraphTextStyle {
            language: "bad language".into(),
            ..q.styles[0].clone()
        });
        assert!(matches!(
            shape_paragraph(&q, FONT, &mut Never, &|| false),
            Err(TextError::Invalid(_))
        ));
        q.styles[1].language = "en".into();
        q.styles[1].candidates[0].variations = vec![ShapeVariation {
            tag: "xxxx".into(),
            value_16_16: 400 * 65536,
        }];
        assert!(matches!(
            shape_paragraph(&q, FONT, &mut Never, &|| false),
            Err(TextError::Invalid(_))
        ));
    }
    let r = shape_paragraph(&request("\t\r\n"), FONT, &mut Never, &|| false).unwrap();
    assert!(r.fallback.items.is_empty());
    assert!(r.shaped_item_indices.is_empty());
    assert_eq!(r.itemization.items.len(), 2);
}
#[test]
fn paragraph_size_identity_and_cancellation_fail_before_component() {
    let mut q = request("A");
    q.styles[0].candidates[0].font = 1;
    assert!(matches!(
        shape_paragraph(&q, FONT, &mut Never, &|| false),
        Err(TextError::Invalid(_))
    ));
    let mut q = request("A");
    q.styles[0].candidates[0].variations = vec![
        ShapeVariation {
            tag: "xxxx".into(),
            value_16_16: 0
        };
        65
    ];
    assert!(matches!(
        shape_paragraph(&q, FONT, &mut Never, &|| false),
        Err(TextError::Limit(_))
    ));
    let mut q = request("A");
    q.fonts[0].expected_sha256 = Digest::from_sha256([0; 32]);
    assert!(matches!(
        shape_paragraph(&q, FONT, &mut Never, &|| false),
        Err(TextError::Font(FontError::ResourceConflict))
    ));
    let q = request("\t");
    let checks = std::cell::Cell::new(0);
    shape_paragraph(&q, FONT, &mut Never, &|| {
        checks.set(checks.get() + 1);
        false
    })
    .unwrap();
    for stop in 1..=checks.get() {
        let n = std::cell::Cell::new(0);
        assert!(matches!(
            shape_paragraph(&q, FONT, &mut Never, &|| {
                n.set(n.get() + 1);
                n.get() == stop
            }),
            Err(TextError::Cancelled | TextError::Font(FontError::Cancelled))
        ));
    }
}
