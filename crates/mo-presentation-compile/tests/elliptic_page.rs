#[allow(dead_code)]
#[path = "../../../tools/test-support/elliptic_page.rs"]
mod support;
use mo_presentation_compile::source_page::*;
use mo_raster::{Brush, GradientField, GradientGeometry};
use support::*;

#[test]
fn page_circle_uses_curve_extrema_and_ignores_move_only_outlier() {
    for (name, width, height) in [
        ("quadratic", 800000f64, 400000f64),
        ("cubic", 1000000., 600000.),
        ("multi-path", 800000., 700000.),
        ("multi-path-visible", 800000., 700000.),
    ] {
        let case = elliptic_cases()
            .into_iter()
            .find(|c| c.name == name)
            .unwrap();
        let i = read(&case.source);
        let before = serde_json::to_value(&i).unwrap();
        let p = compile(&i, &request(&i), &|| false).unwrap();
        assert_eq!(before, serde_json::to_value(&i).unwrap());
        let fields: Vec<_> = p
            .raster
            .scene
            .instances
            .iter()
            .filter_map(|instance| {
                let Brush::Gradient { gradient } = &instance.brush else {
                    return None;
                };
                let GradientGeometry::Plane { field, .. } = &gradient.geometry else {
                    panic!()
                };
                Some(field)
            })
            .collect();
        assert_eq!(
            fields.len(),
            1,
            "{name}: non-painting paths must not create draws"
        );
        let GradientField::Elliptic {
            tile_scale,
            uncertainty,
            ..
        } = fields[0]
        else {
            panic!()
        };
        let norm = width.hypot(height);
        for (axis, expected) in [width / norm, height / norm].into_iter().enumerate() {
            let value = tile_scale[axis].raw() as f64 / 4294967296.;
            let error = uncertainty.as_ref().unwrap()[axis].raw() as f64 / 4294967296.;
            assert!((value - expected).abs() <= error, "{name} axis {axis}");
        }
    }
}

#[test]
fn stationary_circle_reports_the_receiving_object_and_cancellation_is_preserved() {
    let case = elliptic_cases()
        .into_iter()
        .find(|c| c.name == "stationary")
        .unwrap();
    let i = read(&case.source);
    let q = request(&i);
    assert!(
        matches!(compile(&i, &q, &|| false), Err(SourcePageError::AtObject { location, error })
        if location.object == Some(42) && matches!(*error, SourcePageError::Invalid("native stationary gradient orientation required")))
    );
    assert!(matches!(
        compile(&i, &q, &|| true),
        Err(SourcePageError::Raster(mo_raster::RasterError::Cancelled))
    ));
}
