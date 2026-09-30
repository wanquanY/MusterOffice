//! Resolve label declarations and bind their data without inventing chart-style
//! defaults, formatting decimal values or replacing labels with shape text boxes.
mod data;
mod prepared;
mod settings;
mod types;
use mo_opc::PackageRead;
use mo_presentation_source::{
    PptxError,
    source::{SourceIndex, SourceLimits, charts::*},
};
use std::collections::{BTreeMap, BTreeSet};
pub use types::*;

fn unresolved(
    ordinal: u32,
    target: Option<ChartLabelTarget>,
    reason: &'static str,
) -> ChartLabelError {
    ChartLabelError::Unresolved {
        source_ordinal: ordinal,
        series_index: target.map(|t| t.series_index),
        point_index: target.map(|t| t.point_index),
        reason,
    }
}
struct Budget<'a> {
    limits: SourceChartLabelLimits,
    bytes: usize,
    points: usize,
    ratio_work: mo_charts::sectors::SectorWork,
    check: &'a dyn Fn() -> bool,
}
impl Budget<'_> {
    fn check(&self) -> Result<(), ChartLabelError> {
        if (self.check)() {
            Err(PptxError::Cancelled.into())
        } else {
            Ok(())
        }
    }
    fn bytes(&mut self, n: usize) -> Result<(), ChartLabelError> {
        self.bytes = self
            .bytes
            .checked_add(n)
            .filter(|v| *v <= self.limits.max_retained_bytes)
            .ok_or(PptxError::Limit("chart label retained bytes"))?;
        Ok(())
    }
}
pub fn compute(
    package: &dyn PackageRead,
    index: &SourceIndex,
    request: &SourceChartLabelRequest,
    source_limits: SourceLimits,
    limits: SourceChartLabelLimits,
    check: &dyn Fn() -> bool,
) -> Result<SourceChartLabels, ChartLabelError> {
    if request.targets.len() > limits.max_targets {
        return Err(PptxError::Limit("chart label targets").into());
    }
    let charts = query(
        package,
        index,
        &SourceChartQuery {
            expected_source_sha256: request.expected_source_sha256.clone(),
            surface: request.object.part.clone(),
        },
        source_limits,
        limits.source,
        check,
    )?;
    let binding = charts
        .bindings
        .iter()
        .find(|b| b.object == request.object)
        .ok_or_else(|| PptxError::SourceConflict("chart label object missing".into()))?;
    compute_prepared(
        &charts.charts[binding.chart as usize],
        request,
        limits,
        check,
    )
}
/// Same parsed part can be used by the complete chart compiler, avoiding another
/// package query for every label. The caller owns the package/object source pin.
pub(crate) fn compute_prepared(
    chart: &SourceChartPart,
    request: &SourceChartLabelRequest,
    limits: SourceChartLabelLimits,
    check: &dyn Fn() -> bool,
) -> Result<SourceChartLabels, ChartLabelError> {
    let mut budget = Budget {
        limits,
        bytes: 0,
        points: 0,
        ratio_work: Default::default(),
        check,
    };
    budget.check()?;
    if request.targets.len() > limits.max_targets {
        return Err(PptxError::Limit("chart label targets").into());
    }
    let plot = chart
        .plots
        .iter()
        .find(|p| p.source_ordinal == request.plot_source_ordinal)
        .ok_or_else(|| PptxError::SourceConflict("chart label plot missing".into()))?;
    let series_index: BTreeMap<_, _> = plot.series.iter().map(|s| (s.index, s)).collect();
    let mut groups = BTreeMap::new();
    let mut points = BTreeMap::new();
    for node in &chart.annotations.nodes {
        budget.check()?;
        match node.kind {
            ChartAnnotationKind::DataLabels => {
                if groups
                    .insert(
                        node.parent_ordinal,
                        prepared::Declaration::new(node, &budget)?,
                    )
                    .is_some()
                {
                    return Err(PptxError::SourceConflict("duplicate label group".into()).into());
                }
            }
            ChartAnnotationKind::DataLabel => {
                let idx = node
                    .index
                    .ok_or_else(|| PptxError::SourceConflict("label point index missing".into()))?;
                if points
                    .insert(
                        (node.parent_ordinal, idx),
                        prepared::Declaration::new(node, &budget)?,
                    )
                    .is_some()
                {
                    return Err(PptxError::SourceConflict("duplicate label point".into()).into());
                }
            }
            _ => {}
        }
    }
    budget.bytes(chart.part.len() + request.object.part.len() + 256)?;
    let mut result = SourceChartLabels {
        profile: request.profile,
        source_sha256: request.expected_source_sha256.clone(),
        object: request.object.clone(),
        chart_part: chart.part.clone(),
        chart_sha256: chart.sha256.clone(),
        plot_source_ordinal: plot.source_ordinal,
        data_authority: chart.data_authority,
        negative_weights: request.negative_weights,
        labels: vec![],
        normalizations: vec![],
        text_cascades: vec![],
    };
    let mut selected = BTreeSet::new();
    let mut series_data = BTreeMap::new();
    let mut normalized = BTreeMap::new();
    let mut text = mo_presentation_source::source::text::cascade::ChartTextResolver::new(
        chart,
        limits.text,
        check,
    )?;
    let mut cascades = BTreeMap::new();
    let chart_defaults: Vec<_> = chart
        .annotations
        .text_bodies
        .iter()
        .filter(|b| b.parent_ordinal == 0)
        .map(|b| b.source_ordinal)
        .collect();
    for &target in &request.targets {
        budget.check()?;
        budget.bytes(1280)?;
        if !selected.insert(target) {
            return Err(PptxError::SourceConflict("duplicate chart label target".into()).into());
        }
        let series = series_index
            .get(&target.series_index)
            .copied()
            .ok_or_else(|| {
                unresolved(
                    plot.source_ordinal,
                    Some(target),
                    "series not in selected plot",
                )
            })?;
        if let std::collections::btree_map::Entry::Vacant(e) = series_data.entry(series.index) {
            e.insert(data::Series::new(series, &mut budget)?);
        }
        let data = &series_data[&series.index];
        data.validate_target(target)?;
        let series_group = groups.get(&series.source_ordinal);
        let plot_group = groups.get(&plot.source_ordinal);
        let mut chain = Vec::new();
        if let Some(group) = series_group {
            chain.extend(points.get(&(group.node.source_ordinal, target.point_index)));
            chain.push(group);
        }
        // Chart-level dLbl is preserved by source inspection but rejected by the
        // Office rules in settings; it is never used as an undocumented fallback.
        chain.extend(plot_group);
        let settings = settings::resolve(&chain, plot, &chart_defaults, &mut budget)
            .map_err(|e| at(e, target))?;
        let custom = chain.iter().find_map(|n| n.node.text_source.as_ref());
        if let Some(tx) = custom
            && !tx.retained_ordinals.is_empty()
        {
            return Err(unresolved(
                tx.source_ordinal,
                Some(target),
                "custom label text has unresolved content",
            ));
        }
        let mut label = ChartLabelPlan {
            target,
            annotation_chain: chain.iter().map(|n| n.node.source_ordinal).collect(),
            settings,
            components: vec![],
            custom_text_source: custom.map(|t| t.source_ordinal),
            text_cascade: None,
            legend_key_visible: None,
        };
        if !label.settings.deleted.as_ref().is_some_and(|d| d.value) {
            let mut roots = label.settings.text_property_roots.clone();
            if let Some(SourceChartAnnotationText {
                content: ChartAnnotationTextContent::Rich { source_ordinal },
                ..
            }) = custom
            {
                roots.insert(0, *source_ordinal);
            }
            if let Some((&body, parents)) = roots.split_first() {
                let idx = if let Some(&idx) = cascades.get(&roots) {
                    idx
                } else {
                    let idx = result.text_cascades.len() as u32;
                    result.text_cascades.push(text.resolve(body, parents)?);
                    cascades.insert(roots, idx);
                    idx
                };
                label.text_cascade = Some(idx);
            }
        }
        label.legend_key_visible = if label.settings.deleted.as_ref().is_some_and(|d| d.value) {
            Some(false)
        } else {
            match label.settings.flags.get(&ChartLabelFlag::LegendKey) {
                Some(v) if !v.value => Some(false),
                Some(_)
                    if custom.is_some()
                        || settings::FLAGS.iter().any(|(_, f)| {
                            *f != ChartLabelFlag::LegendKey
                                && settings::visible(&label.settings, *f)
                        }) =>
                {
                    Some(true)
                }
                Some(_) if label.settings.unresolved_flags.is_empty() => Some(false),
                _ => None,
            }
        };
        if !label.settings.deleted.as_ref().is_some_and(|d| d.value) && custom.is_none() {
            for flag in [
                ChartLabelFlag::SeriesName,
                ChartLabelFlag::CategoryName,
                ChartLabelFlag::Value,
                ChartLabelFlag::BubbleSize,
            ] {
                if settings::visible(&label.settings, flag) {
                    label
                        .components
                        .push(data.bind(target, flag, &label.settings, &mut budget)?);
                }
            }
            if settings::visible(&label.settings, ChartLabelFlag::Percent) {
                if !matches!(plot.native_kind.as_str(), "pieChart" | "doughnutChart") {
                    return Err(unresolved(
                        plot.source_ordinal,
                        Some(target),
                        "percent denominator for this chart kind requires native data semantics",
                    ));
                }
                let normalization = if let Some(&idx) = normalized.get(&series.index) {
                    idx
                } else {
                    let ratios = data.normalize(request.negative_weights, &mut budget)?;
                    if ratios.ratios.zero_total {
                        return Err(unresolved(
                            series.source_ordinal,
                            Some(target),
                            "zero total has no defined percentage",
                        ));
                    }
                    let idx = result.normalizations.len() as u32;
                    result.normalizations.push(ratios);
                    normalized.insert(series.index, idx);
                    idx
                };
                label.components.push(ChartLabelComponent::Percent {
                    normalization,
                    point_index: target.point_index,
                    format: data::percent_format(&label.settings, &mut budget)?,
                });
            }
        }
        result.labels.push(label);
    }
    budget.check()?;
    Ok(result)
}
fn at(error: ChartLabelError, target: ChartLabelTarget) -> ChartLabelError {
    match error {
        ChartLabelError::Unresolved {
            source_ordinal,
            reason,
            ..
        } => unresolved(source_ordinal, Some(target), reason),
        other => other,
    }
}
