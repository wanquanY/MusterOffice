use super::*;
use crate::geometry::test_support::{Backend, FONT};
use mo_common::Emu;

fn layout(text: &str, width: i64, enabled: bool) -> ParagraphLayoutResult {
    let mut q = super::tests::q(text, width, OverflowPolicy::KeepUnbreakable);
    q.hanging_punctuation = if enabled {
        HangingPunctuation::End
    } else {
        HangingPunctuation::None
    };
    layout_paragraph(&q, FONT, &mut Backend::default(), &|| false).unwrap()
}
#[test]
fn an_overflowing_terminal_punctuation_stays_with_fitting_text_without_losing_its_glyph() {
    let ordinary = layout("AA。", 1200, false);
    assert!(ordinary.decisions[0].overflows);
    let hung = layout("AA。", 1200, true);
    assert_eq!(hung.decisions.len(), 1);
    assert!(!hung.decisions[0].overflows);
    let end = hung.decisions[0].hanging.as_ref().unwrap();
    assert_eq!((end.start.scalar_offset, end.end.scalar_offset), (2, 3));
    assert_eq!(end.body_pen_min, Position::ZERO);
    assert_eq!(end.body_pen_max, Position::emu(Emu::new(1200)));
    let geometry = hung.geometry.unwrap().layout.unwrap();
    assert_eq!(geometry.lines[0].glyphs.len(), 3);
    assert_eq!(geometry.lines[0].pen_max.get(), 1800);
    assert_eq!(geometry.lines[0].glyphs[2].x.get(), 1200);
}
#[test]
fn hanging_changes_farthest_fit_but_does_not_hide_body_overflow_or_hang_openers() {
    let ordinary = layout("A A。A", 1800, false);
    let hung = layout("A A。A", 1800, true);
    assert_eq!(ordinary.decisions[0].end.scalar_offset, 2);
    assert_eq!(hung.decisions[0].end.scalar_offset, 4);
    assert!(hung.decisions[0].hanging.is_some());
    for text in ["AAA。", "AA。 ", "AA（", "AA$", "AA-", "。", "AA。。"] {
        let result = layout(text, 1200, true);
        assert!(
            result.decisions.iter().all(|d| d.hanging.is_none()),
            "{text}"
        );
    }
    let fitting = layout("AA。", 1800, true);
    assert!(fitting.decisions[0].hanging.is_none());
    assert!(!fitting.decisions[0].overflows);
}
#[test]
fn grapheme_marks_and_explicit_line_breaks_keep_their_source_coordinates() {
    let result = layout("AA。\u{301}\u{2028}A", 1200, true);
    let first = &result.decisions[0];
    assert_eq!(first.end.scalar_offset, 5);
    let hanging = first.hanging.as_ref().unwrap();
    assert_eq!(
        (hanging.start.scalar_offset, hanging.end.scalar_offset),
        (2, 4)
    );
    assert_eq!((hanging.start.utf8_offset, hanging.end.utf8_offset), (2, 7));
    assert_eq!(result.decisions[1].end.scalar_offset, 6);
    assert!(result.decisions[1].hanging.is_none());
}
#[test]
fn rtl_visual_edge_hanging_reports_unshifted_body_bounds_for_alignment() {
    let mut q = super::tests::q("אב。", 1200, OverflowPolicy::KeepUnbreakable);
    q.paragraph.direction = mo_unicode::bidi::ParagraphDirection::RightToLeft;
    q.hanging_punctuation = HangingPunctuation::End;
    let result = layout_paragraph(&q, FONT, &mut Backend::default(), &|| false).unwrap();
    assert_eq!(result.decisions.len(), 1);
    assert!(!result.decisions[0].overflows);
    let hanging = result.decisions[0].hanging.as_ref().unwrap();
    assert_eq!(hanging.body_pen_min, Position::emu(Emu::new(600)));
    assert_eq!(hanging.body_pen_max, Position::emu(Emu::new(1800)));
}
#[test]
fn hanging_uses_shaped_cluster_tracking_and_exact_sub_emu_widths() {
    let mut q = super::tests::q("AA。", 1400, OverflowPolicy::KeepUnbreakable);
    q.hanging_punctuation = HangingPunctuation::End;
    q.styles[0].cluster_spacing = Position::emu(Emu::new(100));
    let result = layout_paragraph(&q, FONT, &mut Backend::default(), &|| false).unwrap();
    assert_eq!(
        result.decisions[0].hanging.as_ref().unwrap().body_pen_max,
        Position::emu(Emu::new(1400))
    );
    let mut input = FlowInput::from(&q);
    input.widths = LineWidths::uniform(Position::from_raw((1400i128 << 32) - 1));
    let result = layout_flow(
        &input,
        resources::ResourceInput::Bundle(FONT),
        &mut Backend::default(),
        &|| false,
        geometry::evaluate,
    )
    .unwrap();
    assert!(result.decisions[0].overflows);
    assert!(result.decisions[0].hanging.is_none());
}

struct Ligature(Backend);
impl backend::TextBackend for Ligature {
    fn shape_batch(&mut self, font: &[u8], words: &[u32]) -> Result<Vec<u32>, TextError> {
        let runs = backend::decode_requests(words)?;
        assert_eq!(runs.len(), 1);
        let mut result = self.0.shape_batch(font, words)?;
        for i in 0..runs[0].words[8] as usize {
            result[12 + i * 7] = runs[0].words[7];
        }
        Ok(result)
    }
    fn measure_batch(&mut self, font: &[u8], words: &[u32]) -> Result<Vec<u32>, TextError> {
        self.0.measure_batch(font, words)
    }
    fn invalidate(&mut self) {
        self.0.invalidate();
    }
}
#[test]
fn ligatures_containing_body_text_cannot_be_discounted_as_punctuation() {
    let mut q = super::tests::q("AA!", 1200, OverflowPolicy::KeepUnbreakable);
    q.hanging_punctuation = HangingPunctuation::End;
    let result = layout_paragraph(&q, FONT, &mut Ligature(Backend::default()), &|| false).unwrap();
    assert!(result.decisions[0].overflows);
    assert!(result.decisions[0].hanging.is_none());
}
