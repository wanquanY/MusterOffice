#[path = "../../../tools/test-support/chart_plot.rs"]
mod support;
use mo_opc::{Package, PackageLimits};
use mo_presentation_compile::{source_chart::SourceCircularError, source_chart_plot::*};
use mo_presentation_source::{PptxError, source::inspect_source};
use serde_json::json;
use support::*;

fn run(
    bytes: &[u8],
    limits: SourceChartPlotLimits,
    check: &dyn Fn() -> bool,
) -> Result<SourceChartPlot, SourceCircularError> {
    let p = Package::open(bytes, bytes.len() as u64, PackageLimits::default(), &|| {
        false
    })
    .unwrap();
    let i = inspect_source(&p, Default::default(), &|| false).unwrap();
    compile(
        &p,
        &i,
        &serde_json::from_value(request(bytes)).unwrap(),
        Default::default(),
        limits,
        check,
    )
}
#[test]
fn source_series_and_point_properties_feed_shared_stroke_and_scene() {
    let bytes = fixture(
        BASE,
        &point(
            1,
            r#"<a:solidFill><a:srgbClr val="D4713D"/></a:solidFill><a:ln w="60000"/>"#,
        ),
        &["48", "32", "20"],
    );
    let before = bytes.clone();
    let p = run(&bytes, Default::default(), &|| false).unwrap();
    let v = serde_json::to_value(&p).unwrap();
    assert_eq!(p.scene.paths.len(), 3);
    assert_eq!(p.scene.instances.len(), 6);
    assert_eq!(v["points"][1]["fill"]["rgba8"], json!([212, 113, 61, 255]));
    assert_eq!(v["points"][1]["line"]["rgba8"], json!([249, 249, 249, 255]));
    assert_eq!(p.points[1].line_geometry.width.value.get(), 60000);
    assert_eq!(
        v["points"][1]["lineGeometry"]["width"]["declaredBy"]["kind"],
        "chart"
    );
    assert_ne!(
        v["points"][1]["lineGeometry"]["width"]["declaredBy"],
        v["points"][0]["lineGeometry"]["width"]["declaredBy"]
    );
    assert_eq!(v["points"][1]["line"], v["points"][0]["line"]);
    assert_eq!(
        p.scene.instances[3].stroke.unwrap().width.raw(),
        60000i128 << 32
    );
    assert_eq!(bytes, before);
}
#[test]
fn native_declared_style_corpus_is_admitted_or_diagnosed_without_dropping_features() {
    for (name, bytes, status) in cases() {
        let result = run(&bytes, Default::default(), &|| false);
        assert_eq!(
            if result.is_ok() { "compiled" } else { "error" },
            status,
            "{name}: {result:?}"
        );
        if name == "zero-point" {
            let p = result.unwrap();
            assert!(p.points[0].path.is_none());
            assert_eq!(p.scene.instances.len(), 4);
        } else if name == "full-ring" {
            let p = result.unwrap();
            assert_eq!(p.scene.paths.len(), 1);
            assert_eq!(p.scene.instances.len(), 2);
        } else if name == "all-zero" {
            let p = result.unwrap();
            assert!(p.scene.paths.is_empty());
            assert_eq!(p.points.len(), 3);
        }
    }
}
#[test]
fn empty_effect_list_blocks_series_shadow_and_no_fill_blocks_inherited_paint() {
    let style=BASE.replace("<a:effectLst/>",r#"<a:effectLst><a:outerShdw blurRad="100"><a:srgbClr val="000000"/></a:outerShdw></a:effectLst>"#);
    let points = (0..3)
        .map(|i| point(i, "<a:noFill/><a:ln><a:noFill/></a:ln><a:effectLst/>"))
        .collect::<String>();
    let p = run(
        &fixture(&style, &points, &["1", "2", "3"]),
        Default::default(),
        &|| false,
    )
    .unwrap();
    assert!(p.scene.instances.is_empty());
    assert_eq!(p.scene.paths.len(), 3);
    assert!(
        p.points
            .iter()
            .all(|p| matches!(p.fill, ChartPlotPaint::None { .. })
                && matches!(p.line, ChartPlotPaint::None { .. }))
    );
}
#[test]
fn type_gated_fill_merge_does_not_inherit_a_different_paint_type() {
    let style = BASE.replace(
        r#"<a:solidFill><a:srgbClr val="2468AC"/></a:solidFill>"#,
        "<a:noFill/>",
    );
    let bytes = fixture(&style, &point(1, "<a:solidFill/>"), &["1", "2", "3"]);
    assert!(matches!(
        run(&bytes, Default::default(), &|| false),
        Err(SourceCircularError::Unresolved {
            point_index: Some(1),
            ..
        })
    ));
}
#[test]
fn budgets_and_cancellation_are_shared_across_the_whole_plot() {
    let bytes = fixture(BASE, "", &["1", "2", "3"]);
    for kind in 0..5 {
        let mut limits = SourceChartPlotLimits::default();
        match kind {
            0 => limits.max_draws = 5,
            1 => limits.styles.max_points = 2,
            2 => limits.styles.fills.max_steps = 1,
            3 => limits.styles.lines.max_values = 20,
            _ => limits.styles.lines.max_lexical_bytes = 1,
        }
        assert!(
            matches!(
                run(&bytes, limits, &|| false),
                Err(SourceCircularError::Source(PptxError::Limit(_)))
            ),
            "{kind}"
        );
    }
    let calls = std::cell::Cell::new(0usize);
    run(&bytes, Default::default(), &|| {
        calls.set(calls.get() + 1);
        false
    })
    .unwrap();
    let total = calls.get();
    assert!(total > 50);
    for cutoff in [0, total / 3, total * 2 / 3, total - 1] {
        calls.set(0);
        assert!(
            run(&bytes, Default::default(), &|| {
                let v = calls.get();
                calls.set(v + 1);
                v >= cutoff
            })
            .is_err()
        );
    }
}

#[test]
fn point_modifiers_and_marker_markup_cannot_disappear_during_plot_compilation() {
    for markup in [
        r#"<c:invertIfNegative val="1"/>"#,
        r#"<c:marker><c:symbol val="circle"/></c:marker>"#,
        r#"<c:pictureOptions><c:applyToFront val="1"/></c:pictureOptions>"#,
        r#"<c:extLst><c:ext uri="owned"/></c:extLst>"#,
    ] {
        let local = point(1, "").replace("<c:spPr>", &format!("{markup}<c:spPr>"));
        assert!(
            matches!(
                run(
                    &fixture(BASE, &local, &["1", "2", "3"]),
                    Default::default(),
                    &|| false
                ),
                Err(SourceCircularError::Unresolved { .. })
            ),
            "{markup}"
        );
    }
}
