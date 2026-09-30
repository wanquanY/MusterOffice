use super::*;
pub(super) const FLAGS: [(ChartPropertyKind, ChartLabelFlag); 6] = [
    (ChartPropertyKind::ShowLegendKey, ChartLabelFlag::LegendKey),
    (ChartPropertyKind::ShowValue, ChartLabelFlag::Value),
    (
        ChartPropertyKind::ShowCategoryName,
        ChartLabelFlag::CategoryName,
    ),
    (
        ChartPropertyKind::ShowSeriesName,
        ChartLabelFlag::SeriesName,
    ),
    (ChartPropertyKind::ShowPercent, ChartLabelFlag::Percent),
    (
        ChartPropertyKind::ShowBubbleSize,
        ChartLabelFlag::BubbleSize,
    ),
];
fn boolean(value: Option<&str>, ordinal: u32) -> Result<ChartLabelValue<bool>, ChartLabelError> {
    let v = match value.map(str::trim) {
        None | Some("true" | "1") => true,
        Some("false" | "0") => false,
        _ => return Err(unresolved(ordinal, None, "invalid label boolean")),
    };
    Ok(ChartLabelValue {
        value: v,
        source_ordinal: ordinal,
        schema_defaulted: value.is_none(),
    })
}
pub(super) fn visible(settings: &ChartLabelSettings, flag: ChartLabelFlag) -> bool {
    settings.flags.get(&flag).is_some_and(|v| v.value)
}
pub(super) fn resolve(
    chain: &[&prepared::Declaration<'_>],
    plot: &SourceChartPlot,
    chart_defaults: &[u32],
    budget: &mut Budget<'_>,
) -> Result<ChartLabelSettings, ChartLabelError> {
    let mut result = ChartLabelSettings {
        deleted: None,
        flags: BTreeMap::new(),
        unresolved_flags: vec![],
        position: None,
        separator: None,
        number_format: None,
        show_leader_lines: None,
        text_property_roots: vec![],
        shape_property_roots: vec![],
        layout_source_ordinal: None,
    };
    for prepared in chain {
        budget.check()?;
        let n = prepared.node;
        let d = &n.declarations;
        if let Some(ordinal) = d.retained_attribute_ordinals.first() {
            return Err(unresolved(*ordinal, None, "unknown label attribute"));
        }
        if let Some(child) = d.unrecognized_children.first() {
            return Err(unresolved(
                child.source_ordinal,
                None,
                "unknown label content",
            ));
        }
        let plot_group = n.parent_ordinal == plot.source_ordinal;
        if plot_group && prepared.requires_series_parent {
            return Err(unresolved(
                n.source_ordinal,
                None,
                "Office label declaration requires a series parent",
            ));
        }
        for p in &d.properties {
            if let Some((_, flag)) = FLAGS.iter().find(|(k, _)| *k == p.kind) {
                // Validate shadowed declarations too; invalid input is not a fallback.
                let v = boolean(p.value.as_deref(), p.source_ordinal)?;
                result.flags.entry(*flag).or_insert(v);
            } else {
                match p.kind {
                    ChartPropertyKind::Delete => {
                        let v = boolean(p.value.as_deref(), p.source_ordinal)?;
                        result.deleted.get_or_insert(v);
                    }
                    ChartPropertyKind::ShowLeaderLines => {
                        let v = boolean(p.value.as_deref(), p.source_ordinal)?;
                        result.show_leader_lines.get_or_insert(v);
                    }
                    ChartPropertyKind::Separator => {
                        let value = p.value.as_ref().ok_or_else(|| {
                            unresolved(p.source_ordinal, None, "missing separator character data")
                        })?;
                        if result.separator.is_none() {
                            budget.bytes(value.len())?;
                            result.separator = Some(ChartLabelSeparator::Declared {
                                value: value.clone(),
                                source_ordinal: p.source_ordinal,
                            });
                        }
                    }
                    ChartPropertyKind::LabelPosition => {
                        use ChartLabelPosition as P;
                        let value = match p.value.as_deref() {
                            Some("bestFit") => P::BestFit,
                            Some("b") => P::Bottom,
                            Some("ctr") => P::Center,
                            Some("inBase") => P::InsideBase,
                            Some("inEnd") => P::InsideEnd,
                            Some("l") => P::Left,
                            Some("outEnd") => P::OutsideEnd,
                            Some("r") => P::Right,
                            Some("t") => P::Top,
                            _ => {
                                return Err(unresolved(
                                    p.source_ordinal,
                                    None,
                                    "invalid label position",
                                ));
                            }
                        };
                        result.position.get_or_insert(ChartLabelValue {
                            value,
                            source_ordinal: p.source_ordinal,
                            schema_defaulted: false,
                        });
                    }
                    _ => {
                        return Err(unresolved(
                            p.source_ordinal,
                            None,
                            "unexpected label property",
                        ));
                    }
                }
            }
        }
        if let Some(f) = &n.number_format {
            let code = f.format_code.as_ref().ok_or_else(|| {
                unresolved(f.source_ordinal, None, "missing label number format code")
            })?;
            let linked = boolean(f.source_linked.as_deref(), f.source_ordinal)?;
            if result.number_format.is_none() {
                budget.bytes(code.len())?;
                result.number_format = Some(ChartLabelNumberFormat {
                    source_ordinal: f.source_ordinal,
                    code: code.clone(),
                    source_linked: linked,
                });
            }
        }
        for m in &prepared.markup {
            match m.kind {
                ChartMarkupKind::TextProperties => {
                    result.text_property_roots.push(m.source_ordinal)
                }
                ChartMarkupKind::ShapeProperties => {
                    result.shape_property_roots.push(m.source_ordinal)
                }
                ChartMarkupKind::Layout => {
                    result.layout_source_ordinal.get_or_insert(m.source_ordinal);
                }
                ChartMarkupKind::TextSource
                | ChartMarkupKind::DataLabel
                | ChartMarkupKind::LeaderLines => {}
                _ => {
                    return Err(unresolved(
                        m.source_ordinal,
                        None,
                        "label extension or unsupported markup",
                    ));
                }
            }
        }
    }
    result.text_property_roots.extend_from_slice(chart_defaults);
    result.unresolved_flags = FLAGS
        .iter()
        .map(|(_, f)| *f)
        .filter(|f| !result.flags.contains_key(f))
        .collect();
    if result.separator.is_none() && result.unresolved_flags.is_empty() {
        let only_category_percent = visible(&result, ChartLabelFlag::CategoryName)
            && visible(&result, ChartLabelFlag::Percent)
            && ![
                ChartLabelFlag::Value,
                ChartLabelFlag::SeriesName,
                ChartLabelFlag::BubbleSize,
                ChartLabelFlag::LegendKey,
            ]
            .into_iter()
            .any(|f| visible(&result, f));
        result.separator = Some(
            if matches!(
                plot.native_kind.as_str(),
                "pieChart" | "pie3DChart" | "ofPieChart"
            ) && only_category_percent
            {
                ChartLabelSeparator::PieCategoryPercentLineBreak {}
            } else {
                ChartLabelSeparator::CommaDefault {}
            },
        );
    }
    Ok(result)
}
