//! Fill expressions to working colors, with lazy native placeholder contexts.
mod context;
mod types;
use super::resolve::*;
use crate::{
    PptxError, cancelled,
    source::{
        SourceIndex,
        color::{self, ColorSample, ExpressionRef, Placeholder},
    },
};
pub use types::*;

struct Evaluator<'a> {
    index: &'a SourceIndex,
    contexts: context::Contexts<'a>,
    session: color::Session<'a>,
    remaining: usize,
}
impl Evaluator<'_> {
    fn expression(
        &mut self,
        expression: &FillColorExpression,
    ) -> Result<FillColorEvaluation, PptxError> {
        self.remaining = self
            .remaining
            .checked_sub(1)
            .ok_or(PptxError::Limit("fill color slots"))?;
        let mut binding = None;
        let contexts = &mut self.contexts;
        let index = self.index;
        let mut lookup = |budget: &mut color::Budget<'_>| {
            let Some(owner) = &expression.context_owner else {
                return Ok(None);
            };
            let Some(reference) = contexts.get(index, owner, budget)? else {
                return Ok(None);
            };
            binding = Some(Box::new(FillPlaceholderBinding {
                owner: owner.clone(),
                source_part: reference.part.map(str::to_owned),
                reference_ordinal: reference.ordinal,
                color_ordinal: reference.color.map(|c| c.source_ordinal),
            }));
            if let Some(source_ordinal) = reference.retained {
                return Err(color::Failure::Unresolved(
                    color::ColorUnresolved::RetainedPlaceholderContext {
                        part: reference.part.unwrap_or(&owner.part).into(),
                        source_ordinal,
                    },
                ));
            }
            Ok(reference.color.map(|c| ExpressionRef {
                value: &c.value,
                transforms: &c.transforms,
            }))
        };
        let evaluation = self.session.expression_with_placeholder(
            ExpressionRef {
                value: &expression.color.value,
                transforms: &expression.color.transforms,
            },
            Placeholder::Deferred(&mut lookup),
        )?;
        Ok(FillColorEvaluation {
            outcome: ColorSample::from_computed(evaluation.color),
            placeholder: binding,
            dependencies: evaluation.dependencies,
            notices: evaluation.notices,
        })
    }
    fn paint(&mut self, style: &FillOutcome) -> Result<FillPaintColors, PptxError> {
        Ok(match style {
            FillOutcome::Unresolved { .. } => FillPaintColors::UnresolvedStyle {},
            FillOutcome::Resolved { fill, .. } => match fill.as_ref() {
                EffectiveFill::None { .. } => FillPaintColors::None {},
                EffectiveFill::Image { .. } => FillPaintColors::ImageResourcesRequired {},
                EffectiveFill::Solid { color, .. } => FillPaintColors::Solid {
                    color: self.expression(color)?,
                },
                EffectiveFill::Gradient { gradient, .. } => {
                    if gradient.stops.value.len() > self.remaining {
                        return Err(PptxError::Limit("fill color slots"));
                    }
                    let mut stops = Vec::with_capacity(gradient.stops.value.len());
                    for stop in &gradient.stops.value {
                        stops.push(self.expression(&stop.color)?);
                    }
                    FillPaintColors::Gradient { stops }
                }
                EffectiveFill::Pattern { pattern, .. } => FillPaintColors::Pattern {
                    foreground: self.expression(&pattern.foreground)?,
                    background: self.expression(&pattern.background)?,
                },
            },
        })
    }
}

/// One immutable source, one fill inheritance batch and one shared color budget.
/// This evaluates color slots, not spatial interpolation, images or effects.
pub fn query(
    index: &SourceIndex,
    request: &SourceFillColorQuery,
    limits: FillColorLimits,
    check: &dyn Fn() -> bool,
) -> Result<SourceFillColors, PptxError> {
    query_in_context(index, request, &request.surface, limits, check)
}

