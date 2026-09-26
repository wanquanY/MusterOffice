use super::*;
use mo_common::{ByteLength, Digest};
use sha2::{Digest as _, Sha256};
use std::cell::Cell;
const FONT: &[u8] = include_bytes!("../../../fixtures/fonts/owned.ttf");
fn request(text: &str, ends: Vec<u32>) -> LineShapeRequest {
    LineShapeRequest {
        paragraph: ParagraphShapeRequest {
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
                language: "und".into(),
                features: vec![],
                candidates: vec![FontCandidate {
                    font: 0,
                    variations: vec![],
                }],
                suppress_dotted_circle: false,
                max_glyphs: 100,
            }],
            fonts: vec![CascadeFont {
                expected_sha256: Digest::from_sha256(Sha256::digest(FONT).into()),
                face_index: 0,
                offset: ByteLength::new(0),
                byte_length: ByteLength::new(FONT.len() as u64),
            }],
        },
        line_ends: ends,
    }
}
#[derive(Default)]
struct Backend {
    requests: Vec<Vec<u32>>,
    invalid: bool,
    fail_at: Option<usize>,
}
impl backend::TextBackend for Backend {
    fn shape_batch(&mut self, _: &[u8], frame: &[u32]) -> Result<Vec<u32>, TextError> {
        let runs = backend::decode_requests(frame)?;
        let mut out = vec![0, runs.len() as u32];
        for run in runs {
            let r = run.words;
            self.requests.push(r.to_vec());
            if self.fail_at == Some(self.requests.len()) {
                return Ok(vec![2, 0]);
            }
            let n = r[8];
            out.extend([
                8 + n * 7,
                backend::COMPONENT_MAGIC,
                1,
                1000,
                64000,
                n,
                r[4],
                r[5],
                0,
            ]);
            let mut clusters: Vec<_> = (r[7]..r[7] + r[8]).collect();
            if r[2] == 5 {
                clusters.reverse();
            }
            for c in clusters {
                out.extend([2, c, 0, 38400, 0, 0, 0]);
            }
        }
        Ok(out)
    }
    fn invalidate(&mut self) {
        self.invalid = true;
    }
}
#[test]
fn line_context_clips_features_and_rebases_all_public_coordinates() {
    let mut q = request("Aα😀A", vec![1, 3, 4]);
    q.paragraph.styles[0].features = vec![
        ShapeFeature {
            tag: "liga".into(),
            value: 0,
            start: 1,
            end: Some(3),
        },
        ShapeFeature {
            tag: "kern".into(),
            value: 0,
            start: 3,
            end: None,
        },
    ];
    let mut b = Backend::default();
    let r = shape_lines(&q, FONT, &mut b, &|| false).unwrap();
    assert_eq!(
        r.lines
            .iter()
            .map(|l| l.end.utf8_offset)
            .collect::<Vec<_>>(),
        [1, 7, 8]
    );
    assert_eq!(
        r.lines
            .iter()
            .map(|l| l.end.utf16_offset)
            .collect::<Vec<_>>(),
        [1, 4, 5]
    );
    assert_eq!(r.fallback.verified_faces, 1);
    for f in r.fallback.items.iter().flat_map(|i| &i.fragments) {
        let FontFragment::Selected {
            start, end, shaped, ..
        } = f
        else {
            panic!("owned coverage")
        };
        let run = &shaped.runs[0];
        assert_eq!((run.start, run.end), (*start, *end));
        assert!(
            run.glyphs
                .iter()
                .all(|g| (*start..*end).contains(&g.cluster))
        );
    }
    assert!(b.requests.iter().all(|r| r[6] <= 2));
    let first = &b.requests[0];
    assert_eq!(first[9], 0);
    assert_eq!(first[4] & 3, 3);
    let last = b.requests.last().unwrap();
    assert_eq!(&last[13..14], &[65]);
    assert_eq!(&last[14..18], &[u32::from_be_bytes(*b"kern"), 0, 0, 1]);
}
#[test]
fn bidi_resolution_stays_in_paragraph_while_line_trailing_levels_reset() {
    let q = request("α אב  12", vec![5, 8]);
    let mut b = Backend::default();
    let r = shape_lines(&q, FONT, &mut b, &|| false).unwrap();
    assert_eq!(r.bidi.resolved_levels[4], Some(1));
    assert_eq!(r.bidi.lines[0].levels[4], Some(0));
    assert!(
        r.items
            .iter()
            .any(|i| i.start.scalar_offset == 4 && i.end.scalar_offset == 5 && i.level == 0)
    );
    assert!(r.items.iter().all(|i| {
        r.lines.iter().any(|l| {
            l.start.scalar_offset <= i.start.scalar_offset
                && i.end.scalar_offset <= l.end.scalar_offset
        })
    }));
}
#[test]
fn every_line_and_style_is_validated_before_shaping() {
    let mut variants = vec![
        request("Aα", vec![]),
        request("Aα", vec![1]),
        request("Aα", vec![1, 1, 2]),
        request("A\u{301}", vec![1, 2]),
        request("", vec![1]),
    ];
    let mut q = request("AA", vec![1, 2]);
    q.paragraph.styles[0].candidates[0]
        .variations
        .push(ShapeVariation {
            tag: "xxxx".into(),
            value_16_16: 0,
        });
    variants.push(q);
    let mut q = request("AA", vec![1, 2]);
    q.paragraph.styles.push(q.paragraph.styles[0].clone());
    q.paragraph.styles[1].language = "bad value".into();
    variants.push(q);
    for q in variants {
        let mut b = Backend::default();
        assert!(shape_lines(&q, FONT, &mut b, &|| false).is_err());
        assert!(b.requests.is_empty());
        assert!(!b.invalid);
    }
}
#[test]
fn empty_and_control_only_lines_preserve_source_without_shaping() {
    for q in [
        request("", vec![0]),
        request("\t\u{2028}\u{2029}", vec![2, 3]),
    ] {
        let mut b = Backend::default();
        let r = shape_lines(&q, FONT, &mut b, &|| false).unwrap();
        assert_eq!(r.lines.len(), q.line_ends.len());
        assert!(r.fallback.items.is_empty());
        assert!(b.requests.is_empty());
    }
}
#[test]
fn later_component_failure_and_any_cancellation_publish_no_partial_lines() {
    let q = request("AAAA", vec![2, 4]);
    let mut b = Backend {
        fail_at: Some(2),
        ..Default::default()
    };
    assert!(matches!(
        shape_lines(&q, FONT, &mut b, &|| false),
        Err(TextError::BackendFailure { status: 2, .. })
    ));
    assert!(b.invalid);
    let total = Cell::new(0);
    shape_lines(&q, FONT, &mut Backend::default(), &|| {
        total.set(total.get() + 1);
        false
    })
    .unwrap();
    for stop in 1..=total.get() {
        let at = Cell::new(0);
        let mut b = Backend::default();
        assert!(
            shape_lines(&q, FONT, &mut b, &|| {
                at.set(at.get() + 1);
                at.get() == stop
            })
            .is_err()
        );
    }
}
