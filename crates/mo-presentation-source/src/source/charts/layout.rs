//! Bounded reads of direct chart layout declarations. No defaults or geometry.
use super::*;
use std::collections::BTreeSet;

#[derive(Clone, Copy)]
pub(super) enum Scope {
    Plot,
    Axis,
    Scaling,
    Series,
    Point,
    Annotation(ChartAnnotationKind),
}
fn property(scope: Scope, name: &str) -> Option<ChartPropertyKind> {
    if let Scope::Annotation(kind) = scope {
        return super::annotation_layout::property(kind, name);
    }
    use ChartPropertyKind::*;
    Some(match (scope, name) {
        (Scope::Plot, "barDir") => BarDirection,
        (Scope::Plot, "grouping") => Grouping,
        (Scope::Plot, "varyColors") => VaryColors,
        (Scope::Plot, "gapWidth") => GapWidth,
        (Scope::Plot, "gapDepth") => GapDepth,
        (Scope::Plot, "overlap") => Overlap,
        (Scope::Plot, "firstSliceAng") => FirstSliceAngle,
        (Scope::Plot, "holeSize") => HoleSize,
        (Scope::Axis, "delete") => Delete,
        (Scope::Axis, "axPos") => AxisPosition,
        (Scope::Axis, "majorTickMark") => MajorTickMark,
        (Scope::Axis, "minorTickMark") => MinorTickMark,
        (Scope::Axis, "tickLblPos") => TickLabelPosition,
        (Scope::Axis, "crossAx") => CrossAxis,
        (Scope::Axis, "crosses") => Crosses,
        (Scope::Axis, "crossesAt") => CrossesAt,
        (Scope::Axis, "crossBetween") => CrossBetween,
        (Scope::Axis, "majorUnit") => MajorUnit,
        (Scope::Axis, "minorUnit") => MinorUnit,
        (Scope::Axis, "auto") => Auto,
        (Scope::Axis, "lblAlgn") => LabelAlignment,
        (Scope::Axis, "lblOffset") => LabelOffset,
        (Scope::Axis, "tickLblSkip") => TickLabelSkip,
        (Scope::Axis, "tickMarkSkip") => TickMarkSkip,
        (Scope::Axis, "noMultiLvlLbl") => NoMultiLevelLabels,
        (Scope::Axis, "baseTimeUnit") => BaseTimeUnit,
        (Scope::Axis, "majorTimeUnit") => MajorTimeUnit,
        (Scope::Axis, "minorTimeUnit") => MinorTimeUnit,
        (Scope::Scaling, "logBase") => LogBase,
        (Scope::Scaling, "orientation") => Orientation,
        (Scope::Scaling, "min") => Minimum,
        (Scope::Scaling, "max") => Maximum,
        (Scope::Series | Scope::Point, "explosion") => Explosion,
        (Scope::Series | Scope::Point, "bubble3D") => Bubble3D,
        (Scope::Series | Scope::Point, "invertIfNegative") => InvertIfNegative,
        (Scope::Series, "smooth") => Smooth,
        _ => return None,
    })
}
fn markup(scope: Scope, name: &str) -> Option<ChartMarkupKind> {
    if let Scope::Annotation(kind) = scope {
        return super::annotation_layout::markup(kind, name);
    }
    use ChartMarkupKind::*;
    Some(match (scope, name) {
        (Scope::Axis | Scope::Series | Scope::Point, "spPr") => ShapeProperties,
        (Scope::Axis, "txPr") => TextProperties,
        (Scope::Axis, "title") => Title,
        (Scope::Axis, "majorGridlines") => MajorGridlines,
        (Scope::Axis, "minorGridlines") => MinorGridlines,
        (Scope::Axis, "dispUnits") => DisplayUnits,
        (Scope::Plot | Scope::Series, "dLbls") => DataLabels,
        (Scope::Plot, "serLines") => SeriesLines,
        (Scope::Series | Scope::Point, "marker") => Marker,
        (Scope::Series | Scope::Point, "pictureOptions") => PictureOptions,
        (Scope::Series, "trendline") => Trendline,
        (Scope::Series, "errBars") => ErrorBars,
        (_, "extLst") => Extensions,
        _ => return None,
    })
}
pub(super) fn leaf(node: &tree::Node) -> Result<(), PptxError> {
    if !node.children.is_empty() || !node.extensions.is_empty() || !node.text.trim().is_empty() {
        return Err(invalid("chart layout scalar contains content"));
    }
    Ok(())
}
pub(super) fn read(
    tree: &tree::Tree,
    parent: usize,
    scope: Scope,
    check: &dyn Fn() -> bool,
) -> Result<SourceChartLayout, PptxError> {
    let mut result = SourceChartLayout {
        properties: vec![],
        markup: tree.nodes[parent]
            .extensions
            .iter()
            .map(|source_ordinal| SourceChartMarkup {
                source_ordinal: *source_ordinal,
                kind: ChartMarkupKind::Extensions,
            })
            .collect(),
        unrecognized_children: vec![],
        retained_attribute_ordinals: vec![],
    };
    retain_attributes(
        &tree.nodes[parent],
        &[],
        &mut result.retained_attribute_ordinals,
    );
    let mut properties = BTreeSet::new();
    let mut markups = BTreeSet::new();
    for &i in &tree.nodes[parent].children {
        cancelled(check)?;
        let node = &tree.nodes[i];
        let native = node.element.name.namespace == C;
        if let Some(kind) = native
            .then(|| property(scope, &node.element.name.local))
            .flatten()
        {
            let separator = kind == ChartPropertyKind::Separator;
            if separator {
                if !node.children.is_empty() || !node.extensions.is_empty() {
                    return Err(invalid("chart separator contains elements"));
                }
            } else {
                leaf(node)?;
            }
            retain_attributes(
                node,
                if separator { &[] } else { &["val"] },
                &mut result.retained_attribute_ordinals,
            );
            if !properties.insert(kind) {
                return Err(invalid("duplicate chart layout property"));
            }
            result.properties.push(SourceChartProperty {
                source_ordinal: node.ordinal,
                kind,
                value: if separator {
                    Some(node.text.clone())
                } else {
                    node.element.attribute("val").map(str::to_owned)
                },
            });
        } else if let Some(kind) = native
            .then(|| markup(scope, &node.element.name.local))
            .flatten()
        {
            // serLines permits repeated declarations; keep physical source order.
            if !matches!(
                kind,
                ChartMarkupKind::SeriesLines
                    | ChartMarkupKind::Trendline
                    | ChartMarkupKind::ErrorBars
                    | ChartMarkupKind::DataLabel
                    | ChartMarkupKind::LegendEntry
            ) && !markups.insert(kind)
            {
                return Err(invalid("duplicate chart layout markup"));
            }
            result.markup.push(SourceChartMarkup {
                source_ordinal: node.ordinal,
                kind,
            });
        } else {
            let structural = native
                && match scope {
                    Scope::Plot => matches!(node.element.name.local.as_str(), "ser" | "axId"),
                    Scope::Series => matches!(
                        node.element.name.local.as_str(),
                        "idx"
                            | "order"
                            | "tx"
                            | "cat"
                            | "val"
                            | "xVal"
                            | "yVal"
                            | "bubbleSize"
                            | "dPt"
                    ),
                    Scope::Point => node.element.name.local == "idx",
                    Scope::Axis => matches!(
                        node.element.name.local.as_str(),
                        "axId" | "scaling" | "numFmt"
                    ),
                    Scope::Scaling => false,
                    Scope::Annotation(kind) => {
                        super::annotation_layout::structural(kind, &node.element.name.local)
                    }
                };
            if !structural {
                result.unrecognized_children.push(SourceChartUnknown {
                    source_ordinal: node.ordinal,
                    namespace: node.element.name.namespace.clone(),
                    local_name: node.element.name.local.clone(),
                });
            }
        }
    }
    Ok(result)
}
pub(super) fn retain_attributes(node: &tree::Node, allowed: &[&str], retained: &mut Vec<u32>) {
    if node.element.attributes.iter().any(|a| {
        a.name.namespace != "http://schemas.openxmlformats.org/markup-compatibility/2006"
            && (!a.name.namespace.is_empty() || !allowed.contains(&a.name.local.as_str()))
    }) {
        retained.push(node.ordinal);
    }
}
pub(super) fn number_format(
    tree: &tree::Tree,
    parent: usize,
) -> Result<Option<SourceChartNumberFormat>, PptxError> {
    tree.optional(parent, "numFmt")?
        .map(|i| {
            let n = &tree.nodes[i];
            leaf(n)?;
            Ok(SourceChartNumberFormat {
                source_ordinal: n.ordinal,
                format_code: n.element.attribute("formatCode").map(str::to_owned),
                source_linked: n.element.attribute("sourceLinked").map(str::to_owned),
            })
        })
        .transpose()
}
pub(super) fn axes(
    tree: &tree::Tree,
    budget: &mut Budget,
    check: &dyn Fn() -> bool,
) -> Result<Vec<SourceChartAxis>, PptxError> {
    let chart = tree.required(0, "chart")?;
    let area = tree.required(chart, "plotArea")?;
    let mut result = vec![];
    let mut ids = BTreeSet::new();
    for &node in &tree.nodes[area].children {
        cancelled(check)?;
        let element = &tree.nodes[node].element;
        if element.name.namespace != C {
            continue;
        }
        let kind = match element.name.local.as_str() {
            "catAx" => ChartAxisKind::Category,
            "valAx" => ChartAxisKind::Value,
            "dateAx" => ChartAxisKind::Date,
            "serAx" => ChartAxisKind::Series,
            _ => continue,
        };
        budget.axes += 1;
        if budget.axes > budget.limits.max_axes {
            return Err(PptxError::Limit("chart axes"));
        }
        let id = tree.number(node, "axId")?;
        if !ids.insert(id) {
            return Err(invalid("duplicate chart axis id"));
        }
        let scaling = tree
            .optional(node, "scaling")?
            .map(|i| {
                Ok::<_, PptxError>(SourceChartScaling {
                    source_ordinal: tree.nodes[i].ordinal,
                    properties: read(tree, i, Scope::Scaling, check)?.properties,
                })
            })
            .transpose()?;
        let number_format = number_format(tree, node)?;
        result.push(SourceChartAxis {
            source_ordinal: tree.nodes[node].ordinal,
            kind,
            id,
            layout: read(tree, node, Scope::Axis, check)?,
            scaling,
            number_format,
        });
    }
    Ok(result)
}
