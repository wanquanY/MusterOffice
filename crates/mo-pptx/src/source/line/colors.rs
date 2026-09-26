//! Join immutable line inheritance to shared native color computation.
mod types;
use super::resolve::*;
use crate::{
    PptxError, cancelled,
    source::{
        SourceIndex,
        color::{self, ExpressionRef},
    },
};
pub use types::*;

fn expression(term: &LineColorTerm) -> ExpressionRef<'_> {
    ExpressionRef {
        value: &term.value,
        transforms: &term.transforms,
    }
}

/// Uses one inspected source, one inheritance budget and one color session.
/// No style-reference boundary quantizes a working color into a sample.
pub fn query(
    index: &SourceIndex,
    request: &SourceLineColorQuery,
    limits: LineColorLimits,
    check: &dyn Fn() -> bool,
) -> Result<SourceLineColors, PptxError> {
    query_in_context(index, request, &request.surface, limits, check)
}

/// Resolve declarations on request.surface using the explicitly selected drawing
/// surface's theme, color map and background. Original owners are never rewritten.
/// Page composition validates that these surfaces belong to one source hierarchy.
pub fn query_in_context(
    index: &SourceIndex,
    request: &SourceLineColorQuery,
    drawing_surface: &str,
    limits: LineColorLimits,
    check: &dyn Fn() -> bool,
) -> Result<SourceLineColors, PptxError> {
    cancelled(check)?;
    if request.objects.len() > limits.colors.max_queries {
        return Err(PptxError::Limit("line color queries"));
    }
    let styles = super::resolve::query_in_context(
        index,
        &SourceLineQuery {
            expected_source_sha256: request.expected_source_sha256.clone(),
            surface: request.surface.clone(),
            objects: request.objects.clone(),
            profile: request.line_profile,
        },
        drawing_surface,
        limits.lines,
        check,
    )?;
    let surface = &index.surfaces[drawing_surface]; // validated by the style query
    let mut session = color::Session::new(
        index,
        surface,
        request.color_profile,
        &request.context,
        limits.colors,
        check,
    )?;
    let mut objects = Vec::with_capacity(styles.objects.len());
    for result in styles.objects {
        cancelled(check)?;
        let paint = match &result.outcome {
            LineOutcome::Unresolved { .. } => LinePaintColor::UnresolvedStyle {},
            LineOutcome::Resolved { line } => match &line.fill {
                EffectiveLineFill::None { .. } => LinePaintColor::None {},
                EffectiveLineFill::Solid { color, .. } => {
                    let evaluation = session.expression(
                        expression(&color.color),
                        color.placeholder.as_ref().map(expression),
                    )?;
                    let outcome = LineColorSample::from_computed(evaluation.color);
                    LinePaintColor::Solid {
                        outcome,
                        dependencies: evaluation.dependencies,
                        notices: evaluation.notices,
                    }
                }
            },
        };
        objects.push(SourceLineColorResult {
            native_id: result.native_id,
            style: result.outcome,
            paint,
        });
    }
    Ok(SourceLineColors {
        source_sha256: styles.source_sha256,
        surface: styles.surface,
        line_profile: request.line_profile,
        color_profile: request.color_profile,
        color_mapping: surface.resolved_color_mapping.clone(),
        color_scheme: surface.theme_selection.colors.clone(),
        objects,
    })
}
