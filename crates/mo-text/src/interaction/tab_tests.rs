use super::*;
use crate::{
    flow::LineWrapping,
    geometry::{
        LeftTabStops,
        test_support::{Backend, FONT},
    },
};
use mo_common::Emu;

fn f(n: i64) -> Fixed {
    Fixed::emu(Emu::new(n))
}
fn request(text: &str) -> ParagraphInteractionRequest {
    let mut q = super::tests::q(text);
    q.layout.styles[0].font_size = Emu::new(1000);
    q.layout.tabs = Some(LeftTabStops {
        interval: f(1000),
        stops: vec![],
        first_line_offset: Fixed::ZERO,
        continuation_offset: Fixed::ZERO,
    });
    q
}
fn run(q: &ParagraphInteractionRequest) -> ParagraphInteractionResult {
    paragraph_interaction(q, FONT, &mut Backend::default(), &|| false).unwrap()
}
fn pos(scalar_offset: u32) -> TextPosition {
    TextPosition {
        scalar_offset,
        affinity: Affinity::Downstream,
    }
}
#[test]
fn tabs_use_exact_grid_and_have_real_selection_hit_and_navigation_edges() {
    let mut q = request("A\tA");
    q.queries = vec![
        TextQuery::Selection {
            anchor: pos(2),
            focus: pos(1),
        },
        TextQuery::Hit {
            point: Point {
                x: f(999),
                y: f(100),
            },
        },
        TextQuery::Move {
            position: pos(1),
            movement: CaretMove::Right,
            preferred_x: None,
        },
    ];
    let r = run(&q);
    let map = r.map.unwrap();
    assert_eq!(map.cells.len(), 3);
    let tab = &map.cells[1];
    assert_eq!(tab.placement, CaretPlacement::TabEdges);
    assert_eq!((tab.leading.x, tab.trailing.x), (f(600), f(1000)));
    assert_eq!(map.cells[2].leading.x, tab.trailing.x);
    let line = &r.layout.geometry.unwrap().layout.unwrap().lines[0];
    assert_eq!(line.advance.get(), 1600);
    assert_eq!(line.glyphs.len(), 2);
    assert_eq!(line.glyphs[1].x.get(), 1000);
    let TextQueryResult::Selection { fragments, .. } = &r.results[0] else {
        panic!()
    };
    assert_eq!(fragments.len(), 1);
    assert_eq!(
        (fragments[0].bounds.min.x, fragments[0].bounds.max.x),
        (f(600), f(1000))
    );
    let TextQueryResult::Hit { caret, inside } = &r.results[1] else {
        panic!()
    };
    assert!(*inside);
    assert_eq!(caret.position.scalar_offset, 2);
    let TextQueryResult::Moved { result } = &r.results[2] else {
        panic!()
    };
    assert_eq!(result.caret.position.scalar_offset, 2);
}
#[test]
fn explicit_stops_then_grid_consecutive_tabs_and_tab_only_line_are_measured() {
    let mut q = request("\t\t\t\t");
    q.layout.tabs.as_mut().unwrap().stops = vec![f(800), f(1300)];
    let r = run(&q);
    let cells = r.map.unwrap().cells;
    assert_eq!(
        cells.iter().map(|c| c.trailing.x).collect::<Vec<_>>(),
        [f(800), f(1300), f(2000), f(3000)]
    );
    let layout = r.layout.geometry.unwrap().layout.unwrap();
    assert!(layout.lines[0].glyphs.is_empty());
    assert_eq!(layout.lines[0].pen_max.get(), 3000);
    assert!(layout.lines[0].height.get() > 0);
}
#[test]
fn candidate_fitting_and_nowrap_use_the_same_tab_pen_as_rendering() {
    let mut q = request("A\tA");
    q.layout.width = Emu::new(1500);
    let r = run(&q);
    assert_eq!(
        r.layout
            .decisions
            .iter()
            .map(|d| d.end.scalar_offset)
            .collect::<Vec<_>>(),
        [2, 3]
    );
    assert!(r.layout.decisions.iter().all(|d| !d.overflows));
    assert_eq!(r.map.unwrap().cells[2].leading.x, Fixed::ZERO);
    q.layout.wrapping = LineWrapping::NoWrap;
    let r = run(&q);
    assert_eq!(r.layout.decisions.len(), 1);
    assert!(r.layout.decisions[0].overflows);
    assert_eq!(
        r.layout.geometry.unwrap().layout.unwrap().lines[0]
            .pen_max
            .get(),
        1600
    );
}
#[test]
fn first_indent_and_exact_fractional_stop_do_not_shift_continuation_grid() {
    let mut q = request("\tA\u{2028}\tA");
    let tabs = q.layout.tabs.as_mut().unwrap();
    tabs.first_line_offset = f(-200);
    let fractional = Fixed::from_raw(f(1000).raw() + 1);
    tabs.interval = fractional;
    let r = run(&q);
    let map = r.map.unwrap();
    assert_eq!(map.cells[0].trailing.x, f(200));
    assert_eq!(map.cells[3].trailing.x, fractional);
    assert_eq!(map.cells[4].leading.x, fractional);
    let mut q = request("A\tA");
    q.layout.tabs.as_mut().unwrap().first_line_offset = f(100);
    assert_eq!(run(&q).map.unwrap().cells[1].trailing.x, f(900));
}
#[test]
fn signed_tracking_before_tab_and_mixed_bidi_keep_source_identity() {
    let mut q = request("A\tA");
    q.layout.styles[0].cluster_spacing = f(-900);
    let r = run(&q);
    let map = r.map.unwrap();
    assert_eq!(
        (map.cells[1].leading.x, map.cells[1].trailing.x),
        (f(-300), f(0))
    );
    let line = &r.layout.geometry.unwrap().layout.unwrap().lines[0];
    assert_eq!((line.pen_min.get(), line.pen_max.get()), (-300, 600));
    let q = request("A\tאב");
    let map = run(&q).map.unwrap();
    assert_eq!(
        map.cells
            .iter()
            .map(|c| c.start.scalar_offset)
            .collect::<Vec<_>>(),
        [0, 1, 2, 3]
    );
    assert_eq!(map.cells[1].placement, CaretPlacement::TabEdges);
    assert!(map.cells[2].leading.x > map.cells[2].trailing.x);
}
#[test]
fn tab_style_participates_in_metrics_and_exact_line_spacing() {
    let mut q = request("A\t");
    q.layout
        .paragraph
        .styles
        .push(q.layout.paragraph.styles[0].clone());
    q.layout.styles.push(q.layout.styles[0]);
    q.layout.styles[1].font_size = Emu::new(2000);
    q.layout.styles[1].baseline_shift = Emu::new(200).into();
    q.layout.paragraph.spans = vec![
        crate::itemize::StyleSpan { end: 1, style: 0 },
        crate::itemize::StyleSpan { end: 2, style: 1 },
    ];
    let map = run(&q).map.unwrap();
    assert_eq!(
        map.cells[1]
            .trailing
            .bottom
            .checked_sub(map.cells[1].trailing.top)
            .unwrap(),
        f(2000)
    );
    q.layout.spacing = crate::geometry::LineSpacing::StyleMaximum {
        heights: vec![f(1100), f(2200)],
    };
    assert_eq!(
        run(&q).layout.geometry.unwrap().layout.unwrap().lines[0]
            .height
            .get(),
        2200
    );
}
#[test]
fn absent_invalid_or_excessive_policy_and_cancellation_do_not_fake_geometry() {
    let mut q = request("A\tA");
    q.layout.tabs = None;
    let r = run(&q);
    assert!(r.map.is_none());
    assert!(matches!(
        r.layout.issues.as_slice(),
        [crate::flow::FlowIssue::Tab { scalar: 1 }]
    ));
    for stops in [vec![f(1), f(1)], vec![f(2), f(1)], vec![f(0); 33]] {
        let mut q = request("A\tA");
        q.layout.tabs.as_mut().unwrap().stops = stops;
        let mut backend = Backend::default();
        assert!(paragraph_interaction(&q, FONT, &mut backend, &|| false).is_err());
        assert_eq!((backend.shapes, backend.metrics), (0, 0));
    }
    let mut q = request("A\tA");
    q.layout.tabs.as_mut().unwrap().interval = Fixed::ZERO;
    let mut backend = Backend::default();
    assert!(paragraph_interaction(&q, FONT, &mut backend, &|| false).is_err());
    q.layout.tabs.as_mut().unwrap().interval = f(1000);
    assert!(matches!(
        paragraph_interaction(&q, FONT, &mut backend, &|| true),
        Err(TextError::Cancelled)
    ));
    assert_eq!((backend.shapes, backend.metrics), (0, 0));
}
