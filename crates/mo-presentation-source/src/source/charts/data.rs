use super::*;
use std::collections::BTreeSet;
fn channel(
    tree: &tree::Tree,
    node: usize,
    role: ChartChannelRole,
    budget: &mut Budget,
    check: &dyn Fn() -> bool,
) -> Result<SourceChartChannel, PptxError> {
    let parent = &tree.nodes[node];
    let choices: Vec<_> = parent
        .children
        .iter()
        .copied()
        .filter(|i| {
            let n = &tree.nodes[*i].element.name;
            n.namespace == C
                && matches!(
                    n.local.as_str(),
                    "strRef" | "numRef" | "multiLvlStrRef" | "strLit" | "numLit" | "v"
                )
        })
        .collect();
    if choices.len() != 1 {
        return Err(invalid("chart data channel requires exactly one source"));
    }
    let value = choices[0];
    let kind = tree.nodes[value].element.name.local.clone();
    let formula = if kind.ends_with("Ref") {
        Some(
            tree.value(value, "f")?
                .ok_or_else(|| invalid("chart reference missing formula"))?,
        )
    } else {
        None
    };
    let (cache_kind, cache_node) = match kind.as_str() {
        "strRef" => (ChartCacheKind::String, tree.optional(value, "strCache")?),
        "numRef" => (ChartCacheKind::Number, tree.optional(value, "numCache")?),
        "multiLvlStrRef" => (
            ChartCacheKind::MultiLevelString,
            tree.optional(value, "multiLvlStrCache")?,
        ),
        "strLit" => (ChartCacheKind::String, Some(value)),
        "numLit" => (ChartCacheKind::Number, Some(value)),
        "v" => (ChartCacheKind::String, None),
        _ => unreachable!(),
    };
    if kind == "v" && !tree.nodes[value].children.is_empty() {
        return Err(invalid("chart title scalar has child elements"));
    }
    let cache = cache_node
        .map(|n| cache(tree, n, cache_kind, budget, check))
        .transpose()?;
    Ok(SourceChartChannel {
        source_ordinal: parent.ordinal,
        role,
        native_kind: kind.clone(),
        formula,
        literal_text: (kind == "v").then(|| tree.nodes[value].text.clone()),
        cache,
    })
}
fn cache(
    tree: &tree::Tree,
    node: usize,
    kind: ChartCacheKind,
    budget: &mut Budget,
    check: &dyn Fn() -> bool,
) -> Result<SourceChartCache, PptxError> {
    let declared_point_count = tree
        .optional(node, "ptCount")?
        .map(|i| number(&tree.nodes[i].element, "val"))
        .transpose()?;
    let level_nodes = if kind == ChartCacheKind::MultiLevelString {
        tree.children(node, "lvl").collect()
    } else {
        vec![node]
    };
    let mut levels = vec![];
    for level in level_nodes {
        cancelled(check)?;
        let mut indices = BTreeSet::new();
        let mut points = vec![];
        for i in tree.children(level, "pt") {
            cancelled(check)?;
            budget.points += 1;
            if budget.points > budget.limits.max_points {
                return Err(PptxError::Limit("chart cache points"));
            }
            let index = number(&tree.nodes[i].element, "idx")?;
            if !indices.insert(index) {
                return Err(invalid("duplicate chart point index"));
            }
            if declared_point_count.is_some_and(|count| index >= count) {
                return Err(invalid("chart point index exceeds declared count"));
            }
            points.push(SourceChartPoint {
                source_ordinal: tree.nodes[i].ordinal,
                index,
                value: tree.value(i, "v")?,
                format_code: tree.nodes[i]
                    .element
                    .attribute("formatCode")
                    .map(str::to_owned),
            });
        }
        levels.push(points);
    }
    Ok(SourceChartCache {
        source_ordinal: tree.nodes[node].ordinal,
        kind,
        declared_point_count,
        format_code: tree.value(node, "formatCode")?,
        levels,
    })
}
pub(super) fn read(
    package: &dyn PackageRead,
    part: &PartName,
    tree: &tree::Tree,
    budget: &mut Budget,
    check: &dyn Fn() -> bool,
) -> Result<(Vec<SourceChartPlot>, Option<SourceChartExternalData>), PptxError> {
    if !tree.nodes[0].element.name.is(C, "chartSpace") {
        return Err(invalid("chart root mismatch"));
    }
    let chart = tree.required(0, "chart")?;
    let area = tree.required(chart, "plotArea")?;
    let mut plots = vec![];
    for &plot in &tree.nodes[area].children {
        cancelled(check)?;
        let name = &tree.nodes[plot].element.name;
        if name.namespace != C
            || !matches!(
                name.local.as_str(),
                "areaChart"
                    | "area3DChart"
                    | "lineChart"
                    | "line3DChart"
                    | "stockChart"
                    | "radarChart"
                    | "scatterChart"
                    | "pieChart"
                    | "pie3DChart"
                    | "doughnutChart"
                    | "barChart"
                    | "bar3DChart"
                    | "ofPieChart"
                    | "surfaceChart"
                    | "surface3DChart"
                    | "bubbleChart"
            )
        {
            continue;
        }
        let mut series = vec![];
        let mut series_indices = BTreeSet::new();
        let mut series_orders = BTreeSet::new();
        for s in tree.children(plot, "ser") {
            cancelled(check)?;
            budget.series += 1;
            if budget.series > budget.limits.max_series {
                return Err(PptxError::Limit("chart series"));
            }
            let index = tree.number(s, "idx")?;
            let order = tree.number(s, "order")?;
            if !series_indices.insert(index) || !series_orders.insert(order) {
                return Err(invalid("duplicate chart series index or order"));
            }
            let mut channels = vec![];
            for (name, role) in [
                ("tx", ChartChannelRole::Title),
                ("cat", ChartChannelRole::Categories),
                ("val", ChartChannelRole::Values),
                ("xVal", ChartChannelRole::XValues),
                ("yVal", ChartChannelRole::YValues),
                ("bubbleSize", ChartChannelRole::BubbleSize),
            ] {
                if let Some(n) = tree.optional(s, name)? {
                    channels.push(channel(tree, n, role, budget, check)?);
                }
            }
            let mut point_overrides = vec![];
            let mut override_indices = BTreeSet::new();
            for p in tree.children(s, "dPt") {
                cancelled(check)?;
                budget.points += 1;
                if budget.points > budget.limits.max_points {
                    return Err(PptxError::Limit("chart points and overrides"));
                }
                let index = tree.number(p, "idx")?;
                if !override_indices.insert(index) {
                    return Err(invalid("duplicate chart point override"));
                }
                point_overrides.push(SourceChartPointOverride {
                    source_ordinal: tree.nodes[p].ordinal,
                    index,
                    layout: layout::read(tree, p, layout::Scope::Point, check)?,
                });
            }
            series.push(SourceChartSeries {
                source_ordinal: tree.nodes[s].ordinal,
                index,
                order,
                channels,
                layout: layout::read(tree, s, layout::Scope::Series, check)?,
                point_overrides,
            });
        }
        let axis_ids = tree
            .children(plot, "axId")
            .map(|i| number(&tree.nodes[i].element, "val"))
            .collect::<Result<_, _>>()?;
        plots.push(SourceChartPlot {
            layout: layout::read(tree, plot, layout::Scope::Plot, check)?,
            source_ordinal: tree.nodes[plot].ordinal,
            native_kind: name.local.clone(),
            axis_ids,
            series,
        });
    }
    let external_data = tree
        .optional(0, "externalData")?
        .map(|n| {
            let relationship_id = relationship_id(&tree.nodes[n].element)?;
            let rel = budget.relationship(package, part, &relationship_id, "package", check)?;
            let auto_update = tree
                .optional(n, "autoUpdate")?
                .map(
                    |i| match tree.nodes[i].element.attribute("val").unwrap_or("true") {
                        "true" | "1" => Ok(true),
                        "false" | "0" => Ok(false),
                        _ => Err(invalid("invalid chart autoUpdate")),
                    },
                )
                .transpose()?;
            let target = match &rel.resolved {
                RelationshipTarget::External => ChartWorkbookTarget::External {
                    uri: rel.target.clone(),
                },
                RelationshipTarget::Internal {
                    part,
                    fragment: None,
                } => {
                    let info = package
                        .parts()
                        .get(part)
                        .ok_or_else(|| invalid("missing chart workbook part"))?;
                    ChartWorkbookTarget::Embedded {
                        part: part.to_string(),
                        content_type: info.content_type.clone(),
                        sha256: info.sha256.clone(),
                        byte_length: ByteLength::new(info.byte_length),
                    }
                }
                _ => return Err(invalid("chart workbook fragment unsupported")),
            };
            Ok(SourceChartExternalData {
                source_ordinal: tree.nodes[n].ordinal,
                relationship_id,
                auto_update,
                target,
            })
        })
        .transpose()?;
    Ok((plots, external_data))
}
