use super::*;
use crate::{
    backend::*,
    flow::OverflowPolicy,
    geometry::test_support::{Backend as GeometryBackend, FONT},
};
use std::cell::Cell;
pub(super) fn q(text: &str) -> ParagraphInteractionRequest {
    let mut g = crate::geometry::test_support::request(text);
    g.shaping.paragraph.styles[0].max_glyphs = 4096;
    ParagraphInteractionRequest {
        layout: crate::flow::ParagraphLayoutRequest {
            paragraph: g.shaping.paragraph,
            styles: g.styles,
            strut_style: 0,
            spacing: g.spacing,
            width: mo_common::Emu::new(10_000_000),
            overflow: OverflowPolicy::KeepUnbreakable,
            wrapping: Default::default(),
            hanging_punctuation: Default::default(),
        },
        queries: vec![],
    }
}
#[derive(Default)]
struct Backend {
    inner: GeometryBackend,
    carets: Vec<i32>,
    calls: usize,
    malformed: bool,
    ambiguous: bool,
}
impl TextBackend for Backend {
    fn shape_batch(&mut self, font: &[u8], frame: &[u32]) -> Result<Vec<u32>, TextError> {
        let mut out = self.inner.shape_batch(font, frame)?;
        assert_eq!(out[1], 1);
        // One cluster for all characters; optionally two advancing glyphs.
        let n = if self.ambiguous { 2 } else { 1 };
        out[2] = 8 + n * 7;
        out[7] = n;
        out.truncate(11 + n as usize * 7);
        if self.ambiguous {
            out[19] = out[12];
        }
        Ok(out)
    }
    fn measure_batch(&mut self, font: &[u8], frame: &[u32]) -> Result<Vec<u32>, TextError> {
        self.inner.measure_batch(font, frame)
    }
    fn caret_batch(&mut self, _: &[u8], frame: &[u32]) -> Result<Vec<u32>, TextError> {
        self.calls += 1;
        if self.malformed {
            return Ok(vec![0, 1]);
        }
        let inputs = decode_caret_requests(frame)?;
        assert_eq!(inputs.len(), 1);
        let r = inputs[0];
        assert_eq!(r[5], 1);
        let mut out = vec![
            0,
            1,
            8 + self.carets.len() as u32,
            CARETS_MAGIC,
            1,
            1000,
            64000,
            r[4],
            1,
            r[8 + 2 * r[3] as usize],
            self.carets.len() as u32,
        ];
        out.extend(self.carets.iter().map(|&v| v as u32));
        Ok(out)
    }
    fn invalidate(&mut self) {
        self.inner.invalidate();
    }
}
#[test]
fn font_count_and_order_incompatibilities_are_explicit_partitions() {
    for (values, reason) in [
        (vec![], PartitionReason::FontCaretsAbsent),
        (vec![100], PartitionReason::FontCaretCountMismatch),
        (vec![200, 100], PartitionReason::NonMonotoneFontCarets),
    ] {
        let mut b = Backend {
            carets: values,
            ..Default::default()
        };
        let m = paragraph_interaction(&q("AAA"), FONT, &mut b, &|| false)
            .unwrap()
            .map
            .unwrap();
        assert_eq!(b.calls, 1);
        assert_eq!(
            m.cells[0].placement,
            CaretPlacement::ClusterPartition { reason }
        );
        assert_eq!(
            m.cells[0].trailing.x,
            Fixed::emu(mo_common::Emu::new(50800))
        );
    }
    let mut b = Backend {
        ambiguous: true,
        ..Default::default()
    };
    let m = paragraph_interaction(&q("AAA"), FONT, &mut b, &|| false)
        .unwrap()
        .map
        .unwrap();
    assert_eq!(b.calls, 0);
    assert_eq!(
        m.cells[0].placement,
        CaretPlacement::ClusterPartition {
            reason: PartitionReason::AmbiguousGlyphs
        }
    );
}
#[test]
fn malformed_caret_component_discards_previously_computed_layout() {
    let mut b = Backend {
        malformed: true,
        ..Default::default()
    };
    assert!(matches!(
        paragraph_interaction(&q("AAA"), FONT, &mut b, &|| false),
        Err(TextError::BackendInvalid(_))
    ));
    assert!(b.inner.shapes > 0);
    assert_eq!(b.calls, 1);
    assert!(b.inner.invalid);
}
#[test]
fn every_cancellation_checkpoint_returns_no_partial_interaction_result() {
    let mut request = q("AAA");
    let position = |scalar_offset| TextPosition {
        scalar_offset,
        affinity: Affinity::Downstream,
    };
    request.queries = vec![
        TextQuery::Caret {
            position: position(1),
        },
        TextQuery::Hit {
            point: Point {
                x: Fixed::ZERO,
                y: Fixed::ZERO,
            },
        },
        TextQuery::Selection {
            anchor: position(3),
            focus: position(0),
        },
    ];
    let calls = Cell::new(0usize);
    let mut b = Backend {
        carets: vec![11072, 27584],
        ..Default::default()
    };
    paragraph_interaction(&request, FONT, &mut b, &|| {
        calls.set(calls.get() + 1);
        false
    })
    .unwrap();
    let count = calls.get();
    assert!(count > 100);
    for stop in 1..=count {
        calls.set(0);
        let mut b = Backend {
            carets: vec![11072, 27584],
            ..Default::default()
        };
        let r = paragraph_interaction(&request, FONT, &mut b, &|| {
            calls.set(calls.get() + 1);
            calls.get() == stop
        });
        assert!(
            matches!(
                r,
                Err(TextError::Cancelled | TextError::Font(mo_font::FontError::Cancelled))
            ),
            "checkpoint {stop}/{count}: {r:?}"
        );
    }
}
#[test]
fn prerequisites_and_selection_budgets_never_return_partial_maps_or_queries() {
    for text in ["A\tA", "A\u{ad}A", "A\u{fffc}A"] {
        let mut b = Backend::default();
        let r = paragraph_interaction(&q(text), FONT, &mut b, &|| false).unwrap();
        assert!(r.map.is_none());
        assert!(r.results.is_empty());
        assert_eq!(b.inner.shapes, 0);
        assert_eq!(b.calls, 0);
    }
    let request = q(&"A".repeat(1100));
    // The ordinary mock preserves one cluster per grapheme, avoiding font carets.
    let m = paragraph_interaction(&request, FONT, &mut GeometryBackend::default(), &|| false)
        .unwrap()
        .map
        .unwrap();
    let position = |scalar_offset| TextPosition {
        scalar_offset,
        affinity: Affinity::Downstream,
    };
    let queries = vec![
        TextQuery::Selection {
            anchor: position(0),
            focus: position(1100)
        };
        60
    ];
    assert!(matches!(
        m.query(&queries, &|| false),
        Err(TextError::Limit("interaction selection fragments"))
    ));
}

