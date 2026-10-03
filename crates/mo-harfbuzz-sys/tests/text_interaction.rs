//! Geometry expectations come from the owned font's design units, not from a
//! second call to the interaction implementation. Paths independently exercise
//! the renderer's retained glyph origins and full line-search result.
use mo_common::Emu;
use mo_geometry::{Fixed, Point};
use mo_harfbuzz_sys::NativeShaper;
use mo_text::{
    TextError,
    backend::TextBackend,
    interaction::*,
    scene::{self, ParagraphPathsRequest},
};
use serde_json::{json, to_value};
const FONT: &[u8] = include_bytes!("../../../fixtures/fonts/owned-interaction.ttf");
fn q(text: &str) -> ParagraphInteractionRequest {
    serde_json::from_value(json!({
        "layout": {
            "paragraph": {"text":text,"direction":"leftToRight",
                "spans":if text.is_empty() {vec![]} else {vec![json!({"end":text.chars().count(),"style":0})]},
                "styles":[{"language":"und","features":[],"candidates":[{"font":0,"variations":[]}],
                    "suppressDottedCircle":false,"maxGlyphs":4096}],
                "fonts":[{"expectedSha256":"74c1ab9163bae9fdd7beb62dbe4e02a646a55f609bd48071bdc0f651e7180db8",
                    "faceIndex":0,"offset":"0","byteLength":FONT.len().to_string()}]},
            "styles":[{"fontSize":"1000","baselineShift":"0"}],"strutStyle":0,
            "spacing":{"kind":"natural"},"width":"10000","overflow":"keepUnbreakable"
        },"queries":[]
    })).unwrap()
}
fn f(value: i64) -> Fixed {
    Fixed::emu(Emu::new(value))
}
fn p(offset: u32, affinity: Affinity) -> TextPosition {
    TextPosition {
        scalar_offset: offset,
        affinity,
    }
}
fn run(q: &ParagraphInteractionRequest, font: &[u8]) -> ParagraphInteractionResult {
    let result = paragraph_interaction(q, font, &mut NativeShaper::default(), &|| false).unwrap();
    let rendered = scene::paragraph_paths(
        &ParagraphPathsRequest {
            layout: q.layout.clone(),
            bounds_tolerance: Fixed::from_raw(256),
        },
        font,
        &mut NativeShaper::default(),
        &|| false,
    )
    .unwrap();
    assert_eq!(
        to_value(&result.layout).unwrap(),
        to_value(rendered.layout).unwrap()
    );
    assert!(result.map.is_some(), "{:?}", result.layout.issues);
    result
}
#[test]
fn ligature_carets_include_actual_gpos_origin_and_reuse_unique_glyph_query() {
    let request = q("AAA AAA");
    let r = run(&request, FONT);
    let m = r.map.unwrap();
    for (i, x) in [0, 210, 468, 577].into_iter().enumerate() {
        assert_eq!(
            m.caret(p(i as u32, Affinity::Upstream)).unwrap().edge.x,
            f(x)
        );
    }
    assert_eq!(m.work.caret_calls, 1);
    assert_eq!(m.work.unique_glyphs, 1);
    assert!(
        m.cells[..3]
            .iter()
            .all(|c| c.placement == CaretPlacement::FontLigature)
    );
    let rendered = scene::paragraph_paths(
        &ParagraphPathsRequest {
            layout: request.layout,
            bounds_tolerance: Fixed::from_raw(256),
        },
        FONT,
        &mut NativeShaper::default(),
        &|| false,
    )
    .unwrap()
    .scene
    .unwrap();
    assert_eq!(
        m.cells[0].trailing.x,
        rendered.glyphs[0].origin.x.checked_add(f(173)).unwrap()
    );
    assert_eq!(
        m.cells[0].trailing.top,
        rendered.glyphs[0].origin.y.checked_sub(f(800)).unwrap()
    );
    assert_eq!(m.cells[0].leading.top.raw(), (25i128 << 32) / 2);
    assert_eq!(m.cells[0].trailing.top.raw(), (3i128 << 32) / 2);
}
#[test]
fn absent_font_carets_partition_actual_cluster_advance_explicitly() {
    let font = include_bytes!("../../../fixtures/fonts/owned-tracking.ttf");
    let mut request = q("AA");
    request.layout.paragraph.fonts = serde_json::from_value(
        serde_json::from_str::<serde_json::Value>(include_str!(
            "../../../fixtures/fonts/tracking-manifest.json"
        ))
        .unwrap()["fonts"]
            .clone(),
    )
    .unwrap();
    let m = run(&request, font).map.unwrap();
    assert_eq!(m.cells.len(), 2);
    assert!(m.cells.iter().all(|c| c.placement
        == CaretPlacement::ClusterPartition {
            reason: PartitionReason::FontCaretsAbsent
        }));
    assert_eq!(m.cells[0].leading.x, f(0));
    assert_eq!(m.cells[0].trailing.x, f(300));
    assert_eq!(m.cells[1].trailing.x, f(600));
}
#[test]
fn bidi_split_carets_hit_containing_cell_and_keep_logical_selection() {
    let m = run(&q("AאבA"), FONT).map.unwrap();
    assert_eq!(
        m.cells.iter().map(|c| c.level).collect::<Vec<_>>(),
        [0, 1, 1, 0]
    );
    for offset in [1, 3] {
        assert_eq!(
            m.caret(p(offset, Affinity::Upstream)).unwrap().edge.x,
            f(600)
        );
        assert_eq!(
            m.caret(p(offset, Affinity::Downstream)).unwrap().edge.x,
            f(1800)
        );
    }
    let queries = [
        TextQuery::Hit {
            point: Point {
                x: f(650),
                y: f(500),
            },
        },
        TextQuery::Selection {
            anchor: p(3, Affinity::Upstream),
            focus: p(1, Affinity::Downstream),
        },
    ];
    let r = m.query(&queries, &|| false).unwrap();
    let TextQueryResult::Hit { caret, inside } = &r[0] else {
        panic!()
    };
    assert!(*inside);
    assert_eq!(caret.position, p(3, Affinity::Upstream));
    let TextQueryResult::Selection {
        anchor,
        focus,
        fragments,
    } = &r[1]
    else {
        panic!()
    };
    assert_eq!(anchor.position.scalar_offset, 3);
    assert_eq!(focus.position.scalar_offset, 1);
    assert_eq!(fragments.len(), 2);
    assert_eq!(
        (fragments[0].bounds.min.x, fragments[0].bounds.max.x),
        (f(1200), f(1800))
    );
    assert_eq!(
        (fragments[1].bounds.min.x, fragments[1].bounds.max.x),
        (f(600), f(1200))
    );
}
#[test]
fn grapheme_boundaries_preserve_marks_surrogates_and_structural_breaks() {
    let m = run(&q("A\u{301} 😀"), FONT).map.unwrap();
    assert_eq!(
        m.boundaries
            .iter()
            .map(|b| (b.scalar_offset, b.utf16_offset))
            .collect::<Vec<_>>(),
        [(0, 0), (2, 2), (3, 3), (4, 5)]
    );
    assert_eq!(m.work.caret_calls, 0);
    assert!(m.caret(p(1, Affinity::Downstream)).is_err());
    for text in ["A\u{2028}", "A\u{b}", "A\u{c}"] {
        let m = run(&q(text), FONT).map.unwrap();
        let n = text.chars().count() as u32;
        assert_eq!(m.lines.len(), 2);
        let a = m.caret(p(n, Affinity::Upstream)).unwrap();
        let b = m.caret(p(n, Affinity::Downstream)).unwrap();
        assert_eq!((a.line, a.edge.x), (0, f(600)));
        assert_eq!((b.line, b.edge.x), (1, f(0)));
        assert_eq!(m.cells.last().unwrap().placement, CaretPlacement::Invisible);
    }
    // Paragraph terminators belong to this paragraph; the next paragraph is
    // laid out separately. Only in-paragraph line breaks add an empty line.
    for text in ["A\n", "A\r\n"] {
        let m = run(&q(text), FONT).map.unwrap();
        assert_eq!(m.lines.len(), 1);
        assert_eq!(m.cells.len(), 2);
        assert_eq!(m.cells[1].placement, CaretPlacement::Invisible);
        if text.contains('\r') {
            assert!(m.caret(p(2, Affinity::Downstream)).is_err());
        }
    }
    let m = run(&q(""), FONT).map.unwrap();
    assert!(m.cells.is_empty());
    assert_eq!(m.lines.len(), 1);
    let c = m.caret(p(0, Affinity::Upstream)).unwrap();
    assert_eq!(c.edge.x, f(0));
    assert_eq!(c.edge.bottom.checked_sub(c.edge.top).unwrap(), f(1000));
}
#[test]
fn wrapping_affinity_tracking_and_baseline_share_rendered_line_plan() {
    let mut request = q("A A");
    request.layout.width = Emu::new(1200);
    let m = run(&request, FONT).map.unwrap();
    assert_eq!(m.lines.len(), 2);
    let a = m.caret(p(2, Affinity::Upstream)).unwrap();
    let b = m.caret(p(2, Affinity::Downstream)).unwrap();
    assert_eq!((a.line, a.edge.x), (0, f(1200)));
    assert_eq!((b.line, b.edge.x), (1, f(0)));
    let mut request = q("AA");
    request.layout.styles[0].cluster_spacing = Fixed::from_raw(-(100i128 << 32) + 1);
    request.layout.styles[0].baseline_shift = Emu::new(125).into();
    let m = run(&request, FONT).map.unwrap();
    assert_eq!(m.cells[0].trailing.x.raw(), (500i128 << 32) + 1);
    assert_eq!(m.cells[1].trailing.x.raw(), (1000i128 << 32) + 2);
}
struct NoCalls;
impl TextBackend for NoCalls {
    fn shape_batch(&mut self, _: &[u8], _: &[u32]) -> Result<Vec<u32>, TextError> {
        panic!("preflight must precede backend")
    }
    fn invalidate(&mut self) {
        panic!("unused backend must remain usable")
    }
}
#[test]
fn invalid_late_query_is_rejected_before_font_or_component_work() {
    let mut request = q("A\u{301} 😀");
    request.queries = vec![
        TextQuery::Caret {
            position: p(4, Affinity::Upstream),
        },
        TextQuery::Caret {
            position: p(1, Affinity::Downstream),
        },
    ];
    assert!(matches!(
        paragraph_interaction(&request, &[], &mut NoCalls, &|| false),
        Err(TextError::Invalid(_))
    ));
    request.queries = vec![
        TextQuery::Caret {
            position: p(4, Affinity::Upstream)
        };
        65
    ];
    assert!(matches!(
        paragraph_interaction(&request, &[], &mut NoCalls, &|| false),
        Err(TextError::Limit(_))
    ));
}
