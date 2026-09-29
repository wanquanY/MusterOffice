use super::test_support::Backend;
use super::*;
use crate::geometry::test_support::{FONT, request};
use mo_common::Emu;
use mo_geometry::Fixed;
use std::cell::Cell;
fn q(text: &str) -> ParagraphPathsRequest {
    let g = request(text);
    ParagraphPathsRequest {
        layout: flow::ParagraphLayoutRequest {
            paragraph: g.shaping.paragraph,
            styles: vec![geometry::GeometryStyle {
                cluster_spacing: mo_geometry::Fixed::ZERO,
                font_size: Emu::new(1),
                baseline_shift: Emu::ZERO.into(),
            }],
            strut_style: 0,
            spacing: g.spacing,
            width: Emu::new(10000),
            overflow: flow::OverflowPolicy::KeepUnbreakable,
            hanging_punctuation: crate::flow::HangingPunctuation::None,
            wrapping: flow::LineWrapping::Wrap,
        },
        bounds_tolerance: Fixed::from_raw(1 << 26),
    }
}
#[test]
fn paths_reuse_glyphs_and_origins_preserve_fractional_placement() {
    let mut b = Backend::default();
    let r = paragraph_paths(&q("AAA"), FONT, &mut b, &|| false).unwrap();
    let scene = r.scene.unwrap();
    assert_eq!(scene.paths.len(), 1);
    assert_eq!(scene.glyphs.len(), 3);
    assert_eq!(scene.work.unique_source_glyphs, 1);
    assert_eq!(b.outlines, 1);
    assert_eq!(
        scene.glyphs[1].origin.x,
        Fixed::scale(38400, Emu::new(1), 64000).unwrap()
    );
    assert_ne!(scene.glyphs[1].origin.x, Fixed::emu(Emu::new(1)));
    let ordinary = r.layout.geometry.unwrap().layout.unwrap();
    assert_eq!(ordinary.lines[0].glyphs[1].x.get(), 1);
    let bounds = scene.paths[0].bounds.unwrap();
    assert_eq!(bounds.min.y, Fixed::from_raw(-(1 << 31)));
    assert_eq!(bounds.max.y, Fixed::ZERO);
}
#[test]
fn different_sizes_share_source_outline_but_have_distinct_scaled_paths() {
    let mut q = q("AA");
    q.layout
        .paragraph
        .styles
        .push(q.layout.paragraph.styles[0].clone());
    q.layout.styles.push(geometry::GeometryStyle {
        cluster_spacing: mo_geometry::Fixed::ZERO,
        font_size: Emu::new(2),
        baseline_shift: Emu::new(1).into(),
    });
    q.layout.paragraph.spans = vec![
        itemize::StyleSpan { end: 1, style: 0 },
        itemize::StyleSpan { end: 2, style: 1 },
    ];
    let mut b = Backend::default();
    let r = paragraph_paths(&q, FONT, &mut b, &|| false)
        .unwrap()
        .scene
        .unwrap();
    assert_eq!(b.outlines, 1);
    assert_eq!(r.fonts.len(), 1);
    assert_eq!(r.paths.len(), 2);
    assert_eq!(r.work.unique_source_glyphs, 1);
}
#[test]
fn unresolved_layout_and_outlines_never_publish_a_partial_scene() {
    let mut b = Backend::default();
    let r = paragraph_paths(&q("A\tA"), FONT, &mut b, &|| false).unwrap();
    assert!(r.scene.is_none());
    assert_eq!(b.outlines, 0);
    let mut b = Backend {
        missing: true,
        ..Default::default()
    };
    let r = paragraph_paths(&q("AA"), FONT, &mut b, &|| false).unwrap();
    assert!(r.scene.is_none());
    assert!(matches!(
        r.issues[0],
        PathSceneIssue::OutlineUnavailable { .. }
    ));
    let mut b = Backend {
        failure: true,
        ..Default::default()
    };
    assert!(matches!(
        paragraph_paths(&q("AA"), FONT, &mut b, &|| false),
        Err(TextError::BackendFailure { status: 2, .. })
    ));
    assert!(b.layout.invalid);
}
#[test]
fn every_own_cancel_checkpoint_and_precision_validation_fail_without_scene() {
    let q = q("A A");
    let count = Cell::new(0);
    paragraph_paths(&q, FONT, &mut Backend::default(), &|| {
        count.set(count.get() + 1);
        false
    })
    .unwrap();
    for at in 1..=count.get() {
        let called = Cell::new(0);
        let result = paragraph_paths(&q, FONT, &mut Backend::default(), &|| {
            called.set(called.get() + 1);
            called.get() >= at
        });
        assert!(
            matches!(
                result,
                Err(TextError::Cancelled | TextError::Font(mo_font::FontError::Cancelled))
            ),
            "checkpoint {at}: {result:?}"
        );
    }
    let mut bad = q;
    bad.bounds_tolerance = Fixed::from_raw(255);
    let mut b = Backend::default();
    assert!(paragraph_paths(&bad, FONT, &mut b, &|| false).is_err());
    assert_eq!(b.layout.shapes, 0);
}
