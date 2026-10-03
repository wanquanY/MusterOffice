use super::*;
use crate::{
    flow::OverflowPolicy,
    geometry::{GeometryStyle, LineSpacing},
    scene::test_support::Backend as PathBackend,
};
use mo_common::Emu;
use mo_geometry::Fixed;
use std::cell::Cell;
const STYLE: [GeometryStyle; 1] = [GeometryStyle {
    cluster_spacing: mo_geometry::Fixed::ZERO,
    font_size: Emu::new(100_000),
    baseline_shift: crate::geometry::BaselineShift::Emu(Emu::ZERO),
}];
#[test]
fn prepared_optional_metrics_validate_bindings_axes_and_batch_before_backend() {
    use crate::metrics::{FontMetric, FontMetricsInstance};
    let q = request("A");
    let manifest =
        PreparedManifest::load(&q.manifest, FONT, ManifestLimits::default(), &|| false).unwrap();
    let valid = FontMetricsInstance {
        variations: vec![],
        metrics: vec![FontMetric::UnderlineSize],
    };
    let mut backend = Backend::default();
    assert!(
        manifest
            .measure_instances(99, std::slice::from_ref(&valid), &mut backend, &|| false)
            .is_err()
    );
    assert!(
        manifest
            .measure_instances(0, &vec![valid.clone(); 257], &mut backend, &|| false)
            .is_err()
    );
    let bad_axis = FontMetricsInstance {
        variations: vec![ShapeVariation {
            tag: "xxxx".into(),
            value_16_16: 0,
        }],
        ..valid.clone()
    };
    assert!(
        manifest
            .measure_instances(0, &[bad_axis], &mut backend, &|| false)
            .is_err()
    );
    let duplicate = FontMetricsInstance {
        metrics: vec![FontMetric::UnderlineSize; 2],
        ..valid.clone()
    };
    assert!(
        manifest
            .measure_instances(0, &[duplicate], &mut backend, &|| false)
            .is_err()
    );
    assert!(matches!(
        manifest.measure_instances(0, &[valid], &mut backend, &|| true),
        Err(TextError::Cancelled)
    ));
    assert_eq!(backend.metrics, 0);
    assert!(!backend.invalid);
}
fn input(q: &ManifestParagraphRequest) -> ManifestLayoutInput<'_> {
    ManifestLayoutInput {
        paragraph: q.into(),
        styles: &STYLE,
        strut_style: 0,
        width: Emu::new(120_000),
        spacing: LineSpacing::Natural,
        overflow: OverflowPolicy::EmergencyGrapheme,
    }
}
#[test]
fn precise_line_metrics_are_retained_without_changing_legacy_glyph_paths() {
    let q = request("A A");
    let manifest =
        PreparedManifest::load(&q.manifest, FONT, ManifestLimits::default(), &|| false).unwrap();
    let styles = [GeometryStyle {
        cluster_spacing: mo_geometry::Fixed::ZERO,
        font_size: Emu::new(101),
        baseline_shift: Emu::ZERO.into(),
    }];
    let mut legacy_input = input(&q);
    legacy_input.styles = &styles;
    legacy_input.width = Emu::new(202);
    let old = manifest
        .paragraph_paths(
            legacy_input,
            Fixed::from_raw(1 << 26),
            &mut PathBackend::default(),
            &|| false,
        )
        .unwrap();
    let exact = manifest
        .paragraph_geometry(
            ManifestFlowInput {
                paragraph: (&q).into(),
                styles: &styles,
                strut_style: 0,
                spacing: LineSpacing::Natural,
                widths: flow::LineWidths::uniform(Fixed::emu(Emu::new(202))),
                hanging_punctuation: flow::HangingPunctuation::None,
                wrapping: flow::LineWrapping::Wrap,
                overflow: OverflowPolicy::EmergencyGrapheme,
            },
            Fixed::from_raw(1 << 26),
            &mut PathBackend::default(),
            &|| false,
        )
        .unwrap();
    assert_eq!(
        serde_json::to_value(&old.paths).unwrap(),
        serde_json::to_value(&exact.geometry.paths).unwrap()
    );
    let precise = exact.geometry.precise.unwrap();
    let rounded = old.paths.layout.geometry.unwrap().layout.unwrap();
    assert_eq!(precise.height.wire().unwrap(), rounded.height);
    assert!(precise.height != Fixed::emu(rounded.height));
    for (a, b) in precise.lines.iter().zip(rounded.lines) {
        assert_eq!(a.baseline.wire().unwrap(), b.baseline);
        assert_eq!(a.pen_max.wire().unwrap(), b.pen_max);
    }
}

#[test]
fn a_manifest_reuses_faces_across_shaping_layout_and_paths() {
    let q = request("AA AA");
    let manifest =
        PreparedManifest::load(&q.manifest, FONT, ManifestLimits::default(), &|| false).unwrap();
    for _ in 0..3 {
        let mut backend = PathBackend::default();
        let shaped = manifest
            .shape_paragraph((&q).into(), &mut backend, &|| false)
            .unwrap();
        assert_eq!(shaped.shaping.fallback.verified_faces, 1);
        let layout = manifest
            .layout_paragraph(input(&q), &mut backend, &|| false)
            .unwrap();
        let paths = manifest
            .paragraph_paths(input(&q), Fixed::from_raw(1 << 26), &mut backend, &|| false)
            .unwrap();
        assert_eq!(
            serde_json::to_value(layout.layout).unwrap(),
            serde_json::to_value(&paths.paths.layout).unwrap()
        );
        assert!(paths.paths.scene.is_some());
        assert_eq!(paths.bindings[0].candidate.font, 0);
        assert!(backend.layout.metrics > 0 && backend.outlines > 0);
        assert_eq!(manifest.verified_faces(), 1);
    }
}

