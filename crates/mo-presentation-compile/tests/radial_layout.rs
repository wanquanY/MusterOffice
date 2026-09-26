#[allow(dead_code)]
#[path = "../../../tools/test-support/radial_layout.rs"]
mod support;
use mo_geometry::Fixed;
use mo_pptx::source::fill::resolve::{FillProfile, FillTarget, SourceFillQuery};
use mo_presentation_compile::radial_layout::*;
use std::cell::Cell;
use support::*;
fn q(index: &mo_pptx::source::SourceIndex, targets: Vec<FillTarget>) -> SourceFillQuery {
    SourceFillQuery {
        expected_source_sha256: index.source_sha256.clone(),
        surface: SLIDE.into(),
        targets,
        profile: FillProfile::Drawingml2024DraftV1,
    }
}
fn opts() -> RadialLayoutOptions {
    RadialLayoutOptions {
        coordinate_tolerance: Fixed::from_raw(1 << 24),
    }
}
fn contains<const N: usize>(e: &RadialEstimate<N>, expected: [i128; N]) {
    for (i, v) in expected.into_iter().enumerate() {
        assert!((e.values[i].raw() - (v << 32)).abs() <= e.errors[i].raw());
    }
}
#[test]
fn radial_source_geometry_uses_path_locus_and_preserves_native_declarations() {
    let mut plans = std::collections::BTreeMap::new();
    for case in radial_cases() {
        let index = read(&case.source);
        let before = serde_json::to_value(&index).unwrap();
        let query = q(&index, case.targets);
        let plan = layout_source(
            &index,
            &query,
            opts(),
            RadialLayoutLimits::default(),
            &|| false,
        )
        .unwrap_or_else(|e| panic!("{}: {e:?}", case.name));
        assert_eq!(before, serde_json::to_value(index).unwrap());
        if let Some(dir) = std::env::var_os("MO_RADIAL_LAYOUT_DIR") {
            let dir = std::path::Path::new(&dir);
            assert!(dir.is_absolute());
            std::fs::create_dir_all(dir).unwrap();
            for (ext, bytes) in [
                ("pptx", case.source),
                ("query.json", serde_json::to_vec_pretty(&query).unwrap()),
                ("plan.json", serde_json::to_vec_pretty(&plan).unwrap()),
            ] {
                std::fs::write(dir.join(format!("{}.{ext}", case.name)), bytes).unwrap();
            }
        }
        plans.insert(case.name, plan);
    }
    contains(
        &plans["quadratic"][0].layout.path_bounds,
        [100000, 100000, 900000, 500000],
    );
    contains(
        &plans["cubic"][0].layout.path_bounds,
        [0, 0, 1000000, 600000],
    );
    contains(
        &plans["multi-path"][0].layout.path_bounds,
        [-100000, -200000, 700000, 500000],
    );
    assert_eq!(
        plans["multi-path"][0].layout.work.paths, 2,
        "move-only outlier has no locus"
    );
    assert!(!plans["stationary"][0].layout.rotate_with_shape);
    assert_ne!(
        plans["equal-width-offset"][0].layout.focus_point.values,
        plans["near-equal-width"][0].layout.focus_point.values
    );
    assert!(
        plans["near-equal-width"][0].layout.focus_point.values[0].raw() > 1i128 << 100,
        "near-equal extent is not treated as zero by an epsilon"
    );
    for (a, b) in plans["group-inherited"]
        .iter()
        .zip(&plans["group-inherited-control"])
    {
        assert_eq!(
            serde_json::to_value(&a.layout).unwrap(),
            serde_json::to_value(&b.layout).unwrap()
        );
    }
    if let Some(dir) = std::env::var_os("MO_RADIAL_LAYOUT_DIR") {
        std::fs::write(
            std::path::Path::new(&dir).join("layout.schema.json"),
            serde_json::to_vec_pretty(&schemars::schema_for!(NativeRadialLayout)).unwrap(),
        )
        .unwrap();
    }
}
#[test]
fn radial_queries_bind_identity_and_share_work_limits_atomically() {
    let fill = circle("l=\"50%\" t=\"50%\" r=\"50%\" b=\"50%\"");
    let a = receiver(42, [0, 0, 600000, 800000], "", &fill);
    let b = receiver(43, [600000, 0, 600000, 800000], "", &fill);
    let index = read(&image_fixture(&(a + &b)));
    let query = q(
        &index,
        vec![
            FillTarget::Object { native_id: 42 },
            FillTarget::Object { native_id: 43 },
        ],
    );
    let plan = layout_source(
        &index,
        &query,
        opts(),
        RadialLayoutLimits::default(),
        &|| false,
    )
    .unwrap();
    contains(&plan[0].layout.outer_radius, [500000]);
    contains(&plan[0].layout.outer_center, [300000, 400000]);
    assert_eq!(plan[0].layout.outer_radius.errors, [Fixed::ZERO]);
    let limited = RadialLayoutLimits {
        max_targets: 1,
        ..Default::default()
    };
    assert!(layout_source(&index, &query, opts(), limited, &|| false).is_err());
    let limited = RadialLayoutLimits {
        paths: mo_presentation_compile::native_paths::NativePathLimits {
            max_paths: 1,
            ..Default::default()
        },
        ..Default::default()
    };
    assert!(layout_source(&index, &query, opts(), limited, &|| false).is_err());
    let limited = RadialLayoutLimits {
        max_bounds_steps: plan[0].layout.work.bounds_steps,
        ..Default::default()
    };
    assert!(layout_source(&index, &query, opts(), limited, &|| false).is_err());
    let mut bad = query.clone();
    bad.targets[1] = bad.targets[0].clone();
    assert!(
        layout_source(&index, &bad, opts(), RadialLayoutLimits::default(), &|| {
            false
        })
        .is_err()
    );
    let mut bad = query.clone();
    bad.surface = "/missing.xml".into();
    assert!(
        layout_source(&index, &bad, opts(), RadialLayoutLimits::default(), &|| {
            false
        })
        .is_err()
    );
    let other = read(&image_fixture(""));
    assert!(
        layout_source(
            &other,
            &query,
            opts(),
            RadialLayoutLimits::default(),
            &|| false
        )
        .is_err()
    );
}
#[test]
fn radial_preflight_rejects_inverted_focus_and_invalid_tiles() {
    for (rect, tile) in [
        ("l=\"80%\" r=\"80%\"", ""),
        ("t=\"50%\" b=\"50.000000000000000001%\"", ""),
        ("", "<a:tileRect l=\"75%\" r=\"25%\"/>"),
    ] {
        let fill = rectangular(rect, tile, STOPS).replace("path=\"rect\"", "path=\"circle\"");
        let index = read(&image_fixture(&receiver(
            42,
            [0, 0, 600000, 800000],
            "",
            &fill,
        )));
        let error = layout_source(
            &index,
            &q(&index, vec![FillTarget::Object { native_id: 42 }]),
            opts(),
            RadialLayoutLimits::default(),
            &|| false,
        )
        .unwrap_err();
        assert!(matches!(error, SourceRadialLayoutError::At { .. }));
    }
}
#[test]
fn radial_cancellation_at_each_checkpoint_never_returns_partial_plans() {
    let fill = circle("l=\"50%\" t=\"50%\" r=\"50%\" b=\"50%\"");
    let shape = receiver(42, [0, 0, 1200000, 600000], "", &fill)
        .replace("<a:prstGeom prst=\"rect\"/>", &quadratic());
    let index = read(&image_fixture(&shape));
    let query = q(&index, vec![FillTarget::Object { native_id: 42 }]);
    let calls = Cell::new(0usize);
    let probe = || {
        calls.set(calls.get() + 1);
        false
    };
    layout_source(
        &index,
        &query,
        opts(),
        RadialLayoutLimits::default(),
        &probe,
    )
    .unwrap();
    let total = calls.get();
    assert!(total > 20);
    for stop in 0..total {
        let seen = Cell::new(0);
        let cancel = || {
            let n = seen.get();
            seen.set(n + 1);
            n >= stop
        };
        assert!(
            layout_source(
                &index,
                &query,
                opts(),
                RadialLayoutLimits::default(),
                &cancel
            )
            .is_err(),
            "checkpoint {stop}/{total}"
        );
    }
    if let Some(dir) = std::env::var_os("MO_RADIAL_LAYOUT_DIR") {
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(
            std::path::Path::new(&dir).join("cancellation.json"),
            format!("{{\"checkpoints\":{total}}}\n"),
        )
        .unwrap();
    }
}
