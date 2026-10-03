#[allow(dead_code)]
#[path = "../../../tools/test-support/source_text_page.rs"]
mod support;
use mo_common::Emu;
use mo_geometry::{Fixed, Point};
use mo_harfbuzz_sys::NativeShaper;
use mo_presentation_compile::source_frame::{self, interaction::*, *};
use mo_text::{
    interaction::{Affinity, TextPosition},
    manifest::*,
};
use support::*;
const BYTES: &[u8] = include_bytes!("../../../fixtures/fonts/owned-interaction.ttf");
fn f(n: i64) -> Fixed {
    Fixed::emu(Emu::new(n))
}
fn fonts() -> FontManifest {
    let mut m: serde_json::Value = serde_json::from_str(include_str!(
        "../../../fixtures/fonts/tracking-manifest.json"
    ))
    .unwrap();
    m["fonts"][0]["expectedSha256"] =
        serde_json::json!("74c1ab9163bae9fdd7beb62dbe4e02a646a55f609bd48071bdc0f651e7180db8");
    m["fonts"][0]["byteLength"] = serde_json::json!(BYTES.len().to_string());
    serde_json::from_value(m).unwrap()
}
fn position(paragraph: u32, scalar_offset: u32) -> FrameTextPosition {
    FrameTextPosition {
        paragraph,
        position: TextPosition {
            scalar_offset,
            affinity: Affinity::Downstream,
        },
    }
}
fn editor(shape: &str, limits: FrameInteractionLimits) -> SourceFrameEditor {
    let b = fixture(shape);
    let i = read(&b);
    let fonts = fonts();
    let manifest = PreparedManifest::load(&fonts, BYTES, Default::default(), &|| false).unwrap();
    let q = SourceFrameRequest {
        expected_source_sha256: i.source_sha256.clone(),
        object: mo_pptx::source::SourceObjectRef {
            part: SLIDE.into(),
            native_id: 42,
        },
        bounds_tolerance: Fixed::from_raw(1 << 24),
    };
    let regular = source_frame::compile(
        &i,
        &q,
        &manifest,
        &mut NativeShaper::default(),
        Default::default(),
        &|| false,
    )
    .unwrap();
    let result = source_frame::interaction::compile(
        &i,
        &q,
        &manifest,
        &mut NativeShaper::default(),
        Default::default(),
        limits,
        &|| false,
    )
    .unwrap();
    let mut a = serde_json::to_value(&regular).unwrap();
    let mut b = serde_json::to_value(result.frame()).unwrap();
    a.as_object_mut().unwrap().remove("work");
    b.as_object_mut().unwrap().remove("work");
    assert_eq!(a, b, "editing retains exactly the rendering frame");
    assert_eq!(result.frame().work.glyphs, regular.work.glyphs);
    assert_eq!(
        result.frame().work.path_commands,
        regular.work.path_commands
    );
    let caret_calls: u32 = result.paragraphs().iter().map(|p| p.work.caret_calls).sum();
    assert_eq!(
        result.frame().work.component_calls,
        regular.work.component_calls + caret_calls
    );
    result
}
#[test]
fn alignment_anchor_indent_and_gpos_reuse_rendered_frame_coordinates() {
    for align in ["l", "ctr", "r"] {
        for anchor in ["t", "ctr", "b"] {
            let s = shape(42, 100000, 100000, "", &colored("AAA", "123456"))
                .replace(
                    "<a:p>",
                    &format!("<a:p><a:pPr algn=\"{align}\" marL=\"11000\" indent=\"9000\"/>"),
                )
                .replace("<a:bodyPr ", &format!("<a:bodyPr anchor=\"{anchor}\" "));
            let e = editor(&s, Default::default());
            let result = e
                .query(
                    &[FrameTextQuery::Caret {
                        position: position(0, 1),
                    }],
                    &|| false,
                )
                .unwrap();
            let FrameTextQueryResult::Caret { caret } = &result[0] else {
                panic!()
            };
            let glyph = &e.frame().glyphs[0];
            // 30 pt = 381000 EMU; GDEF caret 173 / 1000 em from actual glyph.
            assert_eq!(
                caret.caret.edge.x,
                glyph.origin.x.checked_add(f(65913)).unwrap()
            );
            assert_eq!(
                caret.caret.edge.top,
                glyph.origin.y.checked_sub(f(304800)).unwrap()
            );
            assert_eq!(caret.visible, Some(caret.caret.edge));
            let hit = e
                .query(
                    &[FrameTextQuery::Hit {
                        point: Point {
                            x: caret.caret.edge.x,
                            y: caret.caret.edge.top.checked_add(f(100)).unwrap(),
                        },
                    }],
                    &|| false,
                )
                .unwrap();
            let FrameTextQueryResult::Hit { caret: hit, inside } = &hit[0] else {
                panic!()
            };
            assert!(*inside);
            assert_eq!(hit.caret.position.scalar_offset, 1);
        }
    }
}
#[test]
fn cross_paragraph_reverse_selection_keeps_endpoints_and_structural_separators() {
    let runs = colored("AA", "123456") + "</a:p><a:p>" + &colored("A", "123456");
    let e = editor(&shape(42, 0, 0, "", &runs), Default::default());
    let result = e
        .query(
            &[FrameTextQuery::Selection {
                anchor: position(1, 1),
                focus: position(0, 1),
            }],
            &|| false,
        )
        .unwrap();
    let FrameTextQueryResult::Selection {
        anchor,
        focus,
        fragments,
        paragraph_breaks,
    } = &result[0]
    else {
        panic!()
    };
    assert_eq!(anchor.paragraph, 1);
    assert_eq!(focus.paragraph, 0);
    assert_eq!(paragraph_breaks, &[0]);
    assert_eq!(fragments.len(), 2);
    assert_eq!(
        (
            fragments[0].paragraph,
            fragments[0].fragment.start.scalar_offset
        ),
        (0, 1)
    );
    assert_eq!(
        (
            fragments[1].paragraph,
            fragments[1].fragment.start.scalar_offset
        ),
        (1, 0)
    );
    assert!(fragments[1].fragment.bounds.min.y > fragments[0].fragment.bounds.min.y);
}
#[test]
fn native_axis_clips_apply_to_carets_selections_and_hit_inside_state() {
    for (attrs, h, v) in [
        ("horzOverflow=\"clip\"", true, false),
        ("vertOverflow=\"clip\"", false, true),
        ("horzOverflow=\"clip\" vertOverflow=\"clip\"", true, true),
    ] {
        let s = shape(
            42,
            0,
            0,
            "",
            &(colored("AAAA", "123456") + "<a:br/>" + &colored("AAAA", "123456")),
        )
        .replace(
            "cx=\"1000000\" cy=\"600000\"",
            "cx=\"240000\" cy=\"240000\"",
        )
        .replace("<a:bodyPr ", &format!("<a:bodyPr wrap=\"none\" {attrs} "));
        let e = editor(&s, Default::default());
        let result = e
            .query(
                &[
                    FrameTextQuery::Selection {
                        anchor: position(0, 0),
                        focus: position(0, 9),
                    },
                    FrameTextQuery::Caret {
                        position: position(0, 9),
                    },
                ],
                &|| false,
            )
            .unwrap();
        let FrameTextQueryResult::Selection { fragments, .. } = &result[0] else {
            panic!()
        };
        assert!(fragments.iter().any(|f| f.visible.is_none()));
        for fragment in fragments.iter().filter_map(|f| f.visible) {
            if h {
                assert!(fragment.min.x >= f(0) && fragment.max.x <= f(240000));
            }
            if v {
                assert!(fragment.min.y >= f(0) && fragment.max.y <= f(240000));
            }
        }
        let FrameTextQueryResult::Caret { caret } = &result[1] else {
            panic!()
        };
        assert!(caret.visible.is_none());
        let result = e
            .query(
                &[FrameTextQuery::Hit {
                    point: Point {
                        x: caret.caret.edge.x,
                        y: caret.caret.edge.top.checked_add(f(100)).unwrap(),
                    },
                }],
                &|| false,
            )
            .unwrap();
        let FrameTextQueryResult::Hit { inside, .. } = &result[0] else {
            panic!()
        };
        assert!(!inside);
    }
}

