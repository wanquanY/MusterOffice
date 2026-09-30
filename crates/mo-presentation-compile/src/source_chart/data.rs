use super::*;
use mo_charts::{DecimalNumber, sectors::SectorWeight};
use std::collections::BTreeMap;
type PreparedValues<'a> = (
    Vec<SectorWeight>,
    Vec<SourceCircularPoint>,
    &'a SourceChartChannel,
);

pub(super) fn geometry_modifiers(series: &SourceChartSeries) -> Result<(), SourceCircularError> {
    for (point, layout) in std::iter::once((None, &series.layout)).chain(
        series
            .point_overrides
            .iter()
            .map(|p| (Some(p.index), &p.layout)),
    ) {
        if let Some(ordinal) = layout.retained_attribute_ordinals.first() {
            return Err(unresolved(
                *ordinal,
                Some(series.index),
                point,
                "unrecognized series or point attribute",
            ));
        }
        if let Some(node) = layout.unrecognized_children.first() {
            return Err(unresolved(
                node.source_ordinal,
                Some(series.index),
                point,
                "unrecognized series or point geometry",
            ));
        }
        for p in &layout.properties {
            match p.kind {
                ChartPropertyKind::Explosion
                    if p.value
                        .as_deref()
                        .and_then(|v| v.trim().parse::<u32>().ok())
                        != Some(0) =>
                {
                    return Err(unresolved(
                        p.source_ordinal,
                        Some(series.index),
                        point,
                        "nonzero or unresolved explosion requires displaced sector geometry",
                    ));
                }
                ChartPropertyKind::Bubble3D
                    if !matches!(p.value.as_deref(), Some("0" | "false")) =>
                {
                    return Err(unresolved(
                        p.source_ordinal,
                        Some(series.index),
                        point,
                        "three-dimensional point geometry unresolved",
                    ));
                }
                _ => {}
            }
        }
        if let Some(p) = layout
            .markup
            .iter()
            .find(|p| p.kind == ChartMarkupKind::Extensions)
        {
            return Err(unresolved(
                p.source_ordinal,
                Some(series.index),
                point,
                "chart series or point extension geometry unresolved",
            ));
        }
    }
    Ok(())
}

pub(super) fn weights<'a>(
    series: &'a SourceChartSeries,
    max_points: usize,
    check: &dyn Fn() -> bool,
) -> Result<PreparedValues<'a>, SourceCircularError> {
    let fail = |ordinal, point, reason| unresolved(ordinal, Some(series.index), point, reason);
    let channel = series
        .channels
        .iter()
        .find(|c| c.role == ChartChannelRole::Values)
        .ok_or_else(|| {
            fail(
                series.source_ordinal,
                None,
                "numeric values channel missing",
            )
        })?;
    let cache = channel.cache.as_ref().ok_or_else(|| {
        fail(
            channel.source_ordinal,
            None,
            "numeric cache missing; no implicit workbook refresh",
        )
    })?;
    if cache.kind != ChartCacheKind::Number || cache.levels.len() != 1 {
        return Err(fail(
            cache.source_ordinal,
            None,
            "numeric cache shape invalid",
        ));
    }
    let count = cache.declared_point_count.ok_or_else(|| {
        fail(
            cache.source_ordinal,
            None,
            "numeric cache point count required",
        )
    })?;
    if count as usize > max_points {
        return Err(chart_geometry::ChartGeometryError::Limit("source numeric points").into());
    }
    let mut sorted = BTreeMap::new();
    for p in &cache.levels[0] {
        cancel(check)?;
        if p.index >= count || sorted.insert(p.index, p).is_some() {
            return Err(fail(
                p.source_ordinal,
                Some(p.index),
                "numeric cache index invalid",
            ));
        }
    }
    if sorted.len() != count as usize {
        return Err(fail(
            cache.source_ordinal,
            (0..count).find(|i| !sorted.contains_key(i)),
            "sparse numeric cache needs explicit missing-data semantics",
        ));
    }
    for point in &series.point_overrides {
        if point.index >= count {
            return Err(fail(
                point.source_ordinal,
                Some(point.index),
                "point formatting refers outside numeric data",
            ));
        }
    }
    let mut weights = Vec::with_capacity(sorted.len());
    let mut points = Vec::with_capacity(sorted.len());
    for (index, p) in sorted {
        cancel(check)?;
        let value = p
            .value
            .as_ref()
            .ok_or_else(|| fail(p.source_ordinal, Some(index), "numeric point value missing"))?;
        let value = DecimalNumber::try_from(value.clone()).map_err(|e| match e {
            mo_charts::NumberError::Invalid => fail(
                p.source_ordinal,
                Some(index),
                "numeric point is blank, an error or a non-finite value",
            ),
            mo_charts::NumberError::Limit => {
                chart_geometry::ChartGeometryError::Limit("source numeric value").into()
            }
        })?;
        weights.push(SectorWeight {
            point_index: index,
            value,
        });
        points.push(SourceCircularPoint {
            index,
            source_ordinal: p.source_ordinal,
            format_code: p.format_code.clone().or_else(|| cache.format_code.clone()),
        });
    }
    Ok((weights, points, channel))
}
