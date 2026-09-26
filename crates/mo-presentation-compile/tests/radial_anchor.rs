#[allow(dead_code)]
#[path = "../../../tools/test-support/radial_layout.rs"]
mod support;
use mo_geometry::Fixed;
use mo_pptx::source::fill::resolve::{FillProfile, FillTarget, SourceFillQuery};
use mo_presentation_compile::radial_layout::*;
use support::*;

fn options() -> RadialLayoutOptions {
    RadialLayoutOptions {
        coordinate_tolerance: Fixed::from_raw(1 << 24),
    }
}
fn query(index: &mo_pptx::source::SourceIndex, targets: Vec<FillTarget>) -> SourceFillQuery {
    SourceFillQuery {
        expected_source_sha256: index.source_sha256.clone(),
        surface: SLIDE.into(),
        targets,
        profile: FillProfile::Drawingml2024DraftV1,
    }
}
fn evaluated(rect: &str) -> NativeRadialLayout {
    let index = read(&image_fixture(&receiver(
        42,
        [0, 0, 1200000, 600000],
        "",
        &circle(rect),
    )));
    layout_source_with_basis(
        &index,
        &query(&index, vec![FillTarget::Object { native_id: 42 }]),
        options(),
        RadialLayoutLimits::default(),
        CircleFocusBasis::AnchorRectangle,
        &|| false,
    )
    .unwrap()
    .remove(0)
    .layout
}
fn contains(e: &RadialEstimate<2>, expected: [i128; 2]) {
    for (i, v) in expected.into_iter().enumerate() {
        assert!((e.values[i].raw() - (v << 32)).abs() <= e.errors[i].raw());
    }
}
#[test]
fn anchor_focus_separates_position_from_circle_scale_and_keeps_exact_degeneracy() {
    let point = evaluated("l=\"80%\" t=\"20%\" r=\"20%\" b=\"80%\"");
    contains(&point.focus_point, [960000, 120000]);
    contains(&point.inner_center, [960000, 120000]);
    assert_eq!(point.inner_radii.values, [Fixed::ZERO; 2]);
    let area = evaluated("l=\"10%\" t=\"25%\" r=\"60%\" b=\"15%\"");
    contains(&area.inner_center, [300000, 330000]);
    let same = evaluated("l=\"10%\" t=\"25%\" r=\"-10%\" b=\"25%\"");
    contains(&same.focus_point, [120000, 300000]);
    contains(&same.inner_center, [600000, 300000]);
    let near = evaluated("l=\"10%\" t=\"25%\" r=\"-9.999999999999999999%\" b=\"25%\"");
    contains(&near.inner_center, [720000, 300000]);
    assert!(near.focus_point.values[0].raw() > 1i128 << 100);
    // Target WPS rounds this near-degeneracy to equal extent. This profile
    // deliberately preserves exact source classification pending calibration.
    assert_ne!(same.inner_center.values, near.inner_center.values);
}
#[test]
fn anchor_focus_corpus_retains_bindings_and_original_profile() {
    for case in radial_cases() {
        let index = read(&case.source);
        let before = serde_json::to_value(&index).unwrap();
        let q = query(&index, case.targets);
        let original = layout_source(
            &index,
            &q,
            options(),
            RadialLayoutLimits::default(),
            &|| false,
        )
        .unwrap();
        let plans = layout_source_with_basis(
            &index,
            &q,
            options(),
            RadialLayoutLimits::default(),
            CircleFocusBasis::AnchorRectangle,
            &|| false,
        )
        .unwrap();
        assert_eq!(serde_json::to_value(&index).unwrap(), before);
        for (a, b) in original.iter().zip(&plans) {
            assert_eq!(a.layout.profile, PROFILE);
            assert_eq!(b.layout.profile, ANCHOR_FOCUS_PROFILE);
            assert_eq!(a.layout.path_bounds, b.layout.path_bounds);
            assert_eq!(a.layout.tile_rectangle, b.layout.tile_rectangle);
            assert_eq!(a.layout.outer_radius, b.layout.outer_radius);
            assert_eq!(a.layout.inner_radii, b.layout.inner_radii);
            assert_eq!(
                serde_json::to_value(&a.style).unwrap(),
                serde_json::to_value(&b.style).unwrap()
            );
            assert_eq!(
                serde_json::to_value(&a.placement).unwrap(),
                serde_json::to_value(&b.placement).unwrap()
            );
        }
        if let Some(dir) = std::env::var_os("MO_RADIAL_ANCHOR_DIR") {
            let dir = std::path::Path::new(&dir);
            assert!(dir.is_absolute());
            std::fs::create_dir_all(dir).unwrap();
            for (ext, bytes) in [
                ("pptx", case.source),
                ("query.json", serde_json::to_vec_pretty(&q).unwrap()),
                ("plan.json", serde_json::to_vec_pretty(&plans).unwrap()),
            ] {
                std::fs::write(dir.join(format!("{}.{ext}", case.name)), bytes).unwrap();
            }
        }
    }
}
