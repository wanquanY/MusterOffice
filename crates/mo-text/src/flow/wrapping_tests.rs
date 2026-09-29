use super::*;
use crate::geometry::test_support::{Backend, FONT};

fn nowrap(text: &str, width: i64) -> ParagraphLayoutRequest {
    let mut q = tests::q(text, width, OverflowPolicy::EmergencyGrapheme);
    q.wrapping = LineWrapping::NoWrap;
    q
}

#[test]
fn no_wrap_preserves_complete_content_and_measures_actual_width() {
    let q = nowrap("A A A", 1);
    let r = layout_paragraph(&q, FONT, &mut Backend::default(), &|| false).unwrap();
    assert!(r.issues.is_empty());
    assert_eq!(r.decisions.len(), 1);
    assert_eq!(r.decisions[0].end.scalar_offset, 5);
    assert!(r.decisions[0].overflows);
    assert!(!r.decisions[0].emergency);
    assert_eq!(r.work.evaluated_candidates, 1);
    assert_eq!(
        r.geometry.unwrap().layout.unwrap().lines[0].advance.get(),
        3000
    );
}

#[test]
fn explicit_breaks_and_empty_final_line_survive_without_soft_wraps() {
    let q = nowrap("AA A\u{2028}A A\u{2028}", 1);
    let r = layout_paragraph(&q, FONT, &mut Backend::default(), &|| false).unwrap();
    assert_eq!(
        r.decisions
            .iter()
            .map(|d| d.end.scalar_offset)
            .collect::<Vec<_>>(),
        [5, 9, 9]
    );
    assert!(r.decisions.iter().all(|d| !d.emergency));
    assert_eq!(r.work.evaluated_candidates, 3);
    let layout = r.geometry.unwrap().layout.unwrap();
    assert_eq!(layout.lines.len(), 3);
    assert!(layout.lines[2].glyphs.is_empty());
    assert!(layout.height.get() > 0);
}

#[test]
fn wrap_mode_is_independent_of_exact_first_and_rest_widths() {
    let q = nowrap("AA\u{2028}AA", 1);
    let mut input = FlowInput::from(&q);
    input.widths = LineWidths {
        first: Position::emu(mo_common::Emu::new(1200)),
        rest: Position::from_raw((1200i128 << 32) - 1),
    };
    let r = layout_flow(
        &input,
        resources::ResourceInput::Bundle(FONT),
        &mut Backend::default(),
        &|| false,
        geometry::evaluate,
    )
    .unwrap();
    assert_eq!(r.decisions.len(), 2);
    assert!(!r.decisions[0].overflows);
    assert!(r.decisions[1].overflows);
    assert!(r.decisions.iter().all(|d| !d.emergency));
}

#[test]
fn legacy_input_defaults_to_wrapping_and_nowrap_does_not_mask_errors() {
    let mut wire =
        serde_json::to_value(tests::q("A A", 1, OverflowPolicy::EmergencyGrapheme)).unwrap();
    wire.as_object_mut().unwrap().remove("wrapping");
    let q: ParagraphLayoutRequest = serde_json::from_value(wire).unwrap();
    assert_eq!(q.wrapping, LineWrapping::Wrap);
    for text in ["A\tA", "A\u{ad}A", "A\u{fffc}A"] {
        let r =
            layout_paragraph(&nowrap(text, 1), FONT, &mut Backend::default(), &|| false).unwrap();
        assert!(!r.issues.is_empty());
        assert!(r.geometry.is_none());
    }
    assert!(layout_paragraph(&nowrap("A", 0), FONT, &mut Backend::default(), &|| false).is_err());
    assert!(matches!(
        layout_paragraph(&nowrap("A", 1), FONT, &mut Backend::default(), &|| true),
        Err(TextError::Cancelled)
    ));
}
