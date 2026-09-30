//! Declared point -> series paint cascade. Automatic chart-style tables are not
//! substituted with shape defaults. This computation is shared by plot compilers.
use super::{paints::*, *};
use crate::source::{
    effects::SourceEffectProperties,
    fill::resolve::{
        self, EffectiveFill, FillBudget, FillInput, FillOrigin, FillPartial, FillResolveLimits,
    },
    line::resolve::{
        EffectiveLineGeometry, GeometryPartial, LineBudget, LineOrigin, LineResolveLimits,
    },
};

#[derive(Debug, Clone, Copy)]
pub struct ChartStyleLimits {
    pub max_points: usize,
    pub fills: FillResolveLimits,
    pub lines: LineResolveLimits,
}
impl Default for ChartStyleLimits {
    fn default() -> Self {
        Self {
            max_points: 4096,
            fills: FillResolveLimits::default(),
            lines: LineResolveLimits::default(),
        }
    }
}
#[derive(Debug, thiserror::Error)]
pub enum ChartStyleError {
    #[error(transparent)]
    Source(#[from] PptxError),
    #[error("unresolved chart style at ordinal {source_ordinal}: {reason}")]
    Unresolved {
        source_ordinal: u32,
        reason: &'static str,
    },
}
fn unresolved(source_ordinal: u32, reason: &'static str) -> ChartStyleError {
    ChartStyleError::Unresolved {
        source_ordinal,
        reason,
    }
}
/// Selected native properties, before shader/stroke/effect admission.
pub struct ChartPointStyle<'a> {
    pub fill: EffectiveFill,
    pub line_fill: EffectiveFill,
    pub line_geometry: EffectiveLineGeometry,
    /// None means automatic chart effects remain unresolved, not "no effect".
    pub effects: Option<&'a SourceEffectProperties>,
}
struct Series<'a> {
    definition: &'a SourceChartSeries,
    points: BTreeMap<u32, &'a SourceChartPointOverride>,
}
/// Immutable declaration bindings with one request-wide inheritance budget.
pub struct PreparedChartStyles<'a> {
    source: &'a SourceChartPaints,
    declarations: BTreeMap<u32, &'a ChartPaintDeclaration>,
    colors: BTreeMap<u32, &'a ChartPaintColor>,
    series: BTreeMap<(u32, u32), Series<'a>>,
    fills: FillBudget<'a>,
    lines: LineBudget<'a>,
    remaining: usize,
}
impl<'a> PreparedChartStyles<'a> {
    pub fn new(
        source: &'a SourceChartPaints,
        limits: ChartStyleLimits,
        check: &'a dyn Fn() -> bool,
    ) -> Result<Self, ChartStyleError> {
        let mut result = Self {
            source,
            declarations: BTreeMap::new(),
            colors: BTreeMap::new(),
            series: BTreeMap::new(),
            fills: FillBudget::new(limits.fills, check),
            lines: LineBudget {
                limits: limits.lines,
                check,
                steps: 0,
                values: 0,
                lexical_bytes: 0,
            },
            remaining: limits.max_points,
        };
        for declaration in &source.declarations {
            result.fills.step()?;
            result.fills.values(1)?;
            if result
                .declarations
                .insert(declaration.source_ordinal, declaration)
                .is_some()
            {
                return Err(invalid("duplicate chart paint declaration").into());
            }
            for color in &declaration.colors {
                result.fills.step()?;
                result.fills.values(1)?;
                if result.colors.insert(color.source_ordinal, color).is_some() {
                    return Err(invalid("duplicate chart color declaration").into());
                }
            }
        }
        for plot in &source.chart.plots {
            for definition in &plot.series {
                result.fills.step()?;
                result.fills.values(1)?;
                let mut points = BTreeMap::new();
                for point in &definition.point_overrides {
                    result.fills.step()?;
                    result.fills.values(1)?;
                    if points.insert(point.index, point).is_some() {
                        return Err(invalid("duplicate chart point style").into());
                    }
                }
                if result
                    .series
                    .insert(
                        (plot.source_ordinal, definition.index),
                        Series { definition, points },
                    )
                    .is_some()
                {
                    return Err(invalid("duplicate chart series style").into());
                }
            }
        }
        cancelled(check)?;
        Ok(result)
    }
    pub fn color(&self, ordinal: u32) -> Option<&'a ChartPaintColor> {
        self.colors.get(&ordinal).copied()
    }
    fn declaration(
        &self,
        layout: &SourceChartLayout,
    ) -> Result<Option<&'a ChartPaintDeclaration>, ChartStyleError> {
        if let Some(ordinal) = layout.retained_attribute_ordinals.first() {
            return Err(unresolved(*ordinal, "unrecognized chart style attribute"));
        }
        if let Some(node) = layout.unrecognized_children.first() {
            return Err(unresolved(
                node.source_ordinal,
                "unrecognized chart style content",
            ));
        }
        if let Some(property) = layout.properties.iter().find(|p| {
            !matches!(
                p.kind,
                ChartPropertyKind::Explosion | ChartPropertyKind::Bubble3D
            )
        }) {
            return Err(unresolved(
                property.source_ordinal,
                "chart point modifier requires semantic resolution",
            ));
        }
        // Geometry owns explosion/3D admission; data labels are a separate plot
        // component. Markers, picture options and extensions cannot disappear.
        if let Some(node) = layout.markup.iter().find(|m| {
            !matches!(
                m.kind,
                ChartMarkupKind::ShapeProperties | ChartMarkupKind::DataLabels
            )
        }) {
            return Err(unresolved(
                node.source_ordinal,
                "chart point markup requires semantic resolution",
            ));
        }
        let mut nodes = layout
            .markup
            .iter()
            .filter(|m| m.kind == ChartMarkupKind::ShapeProperties);
        let Some(node) = nodes.next() else {
            return Ok(None);
        };
        if nodes.next().is_some() {
            return Err(invalid("multiple shape property bindings in chart style").into());
        }
        self.declarations
            .get(&node.source_ordinal)
            .copied()
            .map(Some)
            .ok_or_else(|| invalid("chart shape property declaration missing").into())
    }
    pub fn resolve(
        &mut self,
        plot: u32,
        series: u32,
        point: u32,
    ) -> Result<ChartPointStyle<'a>, ChartStyleError> {
        self.fills.step()?;
        self.remaining = self
            .remaining
            .checked_sub(1)
            .ok_or(PptxError::Limit("chart styled points"))?;
        let bound = self
            .series
            .get(&(plot, series))
            .ok_or_else(|| invalid("chart style series binding missing"))?;
        let local = bound
            .points
            .get(&point)
            .map(|p| self.declaration(&p.layout))
            .transpose()?
            .flatten();
        let inherited = self.declaration(&bound.definition.layout)?;
        let location = bound.definition.source_ordinal;
        let mut fill = FillPartial::default();
        let mut line_fill = FillPartial::default();
        let mut geometry = GeometryPartial::default();
        let mut effects = None;
        let mut width_declared = false;
        for declaration in [local, inherited].into_iter().flatten() {
            self.fills.step()?;
            if let Some(&ordinal) = declaration.retained_ordinals.first() {
                return Err(unresolved(ordinal, "retained chart shape properties"));
            }
            if declaration.black_white_mode.is_some() {
                return Err(unresolved(
                    declaration.source_ordinal,
                    "chart black-white mode",
                ));
            }
            let origin = |source_ordinal| FillOrigin::Chart {
                part: self.source.chart.part.clone(),
                source_ordinal,
            };
            let apply = |result: Result<(), resolve::Failure>| match result {
                Ok(()) => Ok(()),
                Err(resolve::Failure::Abort(e)) => Err(ChartStyleError::Source(e)),
                Err(resolve::Failure::Unresolved(_)) => Err(unresolved(
                    declaration.source_ordinal,
                    "retained or unsupported fill inheritance",
                )),
            };
            if let Some(input) = &declaration.fill {
                apply(fill.merge(
                    FillInput::from(&input.definition),
                    &input.retained_ordinals,
                    &origin(input.source_ordinal),
                    None,
                    &mut self.fills,
                ))?;
            }
            if let Some(line) = &declaration.line {
                // Charge all variable provenance clones before the shared merger.
                self.lines.bytes(
                    self.source
                        .chart
                        .part
                        .len()
                        .checked_mul(64)
                        .ok_or(PptxError::Limit("chart line origin bytes"))?,
                )?;
                self.lines.line(line)?;
                geometry
                    .merge(
                        line,
                        &LineOrigin::Chart {
                            part: self.source.chart.part.clone(),
                            source_ordinal: line.source_ordinal,
                        },
                    )
                    .map_err(|_| unresolved(line.source_ordinal, "retained line geometry"))?;
                width_declared |= line.width.is_some();
                if let Some(input) = &line.fill {
                    let (ordinal, input) = resolve::line_input(input);
                    apply(line_fill.merge(
                        input,
                        &line.retained_ordinals,
                        &origin(ordinal),
                        None,
                        &mut self.fills,
                    ))?;
                }
            }
            if effects.is_none() {
                effects = declaration.effects.as_ref();
            }
        }
        // No hidden chart palette or 9525-EMU shape-width assumption. Profile
        // defaults only complete basic stroke attributes after a declared width.
        if !fill.complete() || !line_fill.complete() {
            return Err(unresolved(
                location,
                "automatic or incomplete chart fill requires style resolution",
            ));
        }
        let finish =
            |partial: FillPartial, budget: &mut FillBudget<'_>| match partial.finish(budget) {
                Ok(v) => Ok(v),
                Err(resolve::Failure::Abort(e)) => Err(ChartStyleError::Source(e)),
                Err(resolve::Failure::Unresolved(_)) => {
                    Err(unresolved(location, "unsupported chart fill inheritance"))
                }
            };
        let fill = finish(fill, &mut self.fills)?;
        let line_fill = finish(line_fill, &mut self.fills)?;
        if !matches!(line_fill, EffectiveFill::None { .. }) && !width_declared {
            return Err(unresolved(
                location,
                "automatic chart line width requires style resolution",
            ));
        }
        cancelled(self.fills.check)?;
        Ok(ChartPointStyle {
            fill,
            line_fill,
            line_geometry: geometry.finish(),
            effects,
        })
    }
}
