//! Physical annotation graph. Defaults, Office override rules, automatic chart
//! styles, number formatting and placement belong to subsequent computation.
use super::*;
use ChartAnnotationKind as N;

fn child_kind(parent: N, name: &str) -> Option<N> {
    Some(match (parent, name) {
        (N::DataLabels, "dLbl") => N::DataLabel,
        (N::Legend, "legendEntry") => N::LegendEntry,
        (N::DataLabel | N::Legend | N::Title, "layout") => N::Layout,
        (N::Layout, "manualLayout") => N::ManualLayout,
        _ => return None,
    })
}
fn text_source(
    tree: &tree::Tree,
    parent: usize,
    roots: &mut BTreeMap<u32, u32>,
    budget: &mut Budget,
    check: &dyn Fn() -> bool,
) -> Result<Option<SourceChartAnnotationText>, PptxError> {
    let Some(tx) = tree.optional(parent, "tx")? else {
        return Ok(None);
    };
    let node = &tree.nodes[tx];
    let rich = tree.optional(tx, "rich")?;
    let reference = tree.optional(tx, "strRef")?;
    let content = match (rich, reference) {
        (Some(r), None) => {
            roots.insert(tree.nodes[r].ordinal, node.ordinal);
            ChartAnnotationTextContent::Rich {
                source_ordinal: tree.nodes[r].ordinal,
            }
        }
        (None, Some(_)) => ChartAnnotationTextContent::StringReference {
            channel: data::channel(tree, tx, ChartChannelRole::Title, budget, check)?,
        },
        _ => {
            return Err(invalid(
                "chart annotation text requires one rich body or string reference",
            ));
        }
    };
    let mut retained_ordinals = node.extensions.clone();
    layout::retain_attributes(node, &[], &mut retained_ordinals);
    for &i in &node.children {
        if Some(i) != rich && Some(i) != reference {
            retained_ordinals.push(tree.nodes[i].ordinal);
        }
    }
    if let Some(reference) = reference {
        retain_reference(tree, reference, &mut retained_ordinals, check)?;
    }
    if !node.text.trim().is_empty() {
        return Err(invalid("chart annotation text outside source"));
    }
    Ok(Some(SourceChartAnnotationText {
        source_ordinal: node.ordinal,
        content,
        retained_ordinals,
    }))
}
fn retain_reference(
    tree: &tree::Tree,
    root: usize,
    retained: &mut Vec<u32>,
    check: &dyn Fn() -> bool,
) -> Result<(), PptxError> {
    let mut pending = vec![root];
    while let Some(i) = pending.pop() {
        cancelled(check)?;
        let node = &tree.nodes[i];
        let (attributes, children): (&[&str], &[&str]) = match node.element.name.local.as_str() {
            "strRef" => (&[], &["f", "strCache"]),
            "strCache" => (&[], &["ptCount", "pt"]),
            "pt" => (&["idx"], &["v"]),
            "ptCount" => (&["val"], &[]),
            _ => (&[], &[]),
        };
        layout::retain_attributes(node, attributes, retained);
        retained.extend(&node.extensions);
        if !matches!(node.element.name.local.as_str(), "f" | "v") && !node.text.trim().is_empty() {
            retained.push(node.ordinal);
        }
        for &child in node.children.iter().rev() {
            let n = &tree.nodes[child];
            if n.element.name.namespace == C && children.contains(&n.element.name.local.as_str()) {
                pending.push(child);
            } else {
                retained.push(n.ordinal);
            }
        }
    }
    retained.sort_unstable();
    retained.dedup();
    Ok(())
}
#[allow(clippy::too_many_arguments)]
pub(super) fn read(
    tree: &tree::Tree,
    bytes: &[u8],
    plots: &[SourceChartPlot],
    axes: &[SourceChartAxis],
    source_limits: SourceLimits,
    budget: &mut Budget,
    check: &dyn Fn() -> bool,
) -> Result<SourceChartAnnotations, PptxError> {
    let chart = tree.required(0, "chart")?;
    let area = tree.required(chart, "plotArea")?;
    let plot_ids: BTreeSet<_> = plots.iter().map(|p| p.source_ordinal).collect();
    let series_ids: BTreeSet<_> = plots
        .iter()
        .flat_map(|p| &p.series)
        .map(|s| s.source_ordinal)
        .collect();
    let axis_ids: BTreeSet<_> = axes.iter().map(|a| a.source_ordinal).collect();
    let mut parents = vec![None; tree.nodes.len()];
    for (i, node) in tree.nodes.iter().enumerate() {
        cancelled(check)?;
        for &child in &node.children {
            parents[child] = Some(i);
        }
    }
    let mut kinds = BTreeMap::new();
    let mut roots = BTreeMap::new();
    let mut indices = BTreeSet::new();
    let mut singles = BTreeSet::new();
    let mut result = SourceChartAnnotations::default();
    for (i, node) in tree.nodes.iter().enumerate() {
        cancelled(check)?;
        let Some(p) = parents[i] else {
            continue;
        };
        if node.element.name.namespace != C {
            continue;
        }
        let parent_ordinal = tree.nodes[p].ordinal;
        let name = node.element.name.local.as_str();
        let parent_kind = kinds.get(&p).copied();
        if name == "txPr"
            && (p == 0
                || axis_ids.contains(&parent_ordinal)
                || parent_kind.is_some_and(|k| {
                    annotation_layout::markup(k, name) == Some(ChartMarkupKind::TextProperties)
                }))
        {
            if !singles.insert((p, name)) {
                return Err(invalid("duplicate chart text properties"));
            }
            roots.insert(node.ordinal, parent_ordinal);
        }
        let kind = if let Some(k) = parent_kind {
            child_kind(k, name)
        } else if p == chart {
            match name {
                "title" => Some(N::Title),
                "legend" => Some(N::Legend),
                _ => None,
            }
        } else if p == area && name == "layout" {
            Some(N::Layout)
        } else if axis_ids.contains(&parent_ordinal) && name == "title" {
            Some(N::Title)
        } else if (plot_ids.contains(&parent_ordinal) || series_ids.contains(&parent_ordinal))
            && name == "dLbls"
        {
            Some(N::DataLabels)
        } else {
            None
        };
        let Some(kind) = kind else {
            continue;
        };
        budget.annotations = budget
            .annotations
            .checked_add(1)
            .filter(|n| *n <= budget.limits.max_annotations)
            .ok_or(PptxError::Limit("chart annotations"))?;
        let index = if matches!(kind, N::DataLabel | N::LegendEntry) {
            let idx = tree.number(i, "idx")?;
            if !indices.insert((p, idx)) {
                return Err(invalid("duplicate chart annotation index"));
            }
            Some(idx)
        } else {
            if !singles.insert((p, name)) {
                return Err(invalid("duplicate chart annotation"));
            }
            None
        };
        kinds.insert(i, kind);
        let mut declarations = layout::read(tree, i, layout::Scope::Annotation(kind), check)?;
        if let Some(idx) = tree.optional(i, "idx")? {
            layout::leaf(&tree.nodes[idx])?;
            layout::retain_attributes(
                &tree.nodes[idx],
                &["val"],
                &mut declarations.retained_attribute_ordinals,
            );
        }
        let number_format = if matches!(kind, N::DataLabels | N::DataLabel) {
            if let Some(n) = tree.optional(i, "numFmt")? {
                layout::retain_attributes(
                    &tree.nodes[n],
                    &["formatCode", "sourceLinked"],
                    &mut declarations.retained_attribute_ordinals,
                );
            }
            layout::number_format(tree, i)?
        } else {
            None
        };
        let text_source = if matches!(kind, N::DataLabel | N::Title) {
            text_source(tree, i, &mut roots, budget, check)?
        } else {
            None
        };
        if declarations
            .properties
            .iter()
            .any(|p| p.kind == ChartPropertyKind::Delete)
            && (declarations.properties.len() != 1
                || number_format.is_some()
                || declarations.markup.iter().any(|m| {
                    !matches!(
                        m.kind,
                        ChartMarkupKind::Extensions | ChartMarkupKind::DataLabel
                    )
                }))
        {
            return Err(invalid(
                "chart annotation delete conflicts with formatting choice",
            ));
        }
        if !node.text.trim().is_empty() {
            return Err(invalid("chart annotation character data"));
        }
        result.nodes.push(SourceChartAnnotation {
            source_ordinal: node.ordinal,
            parent_ordinal,
            kind,
            index,
            declarations,
            number_format,
            text_source,
        });
    }
    result.text_bodies = annotation_text::read(
        bytes,
        &roots,
        source_limits,
        budget.limits.max_text_bodies,
        &mut budget.text,
        check,
    )?;
    Ok(result)
}