#[test]
fn multiple_shaping_clusters_inside_one_extended_grapheme_share_outer_edges() {
    // The mock emits separate glyph clusters for every scalar, including marks.
    // RLO forces the same owned glyphs through real RTL visual ordering.
    for (text, index, leading, trailing) in [
        ("A\u{301}A", 0, 0, 304800),
        ("\u{202e}A\u{301}A\u{202c}", 1, 457200, 152400),
    ] {
        let request = q(text);
        let m = paragraph_interaction(&request, FONT, &mut GeometryBackend::default(), &|| false)
            .unwrap()
            .map
            .unwrap();
        let cell = &m.cells[index];
        assert_eq!(cell.end.scalar_offset - cell.start.scalar_offset, 2);
        assert_eq!(cell.leading.x, Fixed::emu(mo_common::Emu::new(leading)));
        assert_eq!(cell.trailing.x, Fixed::emu(mo_common::Emu::new(trailing)));
        assert_eq!(m.work.caret_calls, 0);
        assert!(
            m.caret(TextPosition {
                scalar_offset: cell.start.scalar_offset + 1,
                affinity: Affinity::Downstream
            })
            .is_err()
        );
    }
}

#[test]
fn hit_query_work_is_bounded_even_when_reply_size_is_small() {
    let mut request = q(&"A".repeat(9000));
    request.layout.paragraph.styles[0].max_glyphs = 10000;
    let m = paragraph_interaction(&request, FONT, &mut GeometryBackend::default(), &|| false)
        .unwrap()
        .map
        .unwrap();
    let queries = vec![
        TextQuery::Hit {
            point: Point {
                x: Fixed::ZERO,
                y: Fixed::ZERO
            }
        };
        64
    ];
    assert!(matches!(
        m.query(&queries, &|| false),
        Err(TextError::Limit("interaction query work"))
    ));
}
