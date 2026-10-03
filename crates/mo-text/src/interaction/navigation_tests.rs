use super::*;
use crate::geometry::test_support::{Backend, FONT};
use std::cell::Cell;
fn position(scalar_offset: u32, affinity: Affinity) -> TextPosition {
    TextPosition {
        scalar_offset,
        affinity,
    }
}
fn map(text: &str) -> InteractionMap {
    paragraph_interaction(&tests::q(text), FONT, &mut Backend::default(), &|| false)
        .unwrap()
        .map
        .unwrap()
}
fn moved(m: &InteractionMap, p: TextPosition, movement: CaretMove) -> CaretNavigation {
    m.move_caret(p, movement, None, &|| false).unwrap()
}
#[test]
fn visual_bidi_edges_follow_render_order_and_keep_split_affinities() {
    // X9 removes the override controls. The three overridden A's are RTL.
    let m = map("AA\u{202e}AAA\u{202c}AA");
    assert_eq!(m.lines[0].visual_cells, [0, 1, 5, 4, 3, 7, 8]);
    let mut p = position(0, Affinity::Downstream);
    for (offset, affinity) in [
        (1, Affinity::Upstream),
        (2, Affinity::Upstream),
        (5, Affinity::Downstream),
        (4, Affinity::Downstream),
        (3, Affinity::Downstream),
        (8, Affinity::Upstream),
        (9, Affinity::Upstream),
    ] {
        let r = moved(&m, p, CaretMove::Right);
        assert_eq!(r.caret.position, position(offset, affinity));
        assert!(!r.exhausted);
        p = r.caret.position;
    }
    assert!(moved(&m, p, CaretMove::Right).exhausted);
    for (offset, affinity) in [
        (8, Affinity::Downstream),
        (7, Affinity::Downstream),
        (4, Affinity::Upstream),
        (5, Affinity::Upstream),
        (6, Affinity::Upstream),
        (1, Affinity::Downstream),
        (0, Affinity::Downstream),
    ] {
        let r = moved(&m, p, CaretMove::Left);
        assert_eq!(r.caret.position, position(offset, affinity));
        p = r.caret.position;
    }
    assert!(moved(&m, p, CaretMove::Left).exhausted);
}
#[test]
fn logical_moves_follow_extended_graphemes_and_visit_removed_controls() {
    let m = map("A\u{301}\u{202e}AA\u{202c}\r\n");
    let mut p = position(0, Affinity::Downstream);
    for offset in [2, 3, 4, 5, 6, 8] {
        let r = moved(&m, p, CaretMove::NextGrapheme);
        assert_eq!(r.caret.position.scalar_offset, offset);
        p = r.caret.position;
    }
    assert!(moved(&m, p, CaretMove::NextGrapheme).exhausted);
    for offset in [6, 5, 4, 3, 2, 0] {
        let r = moved(&m, p, CaretMove::PreviousGrapheme);
        assert_eq!(r.caret.position.scalar_offset, offset);
        p = r.caret.position;
    }
    assert!(moved(&m, p, CaretMove::PreviousGrapheme).exhausted);
    assert_eq!(
        moved(&m, p, CaretMove::TextEnd).caret.boundary.utf16_offset,
        8
    );
}
#[test]
fn vertical_sticky_x_survives_short_and_empty_lines() {
    let m = map("AAAA\u{2028}A\u{2028}\u{2028}AAAA");
    assert_eq!(m.lines.len(), 4);
    let start = m.caret(position(3, Affinity::Downstream)).unwrap();
    let mut r = moved(&m, start.position, CaretMove::Down);
    assert_eq!(r.caret.position.scalar_offset, 6);
    assert_eq!(r.preferred_x, Some(start.edge.x));
    for (line, offset) in [(2, 7), (3, 11)] {
        r = m
            .move_caret(r.caret.position, CaretMove::Down, r.preferred_x, &|| false)
            .unwrap();
        assert_eq!(r.caret.line, line);
        assert_eq!(r.caret.position.scalar_offset, offset);
        assert_eq!(r.preferred_x, Some(start.edge.x));
    }
    assert!(
        m.move_caret(r.caret.position, CaretMove::Down, r.preferred_x, &|| false)
            .unwrap()
            .exhausted
    );
    r = m
        .move_caret(r.caret.position, CaretMove::Up, r.preferred_x, &|| false)
        .unwrap();
    r = m
        .move_caret(r.caret.position, CaretMove::Up, r.preferred_x, &|| false)
        .unwrap();
    r = m
        .move_caret(r.caret.position, CaretMove::Up, r.preferred_x, &|| false)
        .unwrap();
    assert_eq!(r.caret.edge.x, start.edge.x);
    assert_eq!(r.caret.line, 0);
}
#[test]
fn line_and_visual_movement_respect_base_direction_and_soft_wrap_affinity() {
    for rtl in [false, true] {
        let mut q = tests::q("AAAA");
        q.layout.width = mo_common::Emu::new(310000);
        q.layout.overflow = crate::flow::OverflowPolicy::EmergencyGrapheme;
        if rtl {
            q.layout.paragraph.direction = mo_unicode::bidi::ParagraphDirection::RightToLeft;
        }
        let m = paragraph_interaction(&q, FONT, &mut Backend::default(), &|| false)
            .unwrap()
            .map
            .unwrap();
        assert_eq!(m.lines.len(), 2);
        let end = moved(&m, position(0, Affinity::Downstream), CaretMove::LineEnd);
        let start = moved(&m, position(2, Affinity::Downstream), CaretMove::LineStart);
        assert_eq!((end.caret.line, start.caret.line), (0, 1));
        // ASCII remains LTR inside an RTL paragraph: physical edges reverse
        // their reading-order meanings, but the line transition follows base direction.
        let direction = if rtl {
            CaretMove::Left
        } else {
            CaretMove::Right
        };
        let r = moved(&m, end.caret.position, direction);
        assert_eq!(r.caret.position, start.caret.position);
        assert_eq!(r.caret.line, 1);
        assert!(!r.exhausted);
        let back = moved(
            &m,
            r.caret.position,
            if rtl {
                CaretMove::Right
            } else {
                CaretMove::Left
            },
        );
        assert_eq!(back.caret.position, end.caret.position);
    }
}
#[test]
fn negative_tracking_does_not_reorder_logical_visual_slots() {
    let mut q = tests::q("AAAA");
    q.layout.styles[0].cluster_spacing = Fixed::emu(mo_common::Emu::new(-200000));
    let m = paragraph_interaction(&q, FONT, &mut Backend::default(), &|| false)
        .unwrap()
        .map
        .unwrap();
    let mut p = position(0, Affinity::Downstream);
    for offset in 1..=4 {
        let r = moved(&m, p, CaretMove::Right);
        assert_eq!(r.caret.position.scalar_offset, offset);
        p = r.caret.position;
    }
    assert!(m.cells[1].leading.x < m.cells[0].leading.x);
}
#[test]
fn invalid_navigation_is_rejected_before_components_and_batches_are_bounded_and_cancellable() {
    let mut q = tests::q("AA");
    q.queries = vec![TextQuery::Move {
        position: position(0, Affinity::Downstream),
        movement: CaretMove::Right,
        preferred_x: Some(Fixed::ZERO),
    }];
    let mut backend = Backend::default();
    assert!(paragraph_interaction(&q, FONT, &mut backend, &|| false).is_err());
    assert_eq!((backend.shapes, backend.metrics), (0, 0));
    let m = map("AA\u{202e}AA\u{202c}\u{2028}AAAA");
    let queries = vec![TextQuery::Move {
        position: position(4, Affinity::Downstream),
        movement: CaretMove::Down,
        preferred_x: None,
    }];
    let calls = Cell::new(0);
    m.query(&queries, &|| {
        calls.set(calls.get() + 1);
        false
    })
    .unwrap();
    for stop in 1..=calls.get() {
        let count = Cell::new(0);
        assert!(matches!(
            m.query(&queries, &|| {
                count.set(count.get() + 1);
                count.get() == stop
            }),
            Err(TextError::Cancelled)
        ));
    }
    let mut q = tests::q(&"A".repeat(9000));
    q.layout.paragraph.styles[0].max_glyphs = 10000;
    let m = paragraph_interaction(&q, FONT, &mut Backend::default(), &|| false)
        .unwrap()
        .map
        .unwrap();
    assert!(matches!(
        m.query(&vec![queries[0].clone(); 64], &|| false),
        Err(TextError::Limit("interaction query work"))
    ));
    let parsed: TextQuery = serde_json::from_str(r#"{"kind":"move","position":{"scalarOffset":0,"affinity":"downstream"},"movement":"down","preferredX":"0"}"#).unwrap();
    assert!(matches!(
        parsed,
        TextQuery::Move {
            preferred_x: Some(Fixed::ZERO),
            ..
        }
    ));
}