fn navigation(
    e: &SourceFrameEditor,
    p: FrameTextPosition,
    movement: mo_text::interaction::CaretMove,
    preferred_x: Option<Fixed>,
) -> (FrameCaret, Option<Fixed>, bool) {
    let mut r = e
        .query(
            &[FrameTextQuery::Move {
                position: p,
                movement,
                preferred_x,
            }],
            &|| false,
        )
        .unwrap();
    let FrameTextQueryResult::Moved {
        caret,
        preferred_x,
        exhausted,
    } = r.remove(0)
    else {
        panic!()
    };
    (caret, preferred_x, exhausted)
}
fn at(c: &FrameCaret) -> FrameTextPosition {
    FrameTextPosition {
        paragraph: c.paragraph,
        position: c.caret.position,
    }
}
#[test]
fn vertical_navigation_preserves_frame_x_across_alignment_and_empty_paragraphs() {
    use mo_text::interaction::CaretMove::*;
    let runs = colored("AAAA", "123456")
        + "</a:p><a:p><a:pPr algn=\"r\"/>"
        + &colored("A", "123456")
        + "</a:p><a:p></a:p><a:p>"
        + &colored("AAAA", "123456");
    let e = editor(&shape(42, 0, 0, "", &runs), Default::default());
    let (start, _, _) = navigation(&e, position(0, 1), LineStart, None);
    let (start, _, _) = navigation(&e, at(&start), NextGrapheme, None);
    let x = start.caret.edge.x;
    let (r, sticky, exhausted) = navigation(&e, at(&start), Down, None);
    assert_eq!((r.paragraph, r.caret.position.scalar_offset), (1, 0));
    assert!(!exhausted);
    assert_eq!(sticky, Some(x));
    assert!(r.caret.edge.x > x);
    let (r, sticky, _) = navigation(&e, at(&r), Down, sticky);
    assert_eq!((r.paragraph, r.caret.position.scalar_offset), (2, 0));
    assert_eq!(sticky, Some(x));
    let (r, sticky, _) = navigation(&e, at(&r), Down, sticky);
    assert_eq!((r.paragraph, r.caret.position.scalar_offset), (3, 1));
    assert_eq!(r.caret.edge.x, x);
    assert!(navigation(&e, at(&r), Down, sticky).2);
    let (r, sticky, _) = navigation(&e, at(&r), Up, sticky);
    let (r, sticky, _) = navigation(&e, at(&r), Up, sticky);
    let (r, _, _) = navigation(&e, at(&r), Up, sticky);
    assert_eq!((r.paragraph, r.caret.position.scalar_offset), (0, 1));
    assert_eq!(r.caret.edge.x, x);
}
#[test]
fn navigation_crosses_paragraph_separators_and_respects_each_target_direction() {
    use mo_text::interaction::CaretMove::*;
    let runs = colored("A", "123456")
        + "</a:p><a:p><a:pPr rtl=\"1\"/>"
        + &colored("אבג", "123456")
        + "</a:p><a:p></a:p><a:p>"
        + &colored("A", "123456");
    let e = editor(
        &shape(42, 0, 0, "", &runs).replace(
            &format!("<a:latin typeface=\"{FONT}\"/>"),
            &format!("<a:latin typeface=\"{FONT}\"/><a:cs typeface=\"{FONT}\"/>"),
        ),
        Default::default(),
    );
    assert_eq!(e.paragraphs()[1].paragraph_level, 1);
    let (r, _, _) = navigation(&e, position(0, 1), Right, None);
    assert_eq!((r.paragraph, r.caret.position.scalar_offset), (1, 0));
    let (r, _, _) = navigation(&e, at(&r), Right, None);
    assert_eq!((r.paragraph, r.caret.position.scalar_offset), (0, 1));
    let (r, _, _) = navigation(&e, position(1, 3), Left, None);
    assert_eq!((r.paragraph, r.caret.position.scalar_offset), (2, 0));
    let (r, _, _) = navigation(&e, at(&r), Right, None);
    assert_eq!((r.paragraph, r.caret.position.scalar_offset), (3, 0));
    let (r, _, _) = navigation(&e, at(&r), PreviousGrapheme, None);
    assert_eq!((r.paragraph, r.caret.position.scalar_offset), (2, 0));
    let (r, _, _) = navigation(&e, at(&r), PreviousGrapheme, None);
    assert_eq!((r.paragraph, r.caret.position.scalar_offset), (1, 3));
    let (r, _, _) = navigation(&e, at(&r), TextEnd, None);
    assert_eq!((r.paragraph, r.caret.position.scalar_offset), (3, 1));
    assert!(navigation(&e, at(&r), NextGrapheme, None).2);
    let (r, _, _) = navigation(&e, at(&r), TextStart, None);
    assert_eq!((r.paragraph, r.caret.position.scalar_offset), (0, 0));
    assert!(navigation(&e, at(&r), PreviousGrapheme, None).2);
}
#[test]
fn frame_navigation_rejects_invalid_sticky_coordinates_limits_and_cancellation() {
    use mo_text::interaction::CaretMove::*;
    use std::cell::Cell;
    let s = shape(
        42,
        0,
        0,
        "",
        &(colored("AAAA", "123456") + "</a:p><a:p>" + &colored("AAAA", "123456")),
    );
    let e = editor(&s, Default::default());
    let request = |movement, preferred_x| FrameTextQuery::Move {
        position: position(0, 1),
        movement,
        preferred_x,
    };
    assert!(e.query(&[request(Right, Some(f(1)))], &|| false).is_err());
    assert!(
        e.query(
            &[FrameTextQuery::Move {
                position: position(4, 0),
                movement: Down,
                preferred_x: None
            }],
            &|| false
        )
        .is_err()
    );
    let queries = [
        request(Down, None),
        request(Left, None),
        request(TextEnd, None),
    ];
    let count = Cell::new(0);
    e.query(&queries, &|| {
        count.set(count.get() + 1);
        false
    })
    .unwrap();
    for stop in 1..=count.get() {
        let calls = Cell::new(0);
        assert!(
            e.query(&queries, &|| {
                calls.set(calls.get() + 1);
                calls.get() == stop
            })
            .is_err()
        );
    }
    let e = editor(
        &s,
        FrameInteractionLimits {
            max_query_work: 1,
            ..Default::default()
        },
    );
    assert!(matches!(
        e.query(&queries, &|| false),
        Err(SourceFrameError::Limit("frame interaction query work"))
    ));
}
