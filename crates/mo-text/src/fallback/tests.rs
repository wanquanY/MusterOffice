use super::*;
use mo_common::{ByteLength, Digest};
use sha2::{Digest as _, Sha256};
const FONT: &[u8] = include_bytes!("../../../../fixtures/fonts/owned.ttf");
type Glyph = (u32, u32, u32, i32);
struct Reply {
    ranges: Vec<(u32, u32)>,
    glyphs: Vec<Vec<Glyph>>,
    failure: bool,
}
fn reply(ranges: &[(u32, u32)], glyphs: Vec<Vec<Glyph>>) -> Reply {
    Reply {
        ranges: ranges.to_vec(),
        glyphs,
        failure: false,
    }
}
fn glyph(id: u32, cluster: u32, flags: u32) -> Glyph {
    (id, cluster, flags, 32000)
}
struct Backend {
    queue: std::collections::VecDeque<Reply>,
    calls: usize,
    invalid: bool,
}
impl backend::TextBackend for Backend {
    fn shape_batch(&mut self, _: &[u8], frame: &[u32]) -> Result<Vec<u32>, TextError> {
        self.calls += 1;
        let next = self.queue.pop_front().expect("unexpected component call");
        if next.failure {
            return Ok(vec![2, 0]);
        }
        let runs = backend::decode_requests(frame)?;
        assert_eq!(
            runs.iter()
                .map(|r| (r.words[7], r.words[7] + r.words[8]))
                .collect::<Vec<_>>(),
            next.ranges
        );
        let mut out = vec![0, runs.len() as u32];
        for (r, glyphs) in runs.iter().zip(next.glyphs) {
            out.push(8 + glyphs.len() as u32 * 7);
            out.extend([
                backend::COMPONENT_MAGIC,
                1,
                1000,
                64000,
                glyphs.len() as u32,
                r.words[4],
                r.words[5],
                0,
            ]);
            for (id, cluster, flags, advance) in glyphs {
                out.extend([id, cluster, flags, advance as u32, 0, 0, 0]);
            }
        }
        Ok(out)
    }
    fn invalidate(&mut self) {
        self.invalid = true;
    }
}
fn backend(queue: Vec<Reply>) -> Backend {
    Backend {
        queue: queue.into(),
        calls: 0,
        invalid: false,
    }
}
fn request(text: &str, count: usize) -> CascadeRequest {
    let source = CascadeFont {
        expected_sha256: Digest::from_sha256(Sha256::digest(FONT).into()),
        face_index: 0,
        offset: ByteLength::new(0),
        byte_length: ByteLength::new(FONT.len() as u64),
    };
    CascadeRequest {
        text: text.into(),
        fonts: vec![source; count],
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
            max_glyphs: 128,
            candidates: (0..count)
                .map(|font| FontCandidate {
                    font: font as u32,
                    variations: vec![],
                })
                .collect(),
        }],
    }
}
fn mixed() -> Vec<Reply> {
    vec![
        reply(
            &[(0, 3)],
            vec![vec![glyph(2, 0, 2), glyph(0, 1, 2), glyph(2, 2, 2)]],
        ),
        reply(
            &[(0, 3)],
            vec![vec![glyph(0, 0, 2), glyph(3, 1, 2), glyph(0, 2, 2)]],
        ),
        reply(
            &[(0, 1), (2, 3)],
            vec![vec![(2, 0, 2, 31000)], vec![(2, 2, 2, 33000)]],
        ),
        reply(&[(1, 2)], vec![vec![(3, 1, 2, 42000)]]),
    ]
}
fn execute(q: &CascadeRequest, b: &mut Backend) -> FallbackResult {
    shape_fallback(q, FONT, b, FallbackLimits::default(), &|| false).unwrap()
}
#[test]
fn mixed_fonts_reshape_in_batches_instead_of_splicing_unsafe_concat_glyphs() {
    let mut b = backend(mixed());
    let r = execute(&request("A😀A", 2), &mut b);
    assert_eq!(r.verified_faces, 1);
    assert_eq!((r.shaping_runs, r.component_calls), (5, 4));
    assert_eq!(r.context_scalars, 15);
    let fragments = &r.items[0].fragments;
    assert_eq!(fragments.len(), 3);
    for (i, expected) in [31000, 42000, 33000].iter().enumerate() {
        let FontFragment::Selected {
            start,
            end,
            candidate,
            shaped,
            ..
        } = &fragments[i]
        else {
            panic!("missing fragment")
        };
        assert_eq!((*start, *end), (i as u32, i as u32 + 1));
        assert_eq!(*candidate, if i == 1 { 1 } else { 0 });
        assert_eq!(shaped.runs[0].glyphs[0].x_advance, *expected);
    }
    assert!(r.items[0].protected_boundaries.is_empty());
    assert!(b.queue.is_empty());
}
#[test]
fn unsafe_break_dependencies_in_later_candidate_can_require_one_font() {
    let mut b = backend(vec![
        reply(&[(0, 2)], vec![vec![glyph(2, 0, 0), glyph(0, 1, 0)]]),
        reply(&[(0, 2)], vec![vec![glyph(2, 0, 2), glyph(3, 1, 3)]]),
    ]);
    let r = execute(&request("AA", 2), &mut b);
    assert_eq!(r.items[0].protected_boundaries, vec![1]);
    assert_eq!(b.calls, 2);
    assert!(matches!(
        r.items[0].fragments[0],
        FontFragment::Selected {
            candidate: 1,
            start: 0,
            end: 2,
            ..
        }
    ));
}
#[test]
fn observed_ligatures_and_extended_graphemes_are_indivisible() {
    let mut b = backend(vec![
        reply(&[(0, 4)], vec![vec![glyph(2, 0, 0), glyph(0, 3, 0)]]),
        reply(
            &[(0, 4)],
            vec![vec![
                glyph(0, 0, 0),
                glyph(0, 1, 0),
                glyph(0, 2, 0),
                glyph(3, 3, 0),
            ]],
        ),
        reply(&[(0, 3)], vec![vec![glyph(2, 0, 0)]]),
        reply(&[(3, 4)], vec![vec![glyph(3, 3, 0)]]),
    ]);
    let r = execute(&request("AAA😀", 2), &mut b);
    assert_eq!(r.items[0].protected_boundaries, vec![1, 2]);
    assert_eq!(r.items[0].fragments.len(), 2);
    let mut b = backend(vec![
        reply(&[(0, 2)], vec![vec![glyph(2, 0, 0), glyph(0, 1, 0)]]),
        reply(&[(0, 2)], vec![vec![glyph(2, 0, 0), glyph(3, 1, 0)]]),
    ]);
    let r = execute(&request("A\u{301}", 2), &mut b);
    assert!(matches!(
        r.items[0].fragments[0],
        FontFragment::Selected {
            candidate: 1,
            start: 0,
            end: 2,
            ..
        }
    ));
    assert_eq!(r.items[0].fragments.len(), 1);
}
#[test]
fn unresolved_ranges_remain_explicit_and_cover_the_input() {
    let mut b = backend(vec![
        reply(
            &[(0, 3)],
            vec![vec![glyph(2, 0, 0), glyph(0, 1, 0), glyph(2, 2, 0)]],
        ),
        reply(
            &[(0, 1), (2, 3)],
            vec![vec![glyph(2, 0, 0)], vec![glyph(2, 2, 0)]],
        ),
    ]);
    let r = execute(&request("A😀A", 1), &mut b);
    assert_eq!(r.items[0].fragments.len(), 3);
    assert!(matches!(
        r.items[0].fragments[1],
        FontFragment::Unresolved { start: 1, end: 2 }
    ));
    let mut b = backend(vec![reply(&[(0, 2)], vec![vec![glyph(2, 0, 0)]])]);
    let r = execute(&request("A\u{fe02}", 1), &mut b);
    assert!(matches!(
        r.items[0].fragments[0],
        FontFragment::Unresolved { start: 0, end: 2 }
    ));
    assert_eq!(r.items[0].probes[0].variation_issues.len(), 1);
}
#[test]
fn final_reshape_failure_advances_candidates_without_leaking_old_glyphs() {
    let mut queue = mixed();
    queue[2].glyphs[0][0].0 = 0;
    queue.push(reply(
        &[(0, 3)],
        vec![vec![glyph(3, 0, 0), glyph(3, 1, 0), glyph(3, 2, 0)]],
    ));
    queue.push(reply(&[(0, 1)], vec![vec![(3, 0, 0, 45000)]]));
    let mut b = backend(queue);
    let r = execute(&request("A😀A", 3), &mut b);
    assert_eq!(r.items[0].reshape_rejections.len(), 1);
    let FontFragment::Selected {
        candidate, shaped, ..
    } = &r.items[0].fragments[0]
    else {
        panic!("unresolved")
    };
    assert_eq!(*candidate, 2);
    assert_eq!(shaped.runs[0].glyphs[0].x_advance, 45000);
    assert!(b.queue.is_empty());
}
#[test]
fn rtl_clusters_keep_logical_fragment_ranges_and_visual_glyph_order() {
    let mut q = request("A😀A", 2);
    q.items[0].direction = Direction::RightToLeft;
    let mut queue = mixed();
    for r in &mut queue {
        for g in &mut r.glyphs {
            g.reverse();
        }
    }
    let r = execute(&q, &mut backend(queue));
    assert_eq!(r.items[0].fragments.len(), 3);
    for (i, f) in r.items[0].fragments.iter().enumerate() {
        let FontFragment::Selected { start, shaped, .. } = f else {
            panic!("missing")
        };
        assert_eq!(*start, i as u32);
        assert_eq!(shaped.runs[0].direction, Direction::RightToLeft);
    }
}
#[test]
fn failures_and_limits_do_not_publish_partial_fragments() {
    let mut queue = mixed();
    queue[2].failure = true;
    let mut b = backend(queue);
    assert!(matches!(
        shape_fallback(
            &request("A😀A", 2),
            FONT,
            &mut b,
            FallbackLimits::default(),
            &|| false
        ),
        Err(TextError::BackendFailure { status: 2, .. })
    ));
    assert!(b.invalid);
    assert_eq!(b.calls, 3);
    for limits in [
        FallbackLimits {
            max_fragments: 2,
            ..FallbackLimits::default()
        },
        FallbackLimits {
            cascade: CascadeLimits {
                max_attempts: 2,
                ..CascadeLimits::default()
            },
            ..FallbackLimits::default()
        },
        FallbackLimits {
            cascade: CascadeLimits {
                max_selected_glyphs: 2,
                ..CascadeLimits::default()
            },
            ..FallbackLimits::default()
        },
    ] {
        assert!(matches!(
            shape_fallback(
                &request("A😀A", 2),
                FONT,
                &mut backend(mixed()),
                limits,
                &|| false
            ),
            Err(TextError::Limit(_))
        ));
    }
    let mut queue = mixed();
    queue[2].glyphs[0][0].0 = 0;
    assert!(matches!(
        shape_fallback(
            &request("A😀A", 2),
            FONT,
            &mut backend(queue),
            FallbackLimits {
                max_reshape_rejections: 0,
                ..FallbackLimits::default()
            },
            &|| false
        ),
        Err(TextError::Limit(_))
    ));
}
#[test]
fn cancellation_at_every_own_checkpoint_returns_no_result() {
    let q = request("A😀A", 2);
    let calls = std::cell::Cell::new(0);
    shape_fallback(
        &q,
        FONT,
        &mut backend(mixed()),
        FallbackLimits::default(),
        &|| {
            calls.set(calls.get() + 1);
            false
        },
    )
    .unwrap();
    for stop in 1..=calls.get() {
        let n = std::cell::Cell::new(0);
        assert!(matches!(
            shape_fallback(
                &q,
                FONT,
                &mut backend(mixed()),
                FallbackLimits::default(),
                &|| {
                    n.set(n.get() + 1);
                    n.get() == stop
                }
            ),
            Err(TextError::Cancelled | TextError::Font(FontError::Cancelled))
        ));
    }
}
