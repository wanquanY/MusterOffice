use super::test_support::*;
use super::*;
use crate::itemize::StyleSpan;
use mo_common::Emu;
use mo_geometry::Fixed;
use std::cell::Cell;
#[test]
fn fractional_baseline_wire_preserves_integer_semantics_and_numeric_equality() {
    let old: BaselineShift = serde_json::from_str("\"1\"").unwrap();
    let exact: BaselineShift = serde_json::from_str(r#"{"q32":"4294967296"}"#).unwrap();
    assert_eq!(old, exact);
    assert_eq!(serde_json::to_string(&old).unwrap(), "\"1\"");
    let fractional: BaselineShift = serde_json::from_str(r#"{"q32":"2147483648"}"#).unwrap();
    assert_eq!(
        fractional.position(),
        Fixed::scale(1, Emu::new(1), 2).unwrap()
    );
    for invalid in [
        r#"{"q32":0.5}"#,
        r#"{"q32":"1","extra":0}"#,
        r#"{"q32":"01"}"#,
        "0.5",
    ] {
        assert!(serde_json::from_str::<BaselineShift>(invalid).is_err());
    }
}
#[test]
fn fractional_baseline_affects_layout_without_splitting_equal_styles() {
    let mut q = request("AA");
    q.shaping
        .paragraph
        .styles
        .push(q.shaping.paragraph.styles[0].clone());
    q.shaping.paragraph.spans = vec![
        StyleSpan { end: 1, style: 0 },
        StyleSpan { end: 2, style: 1 },
    ];
    q.styles.push(q.styles[0]);
    q.styles[1].baseline_shift = BaselineShift::Q32 { q32: Fixed::ZERO };
    let mut b = Backend::default();
    let r = layout_lines(&q, FONT, &mut b, &|| false).unwrap();
    assert_eq!(b.shapes, 1);
    let original_height = r.layout.unwrap().height.get();
    q.styles[1].baseline_shift = BaselineShift::Q32 {
        q32: Fixed::from_raw(3i128 << 31),
    };
    let mut b = Backend::default();
    let r = layout_lines(&q, FONT, &mut b, &|| false).unwrap();
    assert_eq!(b.shapes, 2);
    let layout = r.layout.unwrap();
    assert_eq!(layout.height.get(), original_height + 2);
    assert_eq!(
        layout.lines[0].glyphs[0].y.get() - layout.lines[0].glyphs[1].y.get(),
        2
    );
}
#[test]
fn empty_line_uses_verified_strut_metrics_and_explicit_spacing() {
    let mut q = request("");
    let mut b = Backend::default();
    let r = layout_lines(&q, FONT, &mut b, &|| false).unwrap();
    assert_eq!((b.shapes, b.metrics), (0, 1));
    let layout = r.layout.unwrap();
    assert_eq!(layout.height.get(), 260350);
    assert_eq!(layout.lines[0].baseline.get(), 206375);
    assert!(layout.lines[0].glyphs.is_empty());
    q.spacing = LineSpacing::Exact {
        height: Emu::new(100000),
    };
    let r = layout_lines(&q, FONT, &mut Backend::default(), &|| false).unwrap();
    let line = &r.layout.unwrap().lines[0];
    assert_eq!(line.height.get(), 100000);
    assert_eq!(line.baseline.get(), 126200);
    q.spacing = LineSpacing::AtLeast {
        height: Emu::new(300000),
    };
    assert_eq!(
        layout_lines(&q, FONT, &mut Backend::default(), &|| false)
            .unwrap()
            .layout
            .unwrap()
            .height
            .get(),
        300000
    );
}
#[test]
fn geometry_style_changes_prevent_incorrect_shape_merging_and_reuse_metrics() {
    let mut q = request("AA");
    q.shaping
        .paragraph
        .styles
        .push(q.shaping.paragraph.styles[0].clone());
    q.shaping.paragraph.spans = vec![
        StyleSpan { end: 1, style: 0 },
        StyleSpan { end: 2, style: 1 },
    ];
    q.styles.push(GeometryStyle {
        cluster_spacing: mo_geometry::Fixed::ZERO,
        font_size: Emu::new(508000),
        baseline_shift: Emu::new(10000).into(),
    });
    let mut b = Backend::default();
    let r = layout_lines(&q, FONT, &mut b, &|| false).unwrap();
    assert_eq!(b.shapes, 2);
    assert_eq!(b.metrics, 1);
    assert_eq!(r.metric_instances.len(), 1);
    let line = &r.layout.unwrap().lines[0];
    assert_eq!(line.advance.get(), 457200);
    assert_eq!(line.glyphs[1].x.get(), 152400);
    assert_eq!(line.glyphs[0].y.get() - line.glyphs[1].y.get(), 10000);
}
#[test]
fn scalar_l2_orders_whole_fragments_and_keeps_rtl_glyph_sequence() {
    let q = request("A אב");
    let r = layout_lines(&q, FONT, &mut Backend::default(), &|| false).unwrap();
    let line = &r.layout.as_ref().unwrap().lines[0];
    let clusters: Vec<_> = line
        .glyphs
        .iter()
        .map(|g| {
            let FontFragment::Selected { shaped, .. } = &r.shaping.fallback.items
                [g.source.fallback_item as usize]
                .fragments[g.source.fragment as usize]
            else {
                panic!()
            };
            shaped.runs[0].glyphs[g.glyph as usize].cluster
        })
        .collect();
    assert_eq!(clusters, [0, 1, 3, 2]);
}
#[test]
fn numeric_validation_precedes_component_and_unresolved_layout_is_explicit() {
    let mut invalid = request("A");
    invalid.styles[0].font_size = Emu::ZERO;
    let mut b = Backend::default();
    assert!(layout_lines(&invalid, FONT, &mut b, &|| false).is_err());
    assert_eq!((b.shapes, b.metrics), (0, 0));
    let r = layout_lines(&request("A\t"), FONT, &mut Backend::default(), &|| false).unwrap();
    assert!(r.layout.is_none());
    assert!(matches!(r.issues[0], GeometryIssue::Tab { .. }));
    let r = layout_lines(
        &request("A"),
        FONT,
        &mut Backend {
            metric_missing: true,
            ..Default::default()
        },
        &|| false,
    )
    .unwrap();
    assert!(r.layout.is_none());
    assert_eq!(r.issues.len(), 3);
    let mut big = request("AAA");
    big.styles[0].font_size = Emu::new(i64::MAX);
    assert!(matches!(
        layout_lines(&big, FONT, &mut Backend::default(), &|| false),
        Err(TextError::Limit(_))
    ));
}
#[test]
fn failure_after_shaping_and_cancellation_do_not_return_partial_positions() {
    let q = request("A");
    let mut b = Backend {
        metric_fail: true,
        ..Default::default()
    };
    assert!(matches!(
        layout_lines(&q, FONT, &mut b, &|| false),
        Err(TextError::BackendFailure { status: 2, .. })
    ));
    assert!(b.invalid);
    assert_eq!(b.shapes, 1);
    let total = Cell::new(0);
    layout_lines(&q, FONT, &mut Backend::default(), &|| {
        total.set(total.get() + 1);
        false
    })
    .unwrap();
    for stop in 1..=total.get() {
        let at = Cell::new(0);
        assert!(
            layout_lines(&q, FONT, &mut Backend::default(), &|| {
                at.set(at.get() + 1);
                at.get() == stop
            })
            .is_err()
        );
    }
}

#[test]
fn signed_pen_and_offsets_cross_fragments_without_becoming_advances() {
    let mut q = request("AA");
    q.styles[0].font_size = Emu::new(64000);
    q.styles.push(GeometryStyle {
        cluster_spacing: mo_geometry::Fixed::ZERO,
        font_size: Emu::new(128000),
        baseline_shift: Emu::ZERO.into(),
    });
    q.shaping
        .paragraph
        .styles
        .push(q.shaping.paragraph.styles[0].clone());
    q.shaping.paragraph.spans = vec![
        StyleSpan { end: 1, style: 0 },
        StyleSpan { end: 2, style: 1 },
    ];
    let mut b = Backend {
        glyph_position: Some([-64, 32, 16, -8]),
        ..Default::default()
    };
    let r = layout_lines(&q, FONT, &mut b, &|| false).unwrap();
    let l = &r.layout.unwrap().lines[0];
    assert_eq!(
        (
            l.advance.get(),
            l.advance_y.get(),
            l.pen_min.get(),
            l.pen_max.get()
        ),
        (-192, -96, -192, 0)
    );
    assert_eq!((l.glyphs[0].x.get(), l.glyphs[1].x.get()), (16, -32));
    assert_eq!(
        (
            l.glyphs[0].y.get() - l.baseline.get(),
            l.glyphs[1].y.get() - l.baseline.get()
        ),
        (8, -16)
    );
}
#[test]
fn zero_metrics_are_present_but_need_positive_explicit_line_height() {
    let mut q = request("");
    let mut b = Backend {
        zero_metrics: true,
        ..Default::default()
    };
    let r = layout_lines(&q, FONT, &mut b, &|| false).unwrap();
    assert!(r.layout.is_none());
    assert!(matches!(
        r.issues.as_slice(),
        [GeometryIssue::NonPositiveNaturalHeight { line: 0 }]
    ));
    q.spacing = LineSpacing::Exact {
        height: Emu::new(100),
    };
    let r = layout_lines(&q, FONT, &mut b, &|| false).unwrap();
    assert!(r.issues.is_empty());
    assert_eq!(r.layout.unwrap().lines[0].baseline.get(), 50);
}