#[test]
fn operation_styles_and_geometry_are_checked_even_after_manifest_preparation() {
    let mut q = request("A");
    let author = q.manifest.clone();
    let manifest =
        PreparedManifest::load(&author, FONT, ManifestLimits::default(), &|| false).unwrap();
    let mut backend = PathBackend::default();
    q.styles.push(q.styles[0].clone());
    q.styles[1].features.push(ShapeFeature {
        tag: "wrong".into(),
        value: 1,
        start: 0,
        end: None,
    });
    assert!(
        manifest
            .paragraph_paths(input(&q), Fixed::from_raw(1 << 26), &mut backend, &|| false)
            .is_err()
    );
    q.styles.pop();
    q.styles[0].font_style = FontStyle::Bold;
    assert!(
        manifest
            .layout_paragraph(input(&q), &mut backend, &|| false)
            .is_err()
    );
    q.styles[0].font_style = FontStyle::Regular;
    let mut geometry = input(&q);
    geometry.width = Emu::ZERO;
    assert!(
        manifest
            .layout_paragraph(geometry, &mut backend, &|| false)
            .is_err()
    );
    assert!(
        manifest
            .paragraph_paths(input(&q), Fixed::ZERO, &mut backend, &|| false)
            .is_err()
    );
    assert_eq!(
        backend.layout.shapes + backend.layout.metrics + backend.outlines,
        0
    );
    assert!(!backend.layout.invalid);
}

#[test]
fn cancellation_and_component_failure_publish_no_paths_and_leave_resources_reusable() {
    let q = request("A A");
    let manifest =
        PreparedManifest::load(&q.manifest, FONT, ManifestLimits::default(), &|| false).unwrap();
    let count = Cell::new(0usize);
    manifest
        .paragraph_paths(
            input(&q),
            Fixed::from_raw(1 << 26),
            &mut PathBackend::default(),
            &|| {
                count.set(count.get() + 1);
                false
            },
        )
        .unwrap();
    for stop in 0..count.get() {
        let at = Cell::new(0usize);
        let r = manifest.paragraph_paths(
            input(&q),
            Fixed::from_raw(1 << 26),
            &mut PathBackend::default(),
            &|| {
                let n = at.get();
                at.set(n + 1);
                n == stop
            },
        );
        assert!(matches!(r, Err(TextError::Cancelled)), "checkpoint {stop}");
    }
    let mut failed = PathBackend {
        failure: true,
        ..Default::default()
    };
    assert!(
        manifest
            .paragraph_paths(input(&q), Fixed::from_raw(1 << 26), &mut failed, &|| false)
            .is_err()
    );
    assert!(failed.layout.invalid);
    assert_eq!(manifest.verified_faces(), 1);
    let r = manifest
        .paragraph_paths(
            input(&q),
            Fixed::from_raw(1 << 26),
            &mut PathBackend::default(),
            &|| false,
        )
        .unwrap();
    assert!(r.paths.scene.is_some());
}

#[test]
fn interaction_uses_prepared_fonts_and_exact_asymmetric_flow_widths() {
    use crate::interaction::{Affinity, TextPosition, TextQuery};
    let q = request("A A");
    let manifest =
        PreparedManifest::load(&q.manifest, FONT, ManifestLimits::default(), &|| false).unwrap();
    let input = ManifestFlowInput {
        paragraph: (&q).into(),
        styles: &STYLE,
        strut_style: 0,
        spacing: LineSpacing::Natural,
        widths: flow::LineWidths {
            first: Fixed::emu(Emu::new(120_000)),
            rest: Fixed::emu(Emu::new(60_000)),
        },
        overflow: OverflowPolicy::KeepUnbreakable,
        wrapping: Default::default(),
        hanging_punctuation: Default::default(),
    };
    let queries = [TextQuery::Caret {
        position: TextPosition {
            scalar_offset: 2,
            affinity: Affinity::Downstream,
        },
    }];
    let r = manifest
        .paragraph_interaction(input.clone(), &queries, &mut Backend::default(), &|| false)
        .unwrap();
    let paths = manifest
        .paragraph_geometry(
            input,
            Fixed::from_raw(256),
            &mut PathBackend::default(),
            &|| false,
        )
        .unwrap();
    assert_eq!(
        serde_json::to_value(r.bindings).unwrap(),
        serde_json::to_value(paths.bindings).unwrap()
    );
    assert_eq!(
        serde_json::to_value(&r.interaction.layout).unwrap(),
        serde_json::to_value(paths.geometry.paths.layout).unwrap()
    );
    let map = r.interaction.map.unwrap();
    assert_eq!(map.lines.len(), 2);
    assert_eq!(
        map.caret(TextPosition {
            scalar_offset: 2,
            affinity: Affinity::Downstream
        })
        .unwrap()
        .line,
        1
    );
}