/// Resolve declarations on request.surface using the explicitly selected drawing
/// surface's theme, color map and background. Original owners are never rewritten.
/// Page composition validates that these surfaces belong to one source hierarchy.
pub fn query_in_context(
    index: &SourceIndex,
    request: &SourceFillColorQuery,
    drawing_surface: &str,
    limits: FillColorLimits,
    check: &dyn Fn() -> bool,
) -> Result<SourceFillColors, PptxError> {
    query_on_page(
        index,
        request,
        drawing_surface,
        drawing_surface,
        limits,
        check,
    )
}

/// Uses physical drawing styles for ordinary paint and consuming-page styles
/// after a native background-fill redirect. Color work remains one session.
pub fn query_on_page(
    index: &SourceIndex,
    request: &SourceFillColorQuery,
    drawing_surface: &str,
    background_surface: &str,
    limits: FillColorLimits,
    check: &dyn Fn() -> bool,
) -> Result<SourceFillColors, PptxError> {
    cancelled(check)?;
    let styles = super::resolve::query_on_page(
        index,
        &SourceFillQuery {
            expected_source_sha256: request.expected_source_sha256.clone(),
            surface: request.surface.clone(),
            targets: request.targets.clone(),
            profile: request.fill_profile,
        },
        drawing_surface,
        background_surface,
        limits.fills,
        check,
    )?;
    colorize(
        index,
        styles,
        request,
        drawing_surface,
        background_surface,
        limits,
        check,
    )
}

/// Paint and text consumers share a source binding session without copying grids.
pub fn query_in_preparation(
    preparation: &mut crate::source::prepared::SourcePreparation<'_>,
    request: &SourceFillColorQuery,
    drawing_surface: &str,
    background_surface: &str,
    limits: FillColorLimits,
    check: &dyn Fn() -> bool,
) -> Result<SourceFillColors, PptxError> {
    let styles = super::resolve::query_in_preparation(
        preparation,
        &SourceFillQuery {
            expected_source_sha256: request.expected_source_sha256.clone(),
            surface: request.surface.clone(),
            targets: request.targets.clone(),
            profile: request.fill_profile,
        },
        drawing_surface,
        background_surface,
        limits.fills,
        check,
    )?;
    colorize(
        preparation.index(),
        styles,
        request,
        drawing_surface,
        background_surface,
        limits,
        check,
    )
}

pub(in crate::source) fn colorize(
    index: &SourceIndex,
    styles: SourceFillStyles,
    request: &SourceFillColorQuery,
    drawing_surface: &str,
    background_surface: &str,
    limits: FillColorLimits,
    check: &dyn Fn() -> bool,
) -> Result<SourceFillColors, PptxError> {
    let surface = &index.surfaces[drawing_surface];
    let mut evaluator = Evaluator {
        index,
        contexts: context::Contexts::default(),
        session: color::Session::new(
            index,
            surface,
            request.color_profile,
            &request.context,
            limits.colors,
            check,
        )?,
        remaining: limits.colors.max_queries,
    };
    let mut targets = Vec::with_capacity(styles.targets.len());
    for result in styles.targets {
        cancelled(check)?;
        let mut context_override = None;
        if drawing_surface != background_surface {
            let is_background = matches!(result.target, FillTarget::Background {})
                || matches!(&result.outcome, FillOutcome::Resolved { redirects, .. } if redirects.iter().any(|r| matches!(r.target.target, FillTarget::Background {})));
            let selected = if is_background {
                background_surface
            } else {
                drawing_surface
            };
            evaluator
                .session
                .select_surface(index, &index.surfaces[selected])?;
            if is_background {
                let selected_surface = &index.surfaces[selected];
                context_override = Some(FillColorSurface {
                    surface: selected.into(),
                    color_mapping: selected_surface.resolved_color_mapping.clone(),
                    color_scheme: selected_surface.theme_selection.colors.clone(),
                });
            }
        }
        let colors = evaluator.paint(&result.outcome)?;
        targets.push(SourceFillColorResult {
            target: result.target,
            style: result.outcome,
            colors,
            context_override,
        });
    }
    Ok(SourceFillColors {
        source_sha256: styles.source_sha256,
        surface: styles.surface,
        fill_profile: request.fill_profile,
        color_profile: request.color_profile,
        color_mapping: surface.resolved_color_mapping.clone(),
        color_scheme: surface.theme_selection.colors.clone(),
        targets,
    })
}
