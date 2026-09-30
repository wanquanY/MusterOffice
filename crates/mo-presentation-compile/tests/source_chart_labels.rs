#[path = "../../../tools/test-support/chart_labels.rs"]
mod support;
use mo_opc::Package;
use mo_presentation_compile::source_chart_labels::*;
use mo_presentation_source::source::inspect_source;
use serde_json::{Value, json};
use support::*;
fn run(
    bytes: &[u8],
    q: Value,
    limits: SourceChartLabelLimits,
    check: &dyn Fn() -> bool,
) -> Result<SourceChartLabels, ChartLabelError> {
    let p = Package::open(bytes, bytes.len() as u64, Default::default(), &|| false).unwrap();
    let index = inspect_source(&p, Default::default(), &|| false).unwrap();
    compute(
        &p,
        &index,
        &serde_json::from_value(q).unwrap(),
        Default::default(),
        limits,
        check,
    )
}
fn by_name(name: &str) -> SourceChartLabels {
    let (_, b, q, _) = cases().into_iter().find(|c| c.0 == name).unwrap();
    run(&b, q, Default::default(), &|| false).unwrap()
}
#[test]
fn numeric_categories_bubble_roles_and_positions_keep_their_native_sources() {
    let result = by_name("numeric-categories");
    for (label, format) in result.labels.iter().zip(["0.0", "0.00", "0.0"]) {
        assert!(
            matches!(&label.components[..], [ChartLabelComponent::Number { role: ChartLabelFlag::CategoryName, format: Some(f), .. }] if f.code == format)
        );
    }
    let result = by_name("bubble-and-series-reference");
    for label in result.labels {
        assert!(matches!(&label.components[..], [
            ChartLabelComponent::Text { role: ChartLabelFlag::SeriesName, value, .. },
            ChartLabelComponent::Text { role: ChartLabelFlag::CategoryName, .. },
            ChartLabelComponent::Number { role: ChartLabelFlag::Value, .. },
            ChartLabelComponent::Number { role: ChartLabelFlag::BubbleSize, format: Some(f), .. }
        ] if value == "收入" && f.code == "0.00"));
    }
    let result = by_name("label-metadata");
    let first = &result.labels[0].settings;
    assert_eq!(
        first.position.as_ref().unwrap().value,
        ChartLabelPosition::OutsideEnd
    );
    assert!(first.show_leader_lines.as_ref().unwrap().value);
    assert!(first.layout_source_ordinal.is_some());
    assert!(result.labels[1].settings.position.is_none());
    assert!(result.labels[1].settings.layout_source_ordinal.is_none());
}
#[test]
fn wide_group_resolves_each_native_point_without_sibling_style_leakage() {
    let (bytes, request) = wide(4096);
    let result = run(&bytes, request, Default::default(), &|| false).unwrap();
    assert_eq!(result.labels.len(), 4096);
    for (idx, label) in result.labels.iter().enumerate() {
        assert_eq!(label.target.point_index as usize, idx);
        assert_eq!(label.annotation_chain.len(), 2);
        if idx % 2 == 0 {
            assert!(label.components.is_empty());
        } else {
            assert!(
                matches!(&label.components[..], [ChartLabelComponent::Number { value, .. }] if value.lexical() == idx.to_string())
            );
        }
    }
}
#[test]
fn point_flags_formats_and_style_roots_override_series_independently() {
    let r = by_name("actual-like");
    assert_eq!(r.labels.len(), 3);
    assert!(r.normalizations.is_empty());
    for (label, value) in r.labels.iter().zip(["48", "32", "20"]) {
        assert!(label.settings.unresolved_flags.is_empty());
        assert!(label.settings.flags[&ChartLabelFlag::Value].value);
        assert!(!label.settings.flags[&ChartLabelFlag::Percent].value);
        assert_eq!(label.settings.text_property_roots.len(), 2);
        assert_eq!(label.annotation_chain.len(), 2);
        let ChartLabelComponent::Number {
            value: n,
            format: Some(format),
            ..
        } = &label.components[0]
        else {
            panic!()
        };
        assert_eq!(n.lexical(), value);
        assert_eq!(format.code, "General");
        assert_eq!(label.legend_key_visible, Some(false));
    }
}
#[test]
fn explicit_false_empty_separator_and_absent_properties_remain_distinct() {
    let inherited = by_name("plot-inheritance");
    assert_eq!(inherited.labels[0].components.len(), 1);
    assert!(!inherited.labels[1].settings.flags[&ChartLabelFlag::Value].value);
    assert!(
        matches!(&inherited.labels[1].components[0],ChartLabelComponent::Text{value,..}if value=="华南")
    );
    let sparse = by_name("unresolved-flags");
    assert_eq!(sparse.labels[0].settings.unresolved_flags.len(), 5);
    assert!(sparse.labels[0].settings.flags[&ChartLabelFlag::Value].schema_defaulted);
    assert_eq!(sparse.labels[0].settings.separator, None);
    let declared = by_name("schema-boolean-default");
    assert!(!declared.labels[0].settings.flags[&ChartLabelFlag::Value].schema_defaulted);
    assert!(matches!(
        by_name("pie-separator").labels[0].settings.separator,
        Some(ChartLabelSeparator::PieCategoryPercentLineBreak {})
    ));
    assert!(matches!(
        by_name("doughnut-separator").labels[0].settings.separator,
        Some(ChartLabelSeparator::CommaDefault {})
    ));
    assert!(
        matches!(&by_name("empty-separator").labels[0].settings.separator,Some(ChartLabelSeparator::Declared{value,..})if value.is_empty())
    );
}
#[test]
fn whole_numfmt_default_attribute_does_not_inherit_lower_source_linked_false() {
    let r = by_name("format-attribute-default");
    let n = r.labels[1].settings.number_format.as_ref().unwrap();
    assert!(n.source_linked.value && n.source_linked.schema_defaulted);
    assert_eq!(n.code, "0.0");
    assert!(
        matches!(&r.labels[1].components[0],ChartLabelComponent::Number{format:Some(f),..}if f.code=="0.000")
    );
    assert!(
        matches!(&r.labels[0].components[0],ChartLabelComponent::Number{format:Some(f),..}if f.code=="General")
    );
}
#[test]
fn percent_normalization_is_exact_reused_and_not_a_rounded_sector_angle() {
    let r = by_name("decimal-ratios");
    assert_eq!(r.normalizations.len(), 1);
    let ratio = &r.normalizations[0].ratios;
    assert_eq!(ratio.denominator, "10");
    assert_eq!(
        ratio
            .numerators
            .iter()
            .map(|v| v.numerator.as_str())
            .collect::<Vec<_>>(),
        ["1", "2", "7"]
    );
    let tiny = by_name("sub-angle-resolution");
    assert_eq!(tiny.normalizations[0].ratios.numerators[0].numerator, "1");
    assert_eq!(
        tiny.normalizations[0].ratios.denominator,
        "900000000000000000000000000000000000000000000000000000000001"
    );
    let abs = by_name("negative-absolute");
    assert_eq!(abs.normalizations[0].ratios.denominator, "10");
    let (_, b, q, _) = cases()
        .into_iter()
        .find(|c| c.0 == "decimal-ratios")
        .unwrap();
    run(
        &b,
        q,
        SourceChartLabelLimits {
            ratios: mo_charts::sectors::SectorLimits {
                max_points: 3,
                ..Default::default()
            },
            ..Default::default()
        },
        &|| false,
    )
    .unwrap();
}
#[test]
fn custom_text_delete_and_legend_key_rules_are_not_flattened() {
    let custom = by_name("custom-rich");
    assert!(custom.labels[0].custom_text_source.is_some());
    assert!(custom.labels[0].components.is_empty());
    assert!(custom.labels[1].custom_text_source.is_none());
    let deleted = by_name("group-delete");
    assert!(!deleted.labels[0].settings.deleted.as_ref().unwrap().value);
    assert_eq!(deleted.labels[0].components.len(), 1);
    assert!(deleted.labels[1].settings.deleted.as_ref().unwrap().value);
    assert!(deleted.labels[1].components.is_empty());
    assert!(
        by_name("legend-key-alone")
            .labels
            .iter()
            .all(|l| l.legend_key_visible == Some(false))
    );
}
#[test]
fn corpus_is_transactional_for_unknown_missing_and_unimplemented_semantics() {
    for (name, b, q, status) in cases() {
        let r = run(&b, q, Default::default(), &|| false);
        assert_eq!(r.is_ok(), status == "planned", "{name}: {r:?}");
    }
}
#[test]
fn cancellation_and_shared_budgets_return_no_partial_plan() {
    let (_, b, q, _) = cases()
        .into_iter()
        .find(|c| c.0 == "decimal-ratios")
        .unwrap();
    for limits in [
        SourceChartLabelLimits {
            max_targets: 2,
            ..Default::default()
        },
        SourceChartLabelLimits {
            max_data_points: 5,
            ..Default::default()
        },
        SourceChartLabelLimits {
            max_retained_bytes: 256,
            ..Default::default()
        },
        SourceChartLabelLimits {
            ratios: mo_charts::sectors::SectorLimits {
                max_points: 2,
                ..Default::default()
            },
            ..Default::default()
        },
    ] {
        assert!(run(&b, q.clone(), limits, &|| false).is_err());
    }
    let count = std::cell::Cell::new(0);
    run(&b, q.clone(), Default::default(), &|| {
        count.set(count.get() + 1);
        false
    })
    .unwrap();
    for limit in [1, 50, 100, count.get() - 1] {
        let current = std::cell::Cell::new(0);
        assert!(
            run(&b, q.clone(), Default::default(), &|| {
                current.set(current.get() + 1);
                current.get() >= limit
            })
            .is_err()
        );
    }
    let s = series(7, &["1", "2", "3"], &group("", &flags(false, false, true)))
        + &series(9, &["3", "2", "1"], &group("", &flags(false, false, true)));
    let b = package(&xml("doughnutChart", &s, ""));
    let mut q = request(&b);
    q["targets"] = json!([{"seriesIndex":7,"pointIndex":0},{"seriesIndex":9,"pointIndex":0}]);
    let r = run(
        &b,
        q.clone(),
        SourceChartLabelLimits {
            ratios: mo_charts::sectors::SectorLimits {
                max_points: 6,
                ..Default::default()
            },
            ..Default::default()
        },
        &|| false,
    )
    .unwrap();
    assert_eq!(r.normalizations.len(), 2);
    assert!(
        run(
            &b,
            q,
            SourceChartLabelLimits {
                ratios: mo_charts::sectors::SectorLimits {
                    max_points: 5,
                    ..Default::default()
                },
                ..Default::default()
            },
            &|| false
        )
        .is_err()
    );
}
