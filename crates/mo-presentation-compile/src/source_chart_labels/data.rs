use super::*;
use mo_charts::{DecimalNumber, sectors::*};

pub(super) fn percent_format(
    settings: &ChartLabelSettings,
    budget: &mut Budget<'_>,
) -> Result<Option<ChartLabelDataFormat>, ChartLabelError> {
    // A cached value format is not a proven default percent format. Keep that
    // application decision unresolved unless the label supplies an unlinked code.
    settings
        .number_format
        .as_ref()
        .filter(|f| !f.source_linked.value)
        .map(|f| {
            budget.bytes(f.code.len())?;
            Ok(ChartLabelDataFormat {
                source_ordinal: f.source_ordinal,
                code: f.code.clone(),
            })
        })
        .transpose()
}
fn decimal(
    value: &str,
    ordinal: u32,
    target: Option<ChartLabelTarget>,
) -> Result<DecimalNumber, ChartLabelError> {
    DecimalNumber::try_from(value.to_owned()).map_err(|e| match e {
        mo_charts::NumberError::Invalid => {
            unresolved(ordinal, target, "blank, error or non-finite label number")
        }
        mo_charts::NumberError::Limit => PptxError::Limit("chart label numeric value").into(),
    })
}

struct Channel<'a> {
    source: &'a SourceChartChannel,
    points: BTreeMap<u32, &'a SourceChartPoint>,
}
pub(super) struct Series<'a> {
    source: &'a SourceChartSeries,
    channels: Vec<Channel<'a>>,
}
impl<'a> Series<'a> {
    pub fn new(
        source: &'a SourceChartSeries,
        budget: &mut Budget<'_>,
    ) -> Result<Self, ChartLabelError> {
        let mut channels = vec![];
        for c in &source.channels {
            budget.check()?;
            let mut points = BTreeMap::new();
            if let Some(cache) = &c.cache
                && cache.levels.len() == 1
            {
                for p in &cache.levels[0] {
                    budget.check()?;
                    budget.points = budget
                        .points
                        .checked_add(1)
                        .filter(|v| *v <= budget.limits.max_data_points)
                        .ok_or(PptxError::Limit("chart label data points"))?;
                    if points.insert(p.index, p).is_some() {
                        return Err(unresolved(
                            p.source_ordinal,
                            None,
                            "duplicate label data index",
                        ));
                    }
                }
            }
            channels.push(Channel { source: c, points });
        }
        Ok(Self { source, channels })
    }
    fn channel(&self, role: ChartChannelRole) -> Result<&Channel<'a>, ChartLabelError> {
        self.channels
            .iter()
            .find(|c| c.source.role == role)
            .ok_or_else(|| {
                unresolved(
                    self.source.source_ordinal,
                    None,
                    "label data channel missing",
                )
            })
    }
    fn primary_role(&self) -> ChartChannelRole {
        if self
            .channels
            .iter()
            .any(|c| c.source.role == ChartChannelRole::Values)
        {
            ChartChannelRole::Values
        } else {
            ChartChannelRole::YValues
        }
    }
    pub fn validate_target(&self, target: ChartLabelTarget) -> Result<(), ChartLabelError> {
        let c = self
            .channel(self.primary_role())
            .map_err(|e| at(e, target))?;
        let cache = c.source.cache.as_ref().ok_or_else(|| {
            unresolved(
                c.source.source_ordinal,
                Some(target),
                "label point count needs a source cache",
            )
        })?;
        let present = cache.declared_point_count.map_or_else(
            || c.points.contains_key(&target.point_index),
            |n| target.point_index < n,
        );
        if !present {
            return Err(unresolved(
                cache.source_ordinal,
                Some(target),
                "label point index outside data",
            ));
        }
        Ok(())
    }
    fn point(
        &self,
        role: ChartChannelRole,
        index: u32,
    ) -> Result<(&SourceChartChannel, &SourceChartPoint), ChartLabelError> {
        let c = self.channel(role)?;
        let cache = c.source.cache.as_ref().ok_or_else(|| {
            unresolved(
                c.source.source_ordinal,
                None,
                "label data cache missing; no workbook refresh",
            )
        })?;
        if cache.levels.len() != 1 || cache.kind == ChartCacheKind::MultiLevelString {
            return Err(unresolved(
                cache.source_ordinal,
                None,
                "multi-level category labels require hierarchy resolution",
            ));
        }
        let point = c.points.get(&index).ok_or_else(|| {
            unresolved(
                cache.source_ordinal,
                None,
                "required label cache point missing",
            )
        })?;
        Ok((c.source, point))
    }
    pub fn format(
        &self,
        target: ChartLabelTarget,
        settings: &ChartLabelSettings,
        role: ChartChannelRole,
        budget: &mut Budget<'_>,
    ) -> Result<Option<ChartLabelDataFormat>, ChartLabelError> {
        let selected = if let Some(f) = &settings.number_format
            && !f.source_linked.value
        {
            Some((f.source_ordinal, &f.code))
        } else {
            let (c, p) = self
                .point(role, target.point_index)
                .map_err(|e| at(e, target))?;
            p.format_code
                .as_ref()
                .map(|f| (p.source_ordinal, f))
                .or_else(|| {
                    c.cache
                        .as_ref()
                        .and_then(|c| c.format_code.as_ref().map(|f| (c.source_ordinal, f)))
                })
        };
        selected
            .map(|(source_ordinal, code)| {
                budget.bytes(code.len())?;
                Ok(ChartLabelDataFormat {
                    source_ordinal,
                    code: code.clone(),
                })
            })
            .transpose()
    }
    pub fn bind(
        &self,
        target: ChartLabelTarget,
        flag: ChartLabelFlag,
        settings: &ChartLabelSettings,
        budget: &mut Budget<'_>,
    ) -> Result<ChartLabelComponent, ChartLabelError> {
        let role = match flag {
            ChartLabelFlag::SeriesName => ChartChannelRole::Title,
            ChartLabelFlag::CategoryName => ChartChannelRole::Categories,
            ChartLabelFlag::Value => self.primary_role(),
            ChartLabelFlag::BubbleSize => ChartChannelRole::BubbleSize,
            _ => {
                return Err(unresolved(
                    self.source.source_ordinal,
                    Some(target),
                    "invalid label data role",
                ));
            }
        };
        if flag == ChartLabelFlag::SeriesName {
            let c = self.channel(role).map_err(|e| at(e, target))?.source;
            if !matches!(c.native_kind.as_str(), "v" | "strRef") {
                return Err(unresolved(
                    c.source_ordinal,
                    Some(target),
                    "series name requires native string data",
                ));
            }
            if let Some(value) = &c.literal_text {
                budget.bytes(value.len())?;
                return Ok(ChartLabelComponent::Text {
                    role: flag,
                    channel_source_ordinal: c.source_ordinal,
                    source_ordinal: c.source_ordinal,
                    value: value.clone(),
                });
            }
        }
        let point_index = if flag == ChartLabelFlag::SeriesName {
            0
        } else {
            target.point_index
        };
        let (c, p) = self.point(role, point_index).map_err(|e| at(e, target))?;
        if matches!(flag, ChartLabelFlag::Value | ChartLabelFlag::BubbleSize)
            && !c
                .cache
                .as_ref()
                .is_some_and(|c| c.kind == ChartCacheKind::Number)
        {
            return Err(unresolved(
                c.source_ordinal,
                Some(target),
                "numeric label role has nonnumeric cache",
            ));
        }
        let value = p.value.as_ref().ok_or_else(|| {
            unresolved(
                p.source_ordinal,
                Some(target),
                "required label value missing",
            )
        })?;
        budget.bytes(value.len())?;
        if c.cache
            .as_ref()
            .is_some_and(|c| c.kind == ChartCacheKind::Number)
        {
            let value = decimal(value, p.source_ordinal, Some(target))?;
            Ok(ChartLabelComponent::Number {
                role: flag,
                channel_source_ordinal: c.source_ordinal,
                source_ordinal: p.source_ordinal,
                value,
                format: self.format(target, settings, role, budget)?,
            })
        } else {
            Ok(ChartLabelComponent::Text {
                role: flag,
                channel_source_ordinal: c.source_ordinal,
                source_ordinal: p.source_ordinal,
                value: value.clone(),
            })
        }
    }
    pub fn normalize(
        &self,
        negative: NegativeWeights,
        budget: &mut Budget<'_>,
    ) -> Result<ChartLabelNormalization, ChartLabelError> {
        let c = self.channel(ChartChannelRole::Values)?;
        let cache = c.source.cache.as_ref().ok_or_else(|| {
            unresolved(
                c.source.source_ordinal,
                None,
                "percent values cache missing",
            )
        })?;
        let count = cache.declared_point_count.ok_or_else(|| {
            unresolved(cache.source_ordinal, None, "percent values count missing")
        })?;
        if cache.kind != ChartCacheKind::Number
            || cache.levels.len() != 1
            || c.points.len() != count as usize
            || c.points.keys().copied().ne(0..count)
        {
            return Err(unresolved(
                cache.source_ordinal,
                None,
                "percent denominator requires complete numeric data",
            ));
        }
        let mut limits = budget.limits.ratios;
        limits.max_points = limits
            .max_points
            .saturating_sub(budget.ratio_work.points as usize);
        limits.max_number_bytes = limits
            .max_number_bytes
            .saturating_sub(budget.ratio_work.number_bytes as usize);
        limits.max_scaled_decimal_digits = limits
            .max_scaled_decimal_digits
            .saturating_sub(budget.ratio_work.scaled_decimal_digits as usize);
        limits.max_boundary_decimal_digits = limits
            .max_boundary_decimal_digits
            .saturating_sub(budget.ratio_work.boundary_decimal_digits as usize);
        if c.points.len() > limits.max_points {
            return Err(PptxError::Limit("chart label normalization points").into());
        }
        let mut weights = Vec::with_capacity(c.points.len());
        for (&point_index, p) in &c.points {
            budget.check()?;
            let value = p
                .value
                .as_ref()
                .ok_or_else(|| unresolved(p.source_ordinal, None, "percent value missing"))?;
            let value = decimal(value, p.source_ordinal, None)?;
            weights.push(SectorWeight { point_index, value });
        }
        let ratios = ratios(&weights, negative, limits, budget.check)?;
        budget.ratio_work.points = budget
            .ratio_work
            .points
            .checked_add(ratios.work.points)
            .ok_or(PptxError::Limit("chart label ratio work"))?;
        budget.ratio_work.number_bytes = budget
            .ratio_work
            .number_bytes
            .checked_add(ratios.work.number_bytes)
            .ok_or(PptxError::Limit("chart label ratio work"))?;
        budget.ratio_work.scaled_decimal_digits = budget
            .ratio_work
            .scaled_decimal_digits
            .checked_add(ratios.work.scaled_decimal_digits)
            .ok_or(PptxError::Limit("chart label ratio work"))?;
        budget.ratio_work.boundary_decimal_digits = budget
            .ratio_work
            .boundary_decimal_digits
            .checked_add(ratios.work.boundary_decimal_digits)
            .ok_or(PptxError::Limit("chart label ratio work"))?;
        budget.bytes(ratios.denominator.len() + 128)?;
        for n in &ratios.numerators {
            budget.bytes(n.numerator.len() + 64)?;
        }
        Ok(ChartLabelNormalization {
            series_index: self.source.index,
            channel_source_ordinal: c.source.source_ordinal,
            ratios,
        })
    }
}
