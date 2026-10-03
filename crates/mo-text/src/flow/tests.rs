use super::*;
use crate::geometry::test_support::{Backend, FONT, request};
use mo_common::Emu;
use std::cell::Cell;
pub(super) fn q(text: &str, width: i64, overflow: OverflowPolicy) -> ParagraphLayoutRequest {
    let g = request(text);
    ParagraphLayoutRequest {
        tabs: None,
        paragraph: g.shaping.paragraph,
        styles: vec![geometry::GeometryStyle {
            cluster_spacing: mo_geometry::Fixed::ZERO,
            font_size: Emu::new(1000),
            baseline_shift: Emu::ZERO.into(),
        }],
        strut_style: 0,
        spacing: g.spacing,
        width: Emu::new(width),
        overflow,
        hanging_punctuation: HangingPunctuation::None,
        wrapping: LineWrapping::Wrap,
    }
}
fn ends(r: &ParagraphLayoutResult) -> Vec<u32> {
    r.decisions.iter().map(|d| d.end.scalar_offset).collect()
}
#[test]
fn tracking_changes_candidate_fit_and_selected_line_geometry_together() {
    for (tracking, expected) in [(0, vec![2, 4]), (100, vec![1, 2, 3, 4]), (-100, vec![2, 4])] {
        let mut q = q("AAAA", 1200, OverflowPolicy::EmergencyGrapheme);
        q.styles[0].cluster_spacing = Position::emu(Emu::new(tracking));
        let r = layout_paragraph(&q, FONT, &mut Backend::default(), &|| false).unwrap();
        assert_eq!(ends(&r), expected);
        assert!(
            r.geometry
                .unwrap()
                .layout
                .unwrap()
                .lines
                .iter()
                .all(|l| l.pen_max.get() - l.pen_min.get() <= 1200)
        );
    }
}
#[test]
fn exact_first_and_continuation_widths_share_the_same_candidate_search() {
    let request = q("AAAA", 1, OverflowPolicy::EmergencyGrapheme);
    for (rest, expected) in [
        (Position::emu(Emu::new(1200)), vec![1, 3, 4]),
        (Position::from_raw((1200i128 << 32) - 1), vec![1, 2, 3, 4]),
    ] {
        let mut input = FlowInput::from(&request);
        input.widths = LineWidths {
            first: Position::emu(Emu::new(600)),
            rest,
        };
        let r = layout_flow(
            &input,
            resources::ResourceInput::Bundle(FONT),
            &mut Backend::default(),
            &|| false,
            geometry::evaluate,
        )
        .unwrap();
        assert_eq!(ends(&r), expected);
    }
    let q = q("A\u{2028}AA", 1, OverflowPolicy::EmergencyGrapheme);
    let mut input = FlowInput::from(&q);
    input.widths = LineWidths {
        first: Position::emu(Emu::new(600)),
        rest: Position::emu(Emu::new(1200)),
    };
    let r = layout_flow(
        &input,
        resources::ResourceInput::Bundle(FONT),
        &mut Backend::default(),
        &|| false,
        geometry::evaluate,
    )
    .unwrap();
    assert_eq!(ends(&r), [2, 4]);
}
#[test]
fn actual_width_selects_legal_lines_and_reuses_selected_shapes() {
    let q = q("A A A", 1500, OverflowPolicy::KeepUnbreakable);
    let mut b = Backend::default();
    let r = layout_paragraph(&q, FONT, &mut b, &|| false).unwrap();
    assert_eq!(ends(&r), [2, 4, 5]);
    assert!(r.issues.is_empty());
    assert_eq!(b.shapes, 6);
    assert_eq!(b.metrics, 1);
    assert_eq!(r.work.evaluated_candidates, 6);
    let g = r.geometry.unwrap();
    assert_eq!(g.shaping.fallback.shaping_runs, 6);
    assert_eq!(g.shaping.fallback.items.len(), 3);
    assert_eq!(g.layout.unwrap().lines.len(), 3);
    assert_eq!(q.paragraph.text, "A A A");
}
#[test]
fn overflow_and_emergency_are_explicit_without_losing_source() {
    let keep = layout_paragraph(
        &q("AAA", 1000, OverflowPolicy::KeepUnbreakable),
        FONT,
        &mut Backend::default(),
        &|| false,
    )
    .unwrap();
    assert_eq!(ends(&keep), [3]);
    assert!(keep.decisions[0].overflows);
    assert!(!keep.decisions[0].emergency);
    let emergency = layout_paragraph(
        &q("AAA", 1000, OverflowPolicy::EmergencyGrapheme),
        FONT,
        &mut Backend::default(),
        &|| false,
    )
    .unwrap();
    assert_eq!(ends(&emergency), [1, 2, 3]);
    assert_eq!(
        emergency
            .decisions
            .iter()
            .map(|d| d.emergency)
            .collect::<Vec<_>>(),
        [true, true, false]
    );
    let wide_cluster = layout_paragraph(
        &q("A\u{301}A", 1, OverflowPolicy::EmergencyGrapheme),
        FONT,
        &mut Backend::default(),
        &|| false,
    )
    .unwrap();
    assert_eq!(ends(&wide_cluster), [2, 3]);
    assert!(wide_cluster.decisions.iter().all(|d| d.overflows));
}
#[test]
fn forced_breaks_and_trailing_empty_line_have_real_strut_boxes() {
    for text in ["A\u{2028}", "A\u{b}", "A\u{c}"] {
        let r = layout_paragraph(
            &q(text, 10000, OverflowPolicy::KeepUnbreakable),
            FONT,
            &mut Backend::default(),
            &|| false,
        )
        .unwrap();
        assert_eq!(ends(&r), [2, 2]);
        let g = r.geometry.unwrap();
        assert!(g.layout.as_ref().unwrap().lines[1].glyphs.is_empty());
        assert_eq!(g.layout.unwrap().height.get(), 2050);
    }
    for (text, expected) in [
        ("A\u{2028}A", vec![2, 3]),
        ("A\r\n", vec![3]),
        ("", vec![0]),
    ] {
        let r = layout_paragraph(
            &q(text, 10000, OverflowPolicy::KeepUnbreakable),
            FONT,
            &mut Backend::default(),
            &|| false,
        )
        .unwrap();
        assert_eq!(ends(&r), expected);
    }
}
#[test]
fn grapheme_tailoring_and_unsupported_prerequisites_are_visible() {
    let r = layout_paragraph(
        &q(" \u{301}A", 10000, OverflowPolicy::KeepUnbreakable),
        FONT,
        &mut Backend::default(),
        &|| false,
    )
    .unwrap();
    assert_eq!(
        r.suppressed_grapheme_breaks
            .iter()
            .map(|b| b.scalar_offset)
            .collect::<Vec<_>>(),
        [1]
    );
    for text in ["A\tA", "A\u{ad}A", "A\u{fffc}A"] {
        let mut b = Backend::default();
        let r = layout_paragraph(
            &q(text, 1000, OverflowPolicy::KeepUnbreakable),
            FONT,
            &mut b,
            &|| false,
        )
        .unwrap();
        assert_eq!(r.issues.len(), 1);
        assert!(r.decisions.is_empty());
        assert!(r.geometry.is_none());
        assert_eq!((b.shapes, b.metrics), (0, 0));
        assert_eq!(r.work.verified_faces, 1);
    }
}
struct NonMonotonic {
    inner: Backend,
    lengths: Vec<u32>,
}
impl backend::TextBackend for NonMonotonic {
    fn shape_batch(&mut self, font: &[u8], frame: &[u32]) -> Result<Vec<u32>, TextError> {
        let runs = backend::decode_requests(frame)?;
        assert_eq!(runs.len(), 1);
        let n = runs[0].words[8];
        self.lengths.push(n);
        let mut out = self.inner.shape_batch(font, frame)?;
        let width = match n {
            5 => 500,
            4 => 100,
            2 => 300,
            _ => 100,
        };
        for i in 0..n as usize {
            out[2 + 1 + 8 + i * 7 + 3] = if i == 0 { width } else { 0 };
        }
        Ok(out)
    }
    fn measure_batch(&mut self, font: &[u8], frame: &[u32]) -> Result<Vec<u32>, TextError> {
        self.inner.measure_batch(font, frame)
    }
    fn invalidate(&mut self) {
        self.inner.invalidate()
    }
}
#[test]
fn longer_context_can_fit_after_a_shorter_context_overflows() {
    let mut q = q("A A A", 200, OverflowPolicy::KeepUnbreakable);
    q.styles[0].font_size = Emu::new(64000);
    let mut b = NonMonotonic {
        inner: Backend::default(),
        lengths: vec![],
    };
    let r = layout_paragraph(&q, FONT, &mut b, &|| false).unwrap();
    assert_eq!(ends(&r), [4, 5]);
    assert_eq!(b.lengths, [5, 4, 1]);
    assert!(r.decisions.iter().all(|d| !d.overflows));
}
#[test]
fn fitting_uses_internal_precision_not_rounded_output_advance() {
    let mut q = q("AA", 1, OverflowPolicy::EmergencyGrapheme);
    q.styles[0].font_size = Emu::new(1);
    let r = layout_paragraph(&q, FONT, &mut Backend::default(), &|| false).unwrap();
    assert_eq!(ends(&r), [1, 2]);
    // Whole-line 1.2 EMU rounds to 1 EMU, but it still must not fit width 1.
    let g = request("AA");
    let g = geometry::LineGeometryRequest {
        tabs: None,
        styles: q.styles,
        ..g
    };
    let r = geometry::layout_lines(&g, FONT, &mut Backend::default(), &|| false).unwrap();
    assert_eq!(r.layout.unwrap().lines[0].advance.get(), 1);
}
#[test]
fn all_styles_validate_and_work_budgets_cancel_atomically() {
    let mut q = q("A A", 1000, OverflowPolicy::KeepUnbreakable);
    q.paragraph.styles.push(q.paragraph.styles[0].clone());
    q.styles.push(q.styles[0]);
    q.paragraph.styles[1].language = "bad language".into();
    let mut b = Backend::default();
    assert!(layout_paragraph(&q, FONT, &mut b, &|| false).is_err());
    assert_eq!((b.shapes, b.metrics), (0, 0));
    let q = super::tests::q("A A", 1000, OverflowPolicy::KeepUnbreakable);
    let total = Cell::new(0);
    layout_paragraph(&q, FONT, &mut Backend::default(), &|| {
        total.set(total.get() + 1);
        false
    })
    .unwrap();
    for stop in 1..=total.get() {
        let at = Cell::new(0);
        assert!(
            layout_paragraph(&q, FONT, &mut Backend::default(), &|| {
                at.set(at.get() + 1);
                at.get() == stop
            })
            .is_err()
        );
    }
    let mut large = super::tests::q(&"A ".repeat(500), 1, OverflowPolicy::KeepUnbreakable);
    large.paragraph.styles[0].max_glyphs = 65536;
    assert!(matches!(
        layout_paragraph(&large, FONT, &mut Backend::default(), &|| false),
        Err(TextError::Limit(_))
    ));
}
